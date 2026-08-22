use gpui::{
    div, prelude::*, px, Animation, AnimationExt, Context, FocusHandle, Focusable, KeyDownEvent,
    Render, Window,
};

use crate::apps::{self, AppEntry};
use crate::fuzzy;
use crate::theme::{self, Theme};
use crate::ui;

pub struct Spotlight {
    pub focus: FocusHandle,
    query: String,
    apps: Vec<AppEntry>,
    results: Vec<AppEntry>,
    selected: usize,
    move_count: u64,
}

impl Spotlight {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let all_apps = apps::scan(&app_dirs());
        let results = all_apps.clone();

        Self {
            focus: cx.focus_handle(),
            query: String::new(),
            apps: all_apps,
            results,
            selected: 0,
            move_count: 0,
        }
    }

    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.results = self.apps.clone();
        } else {
            let mut scored: Vec<(i32, AppEntry)> = self
                .apps
                .iter()
                .filter_map(|app| fuzzy::score(&self.query, &app.name).map(|s| (s, app.clone())))
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            self.results = scored.into_iter().map(|(_, app)| app).collect();
        }
        self.selected = 0;
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_ref() {
            "backspace" => {
                let modifiers = event.keystroke.modifiers;
                if modifiers.platform {
                    // ⌘⌫: clear the whole line
                    self.query.clear();
                } else if modifiers.alt {
                    // ⌥⌫: delete the previous word
                    delete_word(&mut self.query);
                } else {
                    self.query.pop();
                }
                self.refilter();
            }
            "escape" => {
                window.remove_window();
            }
            "enter" => {
                if let Some(app) = self.results.get(self.selected) {
                    apps::launch(&app.path);
                }
                window.remove_window();
            }
            "up" => {
                self.selected = self.selected.saturating_sub(1);
                self.move_count += 1;
            }
            "down" => {
                if self.selected + 1 < self.results.len() {
                    self.selected += 1;
                    self.move_count += 1;
                }
            }
            key => {
                if key.len() == 1 && event.keystroke.modifiers == Default::default() {
                    self.query.push_str(key);
                    self.refilter();
                    self.move_count += 1;
                }
            }
        }
        cx.notify();
    }
}

fn app_dirs() -> Vec<String> {
    let mut dirs = vec!["/Applications".to_string()];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(format!("{home}/Applications"));
    }
    dirs
}

/// Deletes the word before the (implicit) caret, like standard macOS text fields.
fn delete_word(query: &mut String) {
    let mut chars: Vec<char> = query.chars().collect();
    while let Some(&c) = chars.last() {
        chars.pop();
        if c.is_alphanumeric() {
            break;
        }
    }
    while let Some(&c) = chars.last() {
        if c.is_alphanumeric() {
            chars.pop();
        } else {
            break;
        }
    }
    *query = chars.into_iter().collect();
}

impl Focusable for Spotlight {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Spotlight {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<_> = self
            .results
            .iter()
            .enumerate()
            .take(theme::MAX_VISIBLE_RESULTS)
            .map(|(i, app)| {
                ui::result_row::result_row(app, i == self.selected, self.move_count)
                    .into_any_element()
            })
            .collect();

        let body: Vec<_> = if rows.is_empty() {
            vec![ui::empty_state().into_any_element()]
        } else {
            vec![ui::results_list(rows).into_any_element()]
        };

        div()
            .id("spotlight")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .size_full()
            .p_2()
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded_xl()
                    .bg(Theme::BG)
                    .shadow_xl()
                    .child(ui::input_field(&self.query))
                    .children(body)
                    .child(ui::footer()),
            )
            .with_animation(
                "panel-open",
                Animation::new(std::time::Duration::from_millis(160)),
                |el, delta| {
                    let ease = 1.0 - (1.0 - delta) * (1.0 - delta);
                    el.opacity(ease).top(px((1.0 - ease) * 6.0))
                },
            )
    }
}
