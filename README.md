# Better Spotlight

A fast, native Spotlight replacement for macOS, built with Rust and
[GPUI](https://gpui.rs)—the GPU-accelerated UI framework behind Zed.

[Download the latest release](https://github.com/starc007/better-spotlight/releases/latest)

## Features

- Instant fuzzy search over your applications
- Real app icons decoded from `.icns` bundles
- Keyboard-first: type to search, arrows to navigate, Enter to launch
- Mouse support with hover selection and double-click launch
- File and folder search backed by the macOS Spotlight index
- Finder-native file and folder icons with grouped results
- Calculator expressions with Enter-to-copy results
- Private, in-memory clipboard history with search and deletion controls
- Borderless floating panel, always on top
- Configurable global shortcut (⌘Space by default)

## Install

1. Download `Better-Spotlight.dmg` from the
   [latest GitHub Release](https://github.com/starc007/better-spotlight/releases/latest).
2. Open the DMG and drag **Better Spotlight** into **Applications**.
3. Open Better Spotlight once from Applications.
4. Go to **System Settings → Keyboard → Keyboard Shortcuts → Spotlight** and
   turn off **Show Spotlight search**.
5. Press **⌘Space** to open Better Spotlight.

Disabling the shortcut only replaces Apple's Spotlight window. macOS Spotlight
indexing remains enabled and powers Better Spotlight's file search.

To start Better Spotlight automatically, go to **System Settings → General →
Login Items & Extensions**, click **+** under **Open at Login**, and select
Better Spotlight.

## Local setup

Requirements:

- macOS 13 or later
- Stable Rust toolchain
- Xcode with the Metal Toolchain

Clone and run the packaged app locally:

```sh
git clone https://github.com/starc007/better-spotlight.git
cd better-spotlight
rustup default stable
xcodebuild -downloadComponent MetalToolchain
./scripts/run-local.sh
```

This builds and opens the local `.app` bundle, so macOS loads the app name,
icon, and other bundle metadata. Use `cargo run` only for a faster raw-binary
development cycle; a raw executable does not include the macOS app icon or
other bundle metadata.

Better Spotlight registers ⌘Space when it starts. macOS owns that shortcut by
default, so disable **System Settings → Keyboard → Keyboard Shortcuts → Spotlight
→ Show Spotlight search** before launching the app. If registration fails, the
launcher remains usable and shows the reason in its footer.

Run the complete validation suite:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Configuration

### Custom shortcut

Create `~/Library/Application Support/Better Spotlight/config` and set a
shortcut using `super`, `shift`, `alt`, or `control` plus a key:

```text
shortcut = super+shift+Space
```

Restart Better Spotlight after changing the file. You can temporarily override
the file with `BETTER_SPOTLIGHT_SHORTCUT`, for example
`BETTER_SPOTLIGHT_SHORTCUT=alt+Space cargo run`.

File results come from the macOS Spotlight index. If expected personal files do
not appear, allow Better Spotlight under **System Settings → Privacy & Security
→ Files & Folders**, and confirm the location is not excluded from Spotlight.

### Clipboard history

Press **⌘⇧V** while Better Spotlight is open to switch to clipboard history.
Type to filter captured text, use the arrow keys to navigate, and press Enter to
copy the selected entry. Press **⌘⌫** twice to delete the selected entry, or use
the visible Delete and Clear all controls with their confirmation step.

Clipboard history stays in memory, is limited to the 50 most recent unique text
entries, and is erased when Better Spotlight quits. Images and files are not
captured.

## Package locally

Create an ad-hoc signed application bundle and zip archive:

```sh
./scripts/package.sh
open "dist/Better Spotlight.app"
```

Public releases are distributed as a universal, Developer ID-signed and
notarized DMG attached to GitHub Releases. Signing and notarization happen
locally so Apple credentials never leave the release Mac. See
[docs/releasing.md](docs/releasing.md) for the release process.

## Roadmap

- [x] Global hotkey (⌘Space) toggle
- [x] File search via Spotlight metadata
- [x] Calculator
- [x] Clipboard history
- [ ] Plugin system

## License

MIT
