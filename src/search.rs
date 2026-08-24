use std::collections::HashSet;
use std::ops::Range;
use std::time::Instant;

use gpui::{
    Animation, AnimationExt, App, Bounds, Context, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, GlobalElementId, Hsla, InspectorElementId,
    IntoElement, KeyDownEvent, LayoutId, PaintQuad, Pixels, Render, ShapedLine, SharedString,
    Style, TextRun, UTF16Selection, Window, actions, div, fill, point, prelude::*, px, relative,
    size,
};

use crate::apps::{self, AppEntry};
use crate::fuzzy;
use crate::theme::{self, Theme};
use crate::ui;

actions!(search_input, [Paste]);

pub struct Spotlight {
    pub focus: FocusHandle,
    query: String,
    apps: Vec<AppEntry>,
    results: Vec<AppEntry>,
    selected: usize,
    visible_start: usize,
    move_count: u64,
    loading: bool,
    loading_icons: HashSet<String>,
    message: Option<String>,
    shortcut_error: Option<String>,
    marked_range: Option<Range<usize>>,
    last_input_layout: Option<ShapedLine>,
    last_input_bounds: Option<Bounds<Pixels>>,
    caret_blink_started: Instant,
}

impl Spotlight {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let dirs = app_dirs();
        let scan = cx
            .background_executor()
            .spawn(async move { apps::scan(&dirs) });

        cx.spawn(async move |this, cx| {
            let all_apps = scan.await;
            let _ = this.update(cx, |this, cx| {
                this.apps = all_apps;
                this.refilter();
                this.loading = false;
                cx.notify();
            });
        })
        .detach();

