#!/usr/bin/env bash

set -euo pipefail

APP_NAME="Trace"
EXECUTABLE_NAME="Trace"
BUNDLE_NAME="${APP_NAME}.app"
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
BUNDLE_DIR="${DIST_DIR}/${BUNDLE_NAME}"
DMG_NAME="${APP_NAME}.dmg"
DMG_PATH="${DIST_DIR}/${DMG_NAME}"
DMG_RW_PATH="${DIST_DIR}/${APP_NAME}-temp.dmg"
DMG_STAGING_DIR="${DIST_DIR}/dmg-staging"
DMG_VOLUME_NAME="${APP_NAME}"
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
  <string>1.0.1</string>
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

prepare_dmg_staging() {
  require_command ditto

  rm -rf "${DMG_STAGING_DIR}"
  mkdir -p "${DMG_STAGING_DIR}"

  ditto "${BUNDLE_DIR}" "${DMG_STAGING_DIR}/${BUNDLE_NAME}"
  ln -s /Applications "${DMG_STAGING_DIR}/Applications"
}

mount_dmg_rw() {
  local attach_output
  attach_output="$(hdiutil attach -readwrite -noverify -noautoopen "${DMG_RW_PATH}")"

  local device
  device="$(printf '%s\n' "${attach_output}" | awk '/Apple_HFS/ {print $1; exit}')"
  local mount_point
  mount_point="$(printf '%s\n' "${attach_output}" | awk -F '\t' '/Apple_HFS/ {print $NF; exit}')"

  [[ -n "${device}" ]] || fail "Unable to determine mounted device for ${DMG_RW_PATH}"
  [[ -n "${mount_point}" ]] || fail "Unable to determine mount point for ${DMG_RW_PATH}"

  printf '%s\t%s\n' "${device}" "${mount_point}"
}

detach_dmg() {
  local device="$1"
  local attempt

  for attempt in 1 2 3; do
    if hdiutil detach "${device}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  hdiutil detach -force "${device}" >/dev/null 2>&1 || fail "Unable to detach ${device}"
}

customize_dmg_window() {
  local mount_point="$1"
  local volume_name
  volume_name="$(basename "${mount_point}")"

  if ! command -v osascript >/dev/null 2>&1; then
    log "osascript unavailable; skipping Finder layout customization."
    return 0
  fi

  if command -v SetFile >/dev/null 2>&1; then
    SetFile -a C "${mount_point}" 2>/dev/null || true
  fi

  osascript <<EOF
tell application "Finder"
  tell disk "${volume_name}"
    open
    delay 1
    set containerWindow to container window
    set current view of containerWindow to icon view
    set toolbar visible of containerWindow to false
    set statusbar visible of containerWindow to false
    set bounds of containerWindow to {120, 120, 660, 420}
    set viewOptions to the icon view options of containerWindow
    set arrangement of viewOptions to not arranged
    set icon size of viewOptions to 144
    set text size of viewOptions to 16
    set position of item "${BUNDLE_NAME}" of containerWindow to {170, 170}
    set position of item "Applications" of containerWindow to {410, 170}
    close
    open
    update without registering applications
    delay 1
  end tell
end tell
EOF

  if command -v bless >/dev/null 2>&1; then
    bless --folder "${mount_point}" --openfolder "${mount_point}" >/dev/null 2>&1 || true
  fi
}

build_dmg() {
  require_command hdiutil

  [[ -d "${BUNDLE_DIR}" ]] || build_app_bundle

  prepare_dmg_staging
  rm -f "${DMG_RW_PATH}" "${DMG_PATH}"

  log "Creating writable disk image..."
  hdiutil create \
    -volname "${DMG_VOLUME_NAME}" \
    -srcfolder "${DMG_STAGING_DIR}" \
    -fs HFS+ \
    -format UDRW \
    -ov \
    "${DMG_RW_PATH}" >/dev/null

  local mount_info device mount_point
  mount_info="$(mount_dmg_rw)"
  device="${mount_info%%$'\t'*}"
  mount_point="${mount_info#*$'\t'}"

  customize_dmg_window "${mount_point}"
  sync
  detach_dmg "${device}"

  log "Compressing final disk image..."
  hdiutil convert "${DMG_RW_PATH}" \
    -format UDZO \
    -imagekey zlib-level=9 \
    -ov \
    -o "${DMG_PATH}" >/dev/null

  rm -f "${DMG_RW_PATH}"
  rm -rf "${DMG_STAGING_DIR}"
  log "Built disk image: ${DMG_PATH}"
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
  ./scripts/trace.sh build-dmg     Build dist/Trace.dmg with drag-to-Applications layout
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
  build-dmg)
    build_dmg
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
