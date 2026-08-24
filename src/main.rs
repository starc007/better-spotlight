mod apps;
mod calculator;
mod clipboard_history;
mod config;
mod files;
mod fuzzy;
mod results;
mod search;
mod theme;
mod ui;

use std::time::Duration;

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use gpui::{
    App, AppContext, Application, Bounds, Global, KeyBinding, Point, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, px, size,
};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

struct HotKeyRegistration {
    _manager: GlobalHotKeyManager,
}

impl Global for HotKeyRegistration {}

fn configure_as_background_launcher() {
    let main_thread = MainThreadMarker::new().expect("GPUI launches the app on the main thread");
    let application = NSApplication::sharedApplication(main_thread);
    assert!(
        application.setActivationPolicy(NSApplicationActivationPolicy::Accessory),
        "macOS rejected the accessory activation policy"
    );
}

fn main() {
    let shortcut = config::load_shortcut();
    Application::new().run(move |cx: &mut App| {
        // GPUI 0.2 sets the process to a regular app after launch, overriding
        // LSUIElement. Restore accessory behavior so the launcher stays out of
        // the Dock and Command-Tab switcher.
        configure_as_background_launcher();

        cx.bind_keys([
            KeyBinding::new("cmd-v", search::Paste, None),
            KeyBinding::new("cmd-a", search::SelectAllInput, None),
            KeyBinding::new("cmd-shift-v", search::OpenClipboardHistory, None),
            KeyBinding::new("cmd-backspace", search::DeleteClipboardEntry, None),
            KeyBinding::new("cmd-shift-backspace", search::ClearClipboardHistory, None),
        ]);

        let hotkey = shortcut.hotkey;
        let hotkey_id = hotkey.id();
        let hotkey_error = match GlobalHotKeyManager::new() {
            Ok(manager) => match manager.register(hotkey) {
                Ok(()) => {
                    cx.set_global(HotKeyRegistration { _manager: manager });
                    shortcut.warning.clone()
                }
                Err(error) => Some(format!(
                    "{} is unavailable ({error}). Change it in {}.",
                    shortcut.label,
                    config::config_path()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "BETTER_SPOTLIGHT_SHORTCUT".into())
                )),
            },
            Err(error) => Some(format!("Could not initialize the global shortcut: {error}")),
        };

        let window_size = size(px(theme::WINDOW_WIDTH), px(theme::WINDOW_HEIGHT));

        let screen = cx
            .displays()
            .first()
            .map(|d| d.bounds())
            .unwrap_or_else(|| Bounds::new(Point::new(px(0.), px(0.)), size(px(1440.), px(900.))));
        let x = screen.origin.x + (screen.size.width - window_size.width) / 2.;
        let y = screen.origin.y + screen.size.height * 0.18;
        let bounds = Bounds::new(Point::new(x, y), window_size);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: false,
            show: false,
            kind: WindowKind::PopUp,
            is_movable: false,
            is_resizable: false,
            window_background: WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };

        let handle = cx
            .open_window(options, |window, cx| {
                let shortcut_label = shortcut.label.clone();
                let view = cx.new(|cx| search::Spotlight::new(cx, shortcut_label));
                view.update(cx, |_spotlight, cx| {
                    cx.observe_window_activation(window, |_spotlight, window, cx| {
                        if !window.is_window_active() {
                            cx.hide();
                        }
                    })
                    .detach();
                });
                if let Some(error) = hotkey_error {
                    view.update(cx, |spotlight, cx| spotlight.set_shortcut_error(error, cx));
                }
                view
            })
            .unwrap();

        let clipboard_handle = handle;
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                    if event.id == hotkey_id && event.state == HotKeyState::Pressed {
                        let _ = cx.update(|cx| {
                            let _ = handle.update(cx, |spotlight, window, cx| {
                                if window.is_window_active() {
                                    cx.hide();
                                } else {
                                    cx.activate(true);
                                    spotlight.activate(window, cx);
                                }
                            });
                        });
                    }
                }
            }
        })
        .detach();

        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(400))
                    .await;
                let _ = cx.update(|cx| {
                    let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
                        return;
                    };
                    let _ = clipboard_handle.update(cx, |spotlight, _window, cx| {
                        spotlight.capture_clipboard(text, cx);
                    });
                });
            }
        })
        .detach();
    });
}
