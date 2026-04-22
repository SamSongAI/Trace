#!/usr/bin/env python3
"""Convert the repository's MIT LICENSE to minimal RTF for WiX.

WiX UI templates (`WixUI_InstallDir` in particular) embed the EULA
page as an `rtf1`-flavoured RichText blob, referenced by the
`<WixVariable Id="WixUILicenseRtf" Value="..." />` override. The
content is rendered by a Windows Forms `RichTextBox`, so the header
can be intentionally minimal — just enough control words to declare
encoding, font, and default paragraph style.

We keep the LICENSE at the repo root as the single source of truth
and re-derive this RTF on every build (see Phase 14 plan). A license
amendment therefore only needs a one-line text edit + a script run.

Input:  <repo>/LICENSE (UTF-8, plain text)
Output: clients/trace-windows/installer/assets/LICENSE.rtf (ASCII)
"""
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]  # .../scripts -> installer -> trace-windows -> clients -> Trace
SRC = REPO_ROOT / "LICENSE"
OUT = (
    REPO_ROOT
    / "clients"
    / "trace-windows"
    / "installer"
    / "assets"
    / "LICENSE.rtf"
)


def to_rtf(text: str) -> str:
    """Wrap plain text in the minimum RTF header a RichTextBox accepts.

    The three reserved characters in RTF (`\\`, `{`, `}`) are escaped
    first, then newlines are converted to `\\par` paragraph markers so
    the RichTextBox preserves the original line structure (each clause
    of the MIT text on its own line).
    """
    # Escape RTF control characters. Order matters: backslash first,
    # otherwise the replacements that introduce `\{` / `\}` would be
    # double-escaped on the second pass.
    escaped = text.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}")
    # Convert newlines to \par so RichTextBox renders line breaks.
    paragraphs = escaped.splitlines()
    body = "\\par\n".join(paragraphs)
    # Header breakdown:
    #   \rtf1              - RTF version 1 (only version that exists)
    #   \ansi              - 8-bit ANSI character set
    #   \ansicpg1252       - Windows-1252 code page (MIT text is ASCII,
    #                        so this is effectively a no-op)
    #   \deff0             - default font index 0 in the \fonttbl below
    #   \nouicompat        - disable WordPad legacy compat quirks
    #   \deflang1033       - default language en-US (LCID 1033)
    #   \fonttbl{...}      - single entry: Segoe UI, Windows' default UI
    #   \viewkind4         - "normal" view mode (as saved by Word)
    #   \uc1               - fall back to 1 ANSI byte per Unicode escape
    #   \pard              - reset paragraph formatting to defaults
    #   \f0                - select font 0 from \fonttbl
    #   \fs18              - font size 9pt (\fsN is half-points)
    return (
        "{\\rtf1\\ansi\\ansicpg1252\\deff0\\nouicompat"
        "\\deflang1033{\\fonttbl{\\f0\\fnil Segoe UI;}}\n"
        "\\viewkind4\\uc1\\pard\\f0\\fs18\n"
        f"{body}\\par\n"
        "}\n"
    )


def main() -> None:
    if not SRC.exists():
        raise SystemExit(f"LICENSE not found at {SRC}")
    text = SRC.read_text(encoding="utf-8")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(to_rtf(text), encoding="ascii")
    print(f"wrote {OUT} ({OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
