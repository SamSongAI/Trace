#!/usr/bin/env python3
"""Generate a multi-resolution Windows .ico from the source 32x32 PNG.

Input:  clients/trace-windows/assets/trace-32.png (RGBA, 32x32)
Output: clients/trace-windows/installer/assets/trace.ico
Sizes:  16, 24, 32, 48, 64, 128, 256 — covers taskbar, Start menu,
        tile, jumbo shell thumbnail across DPI scaling factors.

Implementation note: Pillow's ICO writer takes a single base image and
downsamples it internally via the `sizes=` parameter; `append_images=`
is silently ignored for this format. We therefore upscale the 32×32
source to 256×256 with Lanczos once, then let Pillow produce the seven
frames from that single base. The 32→256 upscale is lossy, but the
source asset is already 32×32 by product design (Phase 0–11), and
shell thumbnails rarely exceed 64×64 in practice.
"""
from pathlib import Path
from PIL import Image

REPO_ROOT = Path(__file__).resolve().parents[2]  # .../installer -> trace-windows
SRC = REPO_ROOT / "assets" / "trace-32.png"
OUT = REPO_ROOT / "installer" / "assets" / "trace.ico"

SIZES = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]


def main() -> None:
    base = Image.open(SRC).convert("RGBA")
    if base.size != (32, 32):
        raise SystemExit(f"expected 32x32 source, got {base.size}")
    # Upscale once to the largest target so Pillow's internal downsampler
    # always shrinks (higher fidelity than repeated upscales).
    base_256 = base.resize((256, 256), Image.LANCZOS)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    base_256.save(OUT, format="ICO", sizes=SIZES)
    print(f"wrote {OUT} with {len(SIZES)} frames: {[s[0] for s in SIZES]}")


if __name__ == "__main__":
    main()
