#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${PROJECT_DIR}/dist"
APP_PATH="${DIST_DIR}/Better Spotlight.app"
DMG_PATH="${DIST_DIR}/Better-Spotlight.dmg"
NOTARIZATION_ZIP="${DIST_DIR}/Better-Spotlight-notarization.zip"

: "${CODESIGN_IDENTITY:?Set CODESIGN_IDENTITY to a Developer ID Application identity.}"
: "${NOTARY_PROFILE:?Set NOTARY_PROFILE to a notarytool Keychain profile.}"

APP_VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${PROJECT_DIR}/Cargo.toml" | head -n 1)"
EXPECTED_TAG="v${APP_VERSION}"
RELEASE_TAG="${RELEASE_TAG:-${GITHUB_REF_NAME:-}}"

if [[ -n "${RELEASE_TAG}" && "${RELEASE_TAG}" != "${EXPECTED_TAG}" ]]; then
    echo "Release tag ${RELEASE_TAG} does not match Cargo version ${EXPECTED_TAG}." >&2
    exit 1
fi

if ! xcrun notarytool history --keychain-profile "${NOTARY_PROFILE}" >/dev/null; then
    echo "Notary profile ${NOTARY_PROFILE} is missing or invalid." >&2
    exit 1
fi

BUILD_UNIVERSAL=1 "${PROJECT_DIR}/scripts/package.sh"

rm -f -- "${NOTARIZATION_ZIP}" "${DMG_PATH}"
ditto -c -k --keepParent "${APP_PATH}" "${NOTARIZATION_ZIP}"
xcrun notarytool submit "${NOTARIZATION_ZIP}" --keychain-profile "${NOTARY_PROFILE}" --wait
xcrun stapler staple "${APP_PATH}"
xcrun stapler validate "${APP_PATH}"

"${PROJECT_DIR}/scripts/create-dmg.sh"
xcrun notarytool submit "${DMG_PATH}" --keychain-profile "${NOTARY_PROFILE}" --wait
xcrun stapler staple "${DMG_PATH}"
xcrun stapler validate "${DMG_PATH}"

codesign --verify --deep --strict "${APP_PATH}"
spctl --assess --type execute --verbose=2 "${APP_PATH}"
spctl --assess --type open --context context:primary-signature --verbose=2 "${DMG_PATH}"

rm -f -- "${NOTARIZATION_ZIP}"
echo "Created notarized release ${DMG_PATH}"
