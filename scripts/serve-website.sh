#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WEBSITE_DIR="${ROOT_DIR}/website"
PORT="${1:-8080}"

if [[ ! -d "${WEBSITE_DIR}" ]]; then
  echo "[serve-website] Missing website directory: ${WEBSITE_DIR}"
  exit 1
fi

cd "${WEBSITE_DIR}"
echo "[serve-website] Serving website at http://127.0.0.1:${PORT}"
python3 -m http.server "${PORT}"
