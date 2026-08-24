#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "${PROJECT_DIR}"

: "${CODESIGN_IDENTITY:?Set CODESIGN_IDENTITY to a Developer ID Application identity.}"
: "${NOTARY_PROFILE:?Set NOTARY_PROFILE to a notarytool Keychain profile.}"

APP_VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
RELEASE_TAG="v${APP_VERSION}"

if [[ "$(git branch --show-current)" != "main" ]]; then
    echo "Releases must be published from main." >&2
    exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
    echo "Commit or remove all working tree changes before publishing." >&2
    exit 1
fi

git fetch origin main --tags
if [[ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]]; then
    echo "Local main must exactly match origin/main before publishing." >&2
    exit 1
fi

if gh release view "${RELEASE_TAG}" >/dev/null 2>&1; then
    echo "GitHub Release ${RELEASE_TAG} already exists." >&2
    exit 1
fi

RELEASE_TAG="${RELEASE_TAG}" "${PROJECT_DIR}/scripts/release.sh"

if git show-ref --verify --quiet "refs/tags/${RELEASE_TAG}"; then
    if [[ "$(git rev-list -n 1 "${RELEASE_TAG}")" != "$(git rev-parse HEAD)" ]]; then
        echo "Existing tag ${RELEASE_TAG} does not point to HEAD." >&2
        exit 1
    fi
else
    git tag -a "${RELEASE_TAG}" -m "Better Spotlight ${APP_VERSION}"
fi

if ! git ls-remote --exit-code --tags origin "refs/tags/${RELEASE_TAG}" >/dev/null 2>&1; then
    git push origin "${RELEASE_TAG}"
fi

gh release create "${RELEASE_TAG}" \
    "${PROJECT_DIR}/dist/Better-Spotlight.dmg" \
    --title "Better Spotlight ${APP_VERSION}" \
    --generate-notes \
    --verify-tag

echo "Published GitHub Release ${RELEASE_TAG}"
