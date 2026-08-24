# better-spotlight

A fast, Raycast-inspired launcher for macOS built with [GPUI](https://gpui.rs) — the GPU-accelerated UI framework behind Zed.

## Features

- Instant fuzzy search over your applications
- Real app icons decoded from `.icns` bundles
- Keyboard-first: type to search, arrows to navigate, Enter to launch
- Mouse support with hover selection and double-click launch
- File and folder search backed by the macOS Spotlight index
- Borderless floating panel, always on top
- Global ⌘Space toggle

## Building

Requires macOS with Xcode and the Metal Toolchain:

```sh
xcodebuild -downloadComponent MetalToolchain
cargo run
```

Better Spotlight registers ⌘Space when it starts. macOS owns that shortcut by
default, so disable **System Settings → Keyboard → Keyboard Shortcuts → Spotlight
→ Show Spotlight search** before launching the app. If registration fails, the
launcher remains usable and shows the reason in its footer.

## Packaging

Create an ad-hoc signed application bundle and zip archive:

```sh
./scripts/package.sh
open "dist/Better Spotlight.app"
```

For distribution, provide a Developer ID identity through
`CODESIGN_IDENTITY`, then notarize the generated archive with your Apple
developer credentials.

## Roadmap

- [x] Global hotkey (⌘Space) toggle
- [x] File search via Spotlight metadata
- [ ] Calculator, clipboard history
- [ ] Plugin system

## License

MIT
