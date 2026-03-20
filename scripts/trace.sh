#!/usr/bin/env bash

set -euo pipefail

APP_NAME="Trace"
EXECUTABLE_NAME="Trace"
BUNDLE_NAME="${APP_NAME}.app"
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
BUNDLE_DIR="${DIST_DIR}/${BUNDLE_NAME}"
EXECUTABLE_SOURCE="${ROOT_DIR}/.build/release/${EXECUTABLE_NAME}"
EXECUTABLE_DEST="${BUNDLE_DIR}/Contents/MacOS/${APP_NAME}"
RESOURCES_DIR="${BUNDLE_DIR}/Contents/Resources"
INFO_PLIST_PATH="${BUNDLE_DIR}/Contents/Info.plist"
ICONSET_DIR="${DIST_DIR}/Trace.iconset"
ICON_OUTPUT_PATH="${DIST_DIR}/Trace.icns"
ICON_BUNDLE_PATH="${RESOURCES_DIR}/Trace.icns"
APPLICATIONS_DIR="/Applications"
INSTALLED_APP_PATH="${APPLICATIONS_DIR}/${BUNDLE_NAME}"
DEFAULT_CODESIGN_IDENTITY="-"
ICON_SOURCE_CANDIDATES=(
  "${ROOT_DIR}/Sources/Trace/Resources/trace-app-icon.svg"
  "${ROOT_DIR}/logo.png"
)

log() {
  printf '[trace] %s\n' "$*"
}

fail() {
  printf '[trace] ERROR: %s\n' "$*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing command: $1"
}

generate_app_icon() {
  local icon_source_path=""
  for candidate in "${ICON_SOURCE_CANDIDATES[@]}"; do
    if [[ -f "${candidate}" ]]; then
      icon_source_path="${candidate}"
      break
    fi
  done

  [[ -n "${icon_source_path}" ]] || return 0

  require_command swift
  require_command iconutil

  rm -rf "${ICONSET_DIR}" "${ICON_OUTPUT_PATH}"
  mkdir -p "${ICONSET_DIR}"

  swift "${ROOT_DIR}/scripts/generate-app-icon.swift" "${icon_source_path}" "${ICONSET_DIR}"
  iconutil -c icns "${ICONSET_DIR}" -o "${ICON_OUTPUT_PATH}"
}

copy_swiftpm_resource_bundles() {
  require_command ditto

  local resource_bundles=()
  while IFS= read -r bundle_path; do
    resource_bundles+=("${bundle_path}")
  done < <(find "${ROOT_DIR}/.build" -type d -path "*/release/${EXECUTABLE_NAME}_*.bundle" | sort)

  if [[ "${#resource_bundles[@]}" -eq 0 ]]; then
    log "No SwiftPM resource bundles found for ${EXECUTABLE_NAME}."
    return 0
  fi

  for bundle_path in "${resource_bundles[@]}"; do
    local bundle_name
    bundle_name="$(basename "${bundle_path}")"
    rm -rf "${RESOURCES_DIR:?}/${bundle_name}"
    ditto "${bundle_path}" "${RESOURCES_DIR}/${bundle_name}"
    log "Copied resource bundle: ${bundle_name}"
  done
}

build_app_bundle() {
  require_command swift

  log "Building release executable..."
  (cd "${ROOT_DIR}" && swift build -c release)

  [[ -x "${EXECUTABLE_SOURCE}" ]] || fail "Build succeeded but executable not found at ${EXECUTABLE_SOURCE}"

  rm -rf "${BUNDLE_DIR}"
  mkdir -p "$(dirname "${EXECUTABLE_DEST}")"
  mkdir -p "${RESOURCES_DIR}"
  cp "${EXECUTABLE_SOURCE}" "${EXECUTABLE_DEST}"
  chmod +x "${EXECUTABLE_DEST}"

  generate_app_icon
  if [[ -f "${ICON_OUTPUT_PATH}" ]]; then
    cp "${ICON_OUTPUT_PATH}" "${ICON_BUNDLE_PATH}"
  fi
  copy_swiftpm_resource_bundles

  cat > "${INFO_PLIST_PATH}" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDisplayName</key>
  <string>Trace</string>
  <key>CFBundleExecutable</key>
  <string>Trace</string>
  <key>CFBundleIdentifier</key>
  <string>com.trace.app</string>
  <key>CFBundleIconFile</key>
  <string>Trace.icns</string>
  <key>CFBundleName</key>
  <string>Trace</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

  sign_bundle
  log "Built app bundle: ${BUNDLE_DIR}"
}

sign_bundle() {
  require_command codesign
  local identity="${CODESIGN_IDENTITY:-$DEFAULT_CODESIGN_IDENTITY}"
  log "Codesigning bundle (identity: ${identity})..."
  codesign --force --deep --sign "${identity}" "${BUNDLE_DIR}"
}

install_app_bundle() {
  [[ -d "${BUNDLE_DIR}" ]] || fail "App bundle not found. Run: ./scripts/trace.sh build-app"
  require_command ditto

  if [[ -w "${APPLICATIONS_DIR}" ]]; then
    rm -rf "${INSTALLED_APP_PATH}"
    ditto "${BUNDLE_DIR}" "${INSTALLED_APP_PATH}"
  else
    log "Administrator permission is required to write into /Applications."
    sudo rm -rf "${INSTALLED_APP_PATH}"
    sudo ditto "${BUNDLE_DIR}" "${INSTALLED_APP_PATH}"
  fi

  if command -v xattr >/dev/null 2>&1; then
    xattr -dr com.apple.quarantine "${INSTALLED_APP_PATH}" 2>/dev/null || true
  fi

  log "Installed: ${INSTALLED_APP_PATH}"
}

launch_app() {
  require_command open

  if [[ -d "${INSTALLED_APP_PATH}" ]]; then
    open "${INSTALLED_APP_PATH}"
    log "Launched from /Applications."
    return
  fi

  if [[ -d "${BUNDLE_DIR}" ]]; then
    open "${BUNDLE_DIR}"
    log "Launched from local dist bundle."
    return
  fi

  fail "No app found to launch. Run: ./scripts/trace.sh install"
}

usage() {
  cat <<'EOF'
Trace packaging helper

Usage:
  ./scripts/trace.sh build-app     Build dist/Trace.app
  ./scripts/trace.sh install-app   Install dist/Trace.app into /Applications
  ./scripts/trace.sh launch-app    Launch installed app (or local dist app)
  ./scripts/trace.sh install       Build + install
  ./scripts/trace.sh reinstall     Build + install + launch
  ./scripts/trace.sh check         swift build + swift test
EOF
}

check_project() {
  require_command swift
  (cd "${ROOT_DIR}" && swift build && swift test)
  log "Build/test check passed."
}

COMMAND="${1:-install}"

case "${COMMAND}" in
  build-app)
    build_app_bundle
    ;;
  install-app)
    install_app_bundle
    ;;
  launch-app)
    launch_app
    ;;
  install)
    build_app_bundle
    install_app_bundle
    ;;
  reinstall)
    build_app_bundle
    install_app_bundle
    launch_app
    ;;
  check)
    check_project
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    usage
    fail "Unknown command: ${COMMAND}"
    ;;
esac
