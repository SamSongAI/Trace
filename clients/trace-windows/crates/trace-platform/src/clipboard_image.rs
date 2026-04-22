//! Clipboard → PNG byte pipeline for Phase 13 image paste.
//!
//! This module sits **above** [`trace_core::ClipboardImageWriter::write_png`]
//! (Task 13.1 — pure disk-write layer) and **below** the UI layer (Task 13.3
//! — paste-shortcut handler). Its job is narrow:
//!
//! 1. Open the platform clipboard through [`arboard`].
//! 2. If the clipboard holds image data, grab its RGBA8 frame.
//! 3. Encode those pixels as PNG bytes using the [`png`] crate.
//! 4. Hand the bytes back to the caller, ready for `write_png`.
//!
//! The encoder lives in its own public function
//! ([`encode_rgba_as_png`]) so it can be unit-tested with fully
//! deterministic input. The clipboard read itself ([`read_clipboard_image_as_png`])
//! is environment-dependent — whatever the user happens to have copied —
//! so there are no unit tests for it; a manual integration smoke test
//! on the paste shortcut in Task 13.3 is the verification hook.
//!
//! ## Cross-platform
//!
//! Unlike most of this crate, `clipboard_image` carries **no**
//! `#[cfg(windows)]` gate on its public surface. [`arboard`] abstracts
//! the platform: on Windows it talks to the Win32 clipboard, on macOS
//! it talks to `NSPasteboard`. Keeping the module cross-platform lets
//! `cargo check --workspace` pass on any dev host, mirroring how
//! [`crate::vault_validation`] is built.

use thiserror::Error;

/// Errors surfaced by the clipboard-read + PNG-encode pipeline. Each
/// variant names the phase that failed so the UI layer (Task 13.3) can
/// decide whether to fall through to the native paste path, warn the
/// user, or silently log.
///
/// Built on `thiserror::Error` to stay in line with
/// [`trace_core::TraceError`] — the message strings on each `#[error(...)]`
/// line are the wire format consumed by the
/// `clipboard_image_error_display_includes_phase` test and by log scrapers
/// that triage on substring.
#[derive(Debug, Error)]
pub enum ClipboardImageError {
    /// `arboard::Clipboard::new()` failed. On Windows this typically
    /// means `OpenClipboard` returned an error (another process is
    /// holding the clipboard open, COM initialisation lost, etc.).
    /// On macOS it means the `NSPasteboard` handle could not be
    /// acquired — vanishingly rare outside of sandbox misconfigurations.
    #[error("clipboard unavailable: {0}")]
    ClipboardUnavailable(String),
    /// The clipboard opened cleanly but the `get_image()` call failed
    /// for a reason *other* than "no image on the clipboard". That
    /// specific case is handled by returning `Ok(None)` from
    /// [`read_clipboard_image_as_png`]; everything else — serialisation
    /// errors, allocator failures, unknown formats — lands here.
    #[error("clipboard read failed: {0}")]
    ClipboardReadFailed(String),
    /// RGBA → PNG encoding failed. Common triggers: width/height of
    /// zero, `rgba.len()` not equal to `width * height * 4`, dimension
    /// overflow in the size calculation, or a libpng-level error bubbling
    /// up from the [`png`] crate. The message is the original diagnostic
    /// verbatim where possible.
    #[error("png encoding failed: {0}")]
    PngEncodingFailed(String),
}