        Self {
            focus: cx.focus_handle(),
            query: String::new(),
            apps: Vec::new(),
            results: Vec::new(),
            selected: 0,
            visible_start: 0,
            move_count: 0,
            loading: true,
            loading_icons: HashSet::new(),
            message: None,
            shortcut_error: None,
            marked_range: None,
            last_input_layout: None,
            last_input_bounds: None,
            caret_blink_started: Instant::now(),
        }
    }

    pub fn activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.query.clear();
        self.marked_range = None;
        self.message = None;
        self.refilter();
        self.reset_caret_blink();
        self.focus.focus(window);
        window.activate_window();
        cx.notify();
    }

    pub fn set_shortcut_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.shortcut_error = Some(message);
        cx.notify();
    }

    fn reset_caret_blink(&mut self) {
        self.caret_blink_started = Instant::now();
    }

    fn caret_is_visible(&self) -> bool {
        self.caret_blink_started.elapsed().as_millis() % 1_060 < 530
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
            scored.sort_by(|a, b| {
                b.0.cmp(&a.0)
                    .then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()))
            });
            self.results = scored.into_iter().map(|(_, app)| app).collect();
        }
        self.selected = 0;
        self.visible_start = 0;
        self.message = None;
    }

    fn move_selection_up(&mut self) {
        move_selection_up(&mut self.selected, &mut self.visible_start);
        self.move_count += 1;
    }

    fn move_selection_down(&mut self) {
        move_selection_down(
            &mut self.selected,
            &mut self.visible_start,
            self.results.len(),
        );
        self.move_count += 1;
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        match event.keystroke.key.as_ref() {
            "backspace" => {
                if modifiers.platform {
                    self.query.clear();
                } else if modifiers.alt {
                    delete_word(&mut self.query);
                } else {
                    self.query.pop();
                }
                self.marked_range = None;
                self.refilter();
                self.reset_caret_blink();
            }
            "escape" => cx.hide(),
            "enter" => {
                if let Some(app) = self.results.get(self.selected) {
                    match apps::launch(&app.path) {
                        Ok(()) => cx.hide(),
                        Err(error) => {
                            self.message = Some(format!("Could not open {}: {error}", app.name));
                        }
                    }
                }
            }
            "up" => self.move_selection_up(),
            "down" => self.move_selection_down(),
            _ => {}
        }
        cx.notify();
    }

    fn paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.query.push_str(&text.replace(['\r', '\n'], " "));
            self.refilter();
            self.reset_caret_blink();
            cx.notify();
        }
    }

    fn queue_visible_icons(&mut self, cx: &mut Context<Self>) {
        let visible: Vec<_> = self
            .results
            .iter()
            .skip(self.visible_start)
            .take(theme::MAX_VISIBLE_RESULTS)
            .filter(|app| app.icon.is_none())
            .filter_map(|app| Some((app.path.clone(), app.icon_path.clone()?)))
            .filter(|(app_path, _)| self.loading_icons.insert(app_path.clone()))
            .collect();

        for (app_path, icon_path) in visible {
            let task = cx
                .background_executor()
                .spawn(async move { apps::load_icon(&icon_path) });
            cx.spawn(async move |this, cx| {
                let icon = task.await;
                let _ = this.update(cx, |this, cx| {
                    for app in this
                        .apps
                        .iter_mut()
                        .chain(this.results.iter_mut())
                        .filter(|app| app.path == app_path)
                    {
                        app.icon.clone_from(&icon);
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }
}

fn move_selection_up(selected: &mut usize, visible_start: &mut usize) {
    *selected = selected.saturating_sub(1);
    if *selected < *visible_start {
        *visible_start = *selected;
    }
}

fn move_selection_down(selected: &mut usize, visible_start: &mut usize, result_count: usize) {
    if *selected + 1 < result_count {
        *selected += 1;
        if *selected >= *visible_start + theme::MAX_VISIBLE_RESULTS {
            *visible_start = *selected + 1 - theme::MAX_VISIBLE_RESULTS;
        }
    }
}

fn app_dirs() -> Vec<String> {
    let mut dirs = vec![
        "/Applications".to_string(),
        "/System/Applications".to_string(),
        "/System/Applications/Utilities".to_string(),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(format!("{home}/Applications"));
    }
    dirs
}

/// Deletes the word before the implicit end-of-query caret.
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
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl EntityInputHandler for Spotlight {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        self.query.get(range).map(str::to_owned)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let end = self.query.encode_utf16().count();
        Some(UTF16Selection {
            range: end..end,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or(self.query.len()..self.query.len());
        let text = new_text.replace(['\r', '\n'], " ");
        self.query.replace_range(range, &text);
        self.marked_range = None;
        self.refilter();
        self.reset_caret_blink();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or(self.query.len()..self.query.len());
        let start = range.start;
        self.query.replace_range(range, new_text);
        self.marked_range = (!new_text.is_empty()).then_some(start..start + new_text.len());
        self.refilter();
        self.reset_caret_blink();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.last_input_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(bounds.left() + line.x_for_index(range.start), bounds.top()),
            point(bounds.left() + line.x_for_index(range.end), bounds.bottom()),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line = self.last_input_layout.as_ref()?;
        let bounds = self.last_input_bounds?;
        line.index_for_x(point.x - bounds.left())
            .map(|index| self.query[..index].encode_utf16().count())
    }
}

impl Spotlight {
    fn offset_from_utf16(&self, offset: usize) -> usize {
        self.query
            .chars()
            .scan((0, 0), |(utf8, utf16), character| {
                let current = (*utf8, *utf16);
                *utf8 += character.len_utf8();
                *utf16 += character.len_utf16();
                Some(current)
            })
            .find_map(|(utf8, utf16)| (utf16 >= offset).then_some(utf8))
            .unwrap_or(self.query.len())
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        self.query[..offset].encode_utf16().count()
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }
}

struct SearchTextElement {
    spotlight: Entity<Spotlight>,
}

struct SearchTextPrepaint {
    line: ShapedLine,
    caret: PaintQuad,
    text_origin: gpui::Point<Pixels>,
}

impl IntoElement for SearchTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SearchTextElement {
    type RequestLayoutState = ();
    type PrepaintState = SearchTextPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(24.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let spotlight = self.spotlight.read(cx);
        let (text, color): (SharedString, Hsla) = if spotlight.query.is_empty() {
            ("Search applications…".into(), Theme::PLACEHOLDER.into())
        } else {
            (spotlight.query.clone().into(), Theme::INPUT_TEXT.into())
        };
        let run = TextRun {
            len: text.len(),
            font: window.text_style().font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(text, font_size, &[run], None);
        let text_origin = point(
            if spotlight.query.is_empty() {
                bounds.left() + px(6.)
            } else {
                bounds.left()
            },
            bounds.top(),
        );
        let caret_x = if spotlight.query.is_empty() {
            bounds.left()
        } else {
            text_origin.x + line.x_for_index(spotlight.query.len())
        };
        let caret_height = line.ascent + line.descent;
        let caret_top = bounds.top() + (bounds.size.height - caret_height) / 2.;
        let caret = fill(
            Bounds::new(
                point(caret_x + px(2.), caret_top),
                size(px(2.), caret_height),
            ),
            Theme::CARET,
        );
        SearchTextPrepaint {
            line,
            caret,
            text_origin,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (focus, caret_is_visible) = {
            let spotlight = self.spotlight.read(cx);
            (spotlight.focus.clone(), spotlight.caret_is_visible())
        };
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.spotlight.clone()),
            cx,
        );
        let _ = prepaint
            .line
            .paint(prepaint.text_origin, bounds.size.height, window, cx);
        if focus.is_focused(window) {
            window.request_animation_frame();
            if caret_is_visible {
                window.paint_quad(prepaint.caret.clone());
            }
        }
        self.spotlight.update(cx, |spotlight, _| {
            spotlight.last_input_layout = Some(prepaint.line.clone());
            spotlight.last_input_bounds = Some(bounds);
        });
    }
}

impl Render for Spotlight {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.queue_visible_icons(cx);

        let rows: Vec<_> = self
            .results
            .iter()
            .enumerate()
            .skip(self.visible_start)
            .take(theme::MAX_VISIBLE_RESULTS)
            .map(|(index, app)| {
                ui::result_row::result_row(app, index == self.selected, self.move_count)
                    .into_any_element()
            })
            .collect();

        let body = if self.loading {
            ui::empty_state("Finding applications…")
        } else if rows.is_empty() {
            ui::empty_state("No results")
        } else {
            ui::results_list(rows)
        };

        div()
            .id("spotlight")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::paste))
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
                    .child(ui::input_field(SearchTextElement {
                        spotlight: cx.entity(),
                    }))
                    .child(body)
                    .child(ui::footer(
                        self.message.as_deref().or(self.shortcut_error.as_deref()),
                    )),
            )
            .with_animation(
                "panel-open",
                Animation::new(std::time::Duration::from_millis(160)),
                |element, delta| {
                    let ease = 1.0 - (1.0 - delta) * (1.0 - delta);
                    element.opacity(ease).top(px((1.0 - ease) * 6.0))
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_window_follows_downward_navigation() {
        let mut selected = 0;
        let mut visible_start = 0;
        for _ in 0..8 {
            move_selection_down(&mut selected, &mut visible_start, 12);
        }
        assert_eq!(selected, 8);
        assert_eq!(visible_start, 2);
        assert!(selected < visible_start + theme::MAX_VISIBLE_RESULTS);
    }

    #[test]
    fn selection_window_follows_upward_navigation() {
        let mut selected = 0;
        let mut visible_start = 0;
        for _ in 0..9 {
            move_selection_down(&mut selected, &mut visible_start, 12);
        }
        for _ in 0..7 {
            move_selection_up(&mut selected, &mut visible_start);
        }
        assert_eq!(selected, 2);
        assert_eq!(visible_start, 2);
    }

    #[test]
    fn delete_word_handles_unicode() {
        let mut query = "Open 日本語".to_string();
        delete_word(&mut query);
        assert_eq!(query, "Open ");
    }
}
