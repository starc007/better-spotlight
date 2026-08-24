#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${PROJECT_DIR}/dist"
APP_DIR="${DIST_DIR}/Better Spotlight.app"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
ARCHIVE_PATH="${DIST_DIR}/Better-Spotlight.zip"

cargo build --release --manifest-path "${PROJECT_DIR}/Cargo.toml"

if [[ "${APP_DIR}" != "${PROJECT_DIR}/dist/Better Spotlight.app" ]]; then
    echo "Refusing to package to an unexpected path: ${APP_DIR}" >&2
    exit 1
fi

rm -rf -- "${APP_DIR}"
mkdir -p "${MACOS_DIR}"
cp "${PROJECT_DIR}/target/release/better-spotlight" "${MACOS_DIR}/better-spotlight"
cp "${PROJECT_DIR}/packaging/Info.plist" "${CONTENTS_DIR}/Info.plist"

SIGNING_IDENTITY="${CODESIGN_IDENTITY:--}"
codesign --force --options runtime --sign "${SIGNING_IDENTITY}" "${APP_DIR}"

rm -f -- "${ARCHIVE_PATH}"
ditto -c -k --keepParent "${APP_DIR}" "${ARCHIVE_PATH}"
codesign --verify --deep --strict "${APP_DIR}"

echo "Created ${APP_DIR}"
echo "Created ${ARCHIVE_PATH}"