/// Reads the current clipboard and, if it holds image data, returns a
/// fully-formed PNG byte buffer. The output is ready for
/// [`trace_core::ClipboardImageWriter::write_png`] — no further framing
/// or header work is needed.
///
/// Return semantics:
///
/// * `Ok(None)` — either the clipboard opened successfully but does not
///   hold image data (user copied text, files, nothing at all, or an
///   image format `arboard` does not recognise), OR the clipboard was
///   momentarily held open by another process on Windows
///   (`arboard::Error::ClipboardOccupied`). The Win32 clipboard is a
///   globally serialised resource — password managers and clipboard
///   history tools legitimately lock it for short windows — so we treat
///   "occupied" as a transient "no image available right now" rather
///   than a real error. The caller in Task 13.3 should fall back to the
///   platform's native paste behaviour in both sub-cases.
/// * `Ok(Some(bytes))` — `bytes` is a complete PNG file (magic header
///   + IHDR + IDAT + IEND) containing the clipboard's pixels.
/// * `Err(_)` — a platform-layer failure. The UI should surface this
///   as a warning; silently falling through would be confusing.
pub fn read_clipboard_image_as_png() -> Result<Option<Vec<u8>>, ClipboardImageError> {
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(cb) => cb,
        // See the function-level docs: Win32 ClipboardOccupied is a
        // transient serialisation conflict with another process holding
        // the clipboard open, not a real failure. Map it to Ok(None) so
        // the UI falls through to its native / text path instead of
        // surfacing a spurious "剪贴板读取失败" toast.
        Err(arboard::Error::ClipboardOccupied) => return Ok(None),
        Err(e) => return Err(ClipboardImageError::ClipboardUnavailable(e.to_string())),
    };

    let image = match clipboard.get_image() {
        Ok(img) => img,
        // `ContentNotAvailable` is the "normal" miss — text / files /
        // empty clipboard. Map it to Ok(None) so the UI can fall through
        // to the native paste path without treating it as an error.
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        // `ClipboardOccupied` can also surface here per the `arboard`
        // crate docs — the handle from `new()` succeeded but another
        // process reacquired the lock before `get_image()` ran. Same
        // transient-no-op mapping as above.
        Err(arboard::Error::ClipboardOccupied) => return Ok(None),
        Err(e) => {
            return Err(ClipboardImageError::ClipboardReadFailed(e.to_string()));
        }
    };

    // `arboard::ImageData` is documented as tightly-packed RGBA8 with
    // `width` and `height` as `usize`. On 64-bit targets those can
    // exceed u32, so we reject oversized frames up front rather than
    // letting the png crate panic inside the encoder.
    let width = u32::try_from(image.width)
        .map_err(|_| ClipboardImageError::PngEncodingFailed("width exceeds u32".into()))?;
    let height = u32::try_from(image.height)
        .map_err(|_| ClipboardImageError::PngEncodingFailed("height exceeds u32".into()))?;

    let bytes = encode_rgba_as_png(width, height, &image.bytes)?;
    Ok(Some(bytes))
}

