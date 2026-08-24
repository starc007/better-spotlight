#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${PROJECT_DIR}/dist"
APP_DIR="${DIST_DIR}/Better Spotlight.app"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"
ARCHIVE_PATH="${DIST_DIR}/Better-Spotlight.zip"
BUILD_UNIVERSAL="${BUILD_UNIVERSAL:-0}"

APP_VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${PROJECT_DIR}/Cargo.toml" | head -n 1)"
if [[ -z "${APP_VERSION}" ]]; then
    echo "Could not read the package version from Cargo.toml." >&2
    exit 1
fi

if [[ "${BUILD_UNIVERSAL}" == "1" ]]; then
    ARM_TARGET="aarch64-apple-darwin"
    INTEL_TARGET="x86_64-apple-darwin"
    for target in "${ARM_TARGET}" "${INTEL_TARGET}"; do
        if ! rustup target list --installed | grep -qx "${target}"; then
            echo "Missing Rust target ${target}. Install it with: rustup target add ${target}" >&2
            exit 1
        fi
        cargo build --release --target "${target}" --manifest-path "${PROJECT_DIR}/Cargo.toml"
    done
else
    cargo build --release --manifest-path "${PROJECT_DIR}/Cargo.toml"
fi

if [[ "${APP_DIR}" != "${PROJECT_DIR}/dist/Better Spotlight.app" ]]; then
    echo "Refusing to package to an unexpected path: ${APP_DIR}" >&2
    exit 1
fi

rm -rf -- "${APP_DIR}"
mkdir -p "${MACOS_DIR}" "${RESOURCES_DIR}"
if [[ "${BUILD_UNIVERSAL}" == "1" ]]; then
    lipo -create \
        "${PROJECT_DIR}/target/${ARM_TARGET}/release/better-spotlight" \
        "${PROJECT_DIR}/target/${INTEL_TARGET}/release/better-spotlight" \
        -output "${MACOS_DIR}/better-spotlight"
else
    cp "${PROJECT_DIR}/target/release/better-spotlight" "${MACOS_DIR}/better-spotlight"
fi
cp "${PROJECT_DIR}/packaging/Info.plist" "${CONTENTS_DIR}/Info.plist"
cp "${PROJECT_DIR}/packaging/AppIcon.icns" "${RESOURCES_DIR}/AppIcon.icns"
plutil -replace CFBundleShortVersionString -string "${APP_VERSION}" "${CONTENTS_DIR}/Info.plist"
plutil -replace CFBundleVersion -string "${APP_VERSION}" "${CONTENTS_DIR}/Info.plist"

SIGNING_IDENTITY="${CODESIGN_IDENTITY:--}"
SIGN_ARGS=(--force --options runtime --sign "${SIGNING_IDENTITY}")
if [[ "${SIGNING_IDENTITY}" != "-" ]]; then
    SIGN_ARGS+=(--timestamp)
fi
codesign "${SIGN_ARGS[@]}" "${APP_DIR}"

rm -f -- "${ARCHIVE_PATH}"
ditto -c -k --keepParent "${APP_DIR}" "${ARCHIVE_PATH}"
codesign --verify --deep --strict "${APP_DIR}"

if [[ "${BUILD_UNIVERSAL}" == "1" ]]; then
    lipo "${MACOS_DIR}/better-spotlight" -verify_arch arm64 x86_64
fi

echo "Created ${APP_DIR}"
echo "Created ${ARCHIVE_PATH}"
