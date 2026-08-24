# Releasing Better Spotlight

Releases are built and notarized locally so the Developer ID private key and
Apple app-specific password never leave the release Mac. The publish script
attaches only the final notarized DMG to a GitHub Release.

## One-time local setup

Install both Rust macOS targets:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

Store Apple notarization credentials in Keychain. Use an app-specific password,
not your Apple Account password:

```sh
xcrun notarytool store-credentials "better-spotlight" \
  --apple-id "YOUR_APPLE_ID" \
  --team-id "PZTKZ6MYVZ" \
  --password "YOUR_APP_SPECIFIC_PASSWORD"
```

Create and validate a release locally:

```sh
CODESIGN_IDENTITY="Developer ID Application: YAGANA DIGITAL PRIVATE LIMITED (PZTKZ6MYVZ)" \
NOTARY_PROFILE="better-spotlight" \
./scripts/release.sh
```

The final file is `dist/Better-Spotlight.dmg`.

## Publish a release

Update the version in `Cargo.toml`, commit and push it to `main`, then run:

```sh
CODESIGN_IDENTITY="Developer ID Application: YAGANA DIGITAL PRIVATE LIMITED (PZTKZ6MYVZ)" \
NOTARY_PROFILE="better-spotlight" \
./scripts/publish-release.sh
```

The script requires a clean `main` branch that exactly matches `origin/main`.
It builds and notarizes the release, creates the matching version tag, pushes
the tag, and publishes the DMG on GitHub Releases. Existing matching tags can be
reused after a failed upload, but tags pointing to another commit are rejected.