/// Encodes a tightly-packed RGBA8 pixel buffer as PNG bytes.
///
/// Exposed as a standalone public function — not folded into
/// [`read_clipboard_image_as_png`] — specifically to make unit testing
/// feasible. Clipboard reads are by definition non-deterministic
/// (depends on what the user copied), but the encoder itself is pure
/// and can be exercised with hand-crafted pixel buffers.
///
/// Validation performed (all before any allocation):
/// * `width > 0 && height > 0` — zero dimensions are rejected because
///   the PNG spec forbids them (`width` and `height` are declared as
///   four-byte unsigned integers with the range `[1, 2^31 - 1]`).
/// * `width * height * 4` does not overflow `u32`. Oversized clipboard
///   frames (a hypothetical screenshot larger than 4 GiB of RGBA) are
///   rejected here rather than allowed to wrap.
/// * `rgba.len() == width * height * 4` — the caller must hand over
///   exactly the pixel count declared by the dimensions.
///
/// Returns [`ClipboardImageError::PngEncodingFailed`] with a human-
/// readable message on any failure, including libpng-level errors
/// produced by [`png::Encoder`].
pub fn encode_rgba_as_png(
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<Vec<u8>, ClipboardImageError> {
    if width == 0 || height == 0 {
        return Err(ClipboardImageError::PngEncodingFailed(format!(
            "zero dimensions not allowed (w={width}, h={height})"
        )));
    }

    // `checked_mul` chain guards against the (unlikely but possible)
    // case of a clipboard frame so large that the RGBA byte count
    // overflows u32. Without this the subsequent length compare would
    // silently wrap and the encoder would be fed a mismatched buffer.
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| {
            ClipboardImageError::PngEncodingFailed("frame dimensions overflow".into())
        })?;

    if rgba.len() as u64 != u64::from(expected) {
        return Err(ClipboardImageError::PngEncodingFailed(format!(
            "buffer size mismatch: expected {expected} bytes \
             (w={width}, h={height}, 4 bytes/pixel), got {}",
            rgba.len()
        )));
    }

    let mut out: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|e| ClipboardImageError::PngEncodingFailed(e.to_string()))?;
        writer
            .write_image_data(rgba)
            .map_err(|e| ClipboardImageError::PngEncodingFailed(e.to_string()))?;
        // Writer is dropped here, flushing the IEND chunk into `out`.
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Canonical 8-byte PNG signature from the PNG spec
    /// (ISO/IEC 15948:2003 §5.2). Every valid PNG begins with these
    /// exact bytes; the test suite uses it as a byte-level smoke check
    /// that the encoder produced a real PNG, not some arbitrary blob.
    const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn encode_rgba_as_png_emits_valid_png_magic_header() {
        // 1×1 opaque red pixel — the smallest legal PNG we can build.
        let red = [0xFFu8, 0x00, 0x00, 0xFF];
        let bytes = encode_rgba_as_png(1, 1, &red).expect("encode 1x1 red");

        assert!(
            bytes.len() >= 8,
            "encoder must emit at least the 8-byte signature"
        );
        assert_eq!(
            &bytes[..8],
            &PNG_SIGNATURE,
            "first 8 bytes must be the PNG magic header"
        );
    }

    #[test]
    fn encode_rgba_as_png_round_trips_via_decoder() {
        // 2×3 image = 6 pixels × 4 bytes = 24 bytes. Every pixel is
        // distinct so a byte-mismatch anywhere in the pipeline shows
        // up as an assertion failure.
        let width = 2u32;
        let height = 3u32;
        #[rustfmt::skip]
        let rgba: [u8; 24] = [
            0x00, 0x11, 0x22, 0x33,  0x44, 0x55, 0x66, 0x77,
            0x88, 0x99, 0xAA, 0xBB,  0xCC, 0xDD, 0xEE, 0xFF,
            0x01, 0x23, 0x45, 0x67,  0x89, 0xAB, 0xCD, 0xEF,
        ];

        let encoded = encode_rgba_as_png(width, height, &rgba).expect("encode 2x3 distinct pixels");

        let decoder = png::Decoder::new(encoded.as_slice());
        let mut reader = decoder.read_info().expect("decode info");
        let info = reader.info();
        assert_eq!(info.width, width);
        assert_eq!(info.height, height);
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);

        let mut buf = vec![0u8; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf).expect("decode frame");
        let decoded = &buf[..frame.buffer_size()];

        assert_eq!(
            decoded,
            &rgba[..],
            "decoded pixels must equal the original RGBA buffer"
        );
    }

    #[test]
    fn encode_rgba_as_png_rejects_buffer_size_mismatch() {
        // 2×2 ⇒ 16 bytes expected. Hand over 15 and 17 in turn; both
        // must error with a "buffer size mismatch" diagnostic.
        let short = vec![0u8; 15];
        let long = vec![0u8; 17];

        for buffer in [short, long] {
            let err = encode_rgba_as_png(2, 2, &buffer).expect_err("mismatch must error");
            match err {
                ClipboardImageError::PngEncodingFailed(msg) => {
                    assert!(
                        msg.contains("buffer size mismatch"),
                        "message should mention the mismatch phase, got: {msg}"
                    );
                }
                other => panic!("expected PngEncodingFailed, got {other:?}"),
            }
        }
    }

    #[test]
    fn encode_rgba_as_png_rejects_zero_dimensions() {
        // Width = 0 with a 4-byte buffer (what the caller might naively
        // supply for a 1×1 image where they forgot to set the width).
        let err_w = encode_rgba_as_png(0, 1, &[0u8; 4]).expect_err("zero width must error");
        match err_w {
            ClipboardImageError::PngEncodingFailed(msg) => {
                assert!(
                    msg.contains("zero") || msg.contains("dimensions"),
                    "zero-width message should mention zero/dimensions, got: {msg}"
                );
            }
            other => panic!("expected PngEncodingFailed, got {other:?}"),
        }

        // Symmetric check for height = 0.
        let err_h = encode_rgba_as_png(1, 0, &[0u8; 4]).expect_err("zero height must error");
        match err_h {
            ClipboardImageError::PngEncodingFailed(msg) => {
                assert!(
                    msg.contains("zero") || msg.contains("dimensions"),
                    "zero-height message should mention zero/dimensions, got: {msg}"
                );
            }
            other => panic!("expected PngEncodingFailed, got {other:?}"),
        }
    }

    #[test]
    fn encode_rgba_as_png_rejects_dimension_overflow() {
        // Two independent cases, one per link in the `checked_mul` chain
        // at `encode_rgba_as_png`:
        //
        //   1. `width.checked_mul(height)` — tripped by u32::MAX × u32::MAX.
        //   2. `pixels.checked_mul(4)`    — tripped by (u32::MAX, 1): the
        //      first mul succeeds with u32::MAX, only the × 4 stage overflows.
        //
        // Covering both stops a future refactor that accidentally drops
        // one link (e.g. collapsing the chain into a single `as u64`
        // compare) from silently shipping.
        //
        // We pass empty slices because this test only exercises the
        // size calculation — we deliberately do NOT allocate multi-GiB
        // buffers just to satisfy the length check.
        let cases = [
            // (width, height, label)
            (u32::MAX, u32::MAX, "width*height overflow"),
            (u32::MAX, 1, "width*height*4 overflow (second link)"),
        ];

        for (width, height, label) in cases {
            let err =
                encode_rgba_as_png(width, height, &[]).expect_err(&format!("{label} must error"));
            match err {
                ClipboardImageError::PngEncodingFailed(msg) => {
                    assert!(
                        msg.contains("overflow"),
                        "{label}: message should mention overflow, got: {msg}"
                    );
                }
                other => panic!("{label}: expected PngEncodingFailed, got {other:?}"),
            }
        }
    }

    #[test]
    fn clipboard_image_error_display_includes_phase() {
        // Each variant's Display must mention its phase so `grep`-based
        // log triage can tell unavailable/read/encode apart at a glance.
        let unavailable = ClipboardImageError::ClipboardUnavailable("handle busy".into());
        let read = ClipboardImageError::ClipboardReadFailed("format error".into());
        let encode = ClipboardImageError::PngEncodingFailed("bad header".into());

        assert!(
            unavailable.to_string().contains("clipboard"),
            "unavailable display must say 'clipboard', got: {unavailable}"
        );
        assert!(
            read.to_string().contains("clipboard"),
            "read display must say 'clipboard', got: {read}"
        );
        assert!(
            encode.to_string().contains("png"),
            "encode display must say 'png', got: {encode}"
        );
    }

    #[test]
    fn clipboard_image_error_is_send_sync() {
        // Compile-time trait check: if a future variant accidentally
        // holds a non-thread-safe field (e.g. `Rc<...>`), this test
        // stops compiling and forces a conscious decision. Tokio-based
        // error plumbing in higher layers relies on Send + Sync.
        fn _assert<T: Send + Sync>() {}
        _assert::<ClipboardImageError>();
    }
}
