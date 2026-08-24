#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${PROJECT_DIR}/dist"
APP_PATH="${DIST_DIR}/Better Spotlight.app"
DMG_PATH="${DIST_DIR}/Better-Spotlight.dmg"

: "${CODESIGN_IDENTITY:?Set CODESIGN_IDENTITY to a Developer ID Application identity.}"

if [[ ! -d "${APP_PATH}" ]]; then
    echo "Missing ${APP_PATH}. Run scripts/package.sh first." >&2
    exit 1
fi

RELEASE_TMP_DIR="$(mktemp -d /tmp/better-spotlight-dmg.XXXXXX)"
cleanup() {
    rm -rf -- "${RELEASE_TMP_DIR}"
}
trap cleanup EXIT

DMG_ROOT="${RELEASE_TMP_DIR}/dmg"
mkdir "${DMG_ROOT}"
ditto "${APP_PATH}" "${DMG_ROOT}/Better Spotlight.app"
ln -s /Applications "${DMG_ROOT}/Applications"

rm -f -- "${DMG_PATH}"
hdiutil create \
    -volname "Better Spotlight" \
    -srcfolder "${DMG_ROOT}" \
    -format UDZO \
    -ov \
    "${DMG_PATH}"

codesign --force --timestamp --sign "${CODESIGN_IDENTITY}" "${DMG_PATH}"
codesign --verify --verbose=2 "${DMG_PATH}"

echo "Created signed disk image ${DMG_PATH}"
