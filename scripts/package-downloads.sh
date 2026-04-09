#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DMG="${ROOT_DIR}/dist/Trace.dmg"
DOWNLOAD_DIR="${ROOT_DIR}/website/downloads"
MAC_ARCHIVE="${DOWNLOAD_DIR}/Trace.dmg"

if [[ ! -f "${DIST_DMG}" ]]; then
  echo "[package-downloads] Missing ${DIST_DMG}. Run ./scripts/trace.sh build-dmg first."
  exit 1
fi

mkdir -p "${DOWNLOAD_DIR}"
rm -f "${MAC_ARCHIVE}"

/bin/cp "${DIST_DMG}" "${MAC_ARCHIVE}"

if command -v shasum >/dev/null 2>&1; then
  echo "[package-downloads] macOS archive ready: ${MAC_ARCHIVE}"
  echo "[package-downloads] SHA256:"
  if ! LC_ALL=C LANG=C shasum -a 256 "${MAC_ARCHIVE}"; then
    if command -v openssl >/dev/null 2>&1; then
      openssl dgst -sha256 "${MAC_ARCHIVE}"
    else
      echo "[package-downloads] SHA256 unavailable: shasum/openssl not usable in current locale."
    fi
  fi
fi
