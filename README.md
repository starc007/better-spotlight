# better-spotlight

A fast, Raycast-inspired launcher for macOS built with [GPUI](https://gpui.rs) — the GPU-accelerated UI framework behind Zed.

## Features

- Instant fuzzy search over your applications
- Real app icons decoded from `.icns` bundles
- Keyboard-first: type to search, arrows to navigate, Enter to launch
- Borderless floating panel, always on top

## Building

Requires macOS with Xcode and the Metal Toolchain:

```sh
xcodebuild -downloadComponent MetalToolchain
cargo run
```

## Roadmap

- [ ] Global hotkey (⌘Space) toggle
- [ ] File search via Spotlight metadata / custom index
- [ ] Calculator, clipboard history
- [ ] Plugin system

## License

MIT
