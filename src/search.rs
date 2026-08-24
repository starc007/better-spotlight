use std::collections::HashSet;
use std::ops::Range;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use gpui::{
    Animation, AnimationExt, App, Bounds, ClickEvent, ClipboardItem, Context, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, KeyDownEvent, LayoutId, PaintQuad, Pixels, Render,
    ScrollWheelEvent, ShapedLine, SharedString, Style, TextRun, UTF16Selection, Window, actions,
    div, fill, point, prelude::*, px, relative, size,
};

use crate::apps::{self, AppEntry};
use crate::calculator;
use crate::clipboard_history::ClipboardHistory;
use crate::files::{self, FileEntry};
use crate::fuzzy;
use crate::results::SearchResult;
use crate::theme::{self, Theme};
use crate::ui;

actions!(
    search_input,
    [
        Paste,
        SelectAllInput,
        OpenClipboardHistory,
        DeleteClipboardEntry,
        ClearClipboardHistory
    ]
);

#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchMode {
    Applications,
    Clipboard,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingClipboardAction {
    Delete(u64),
    Clear,
}

pub struct Spotlight {
    pub focus: FocusHandle,
    query: String,
    selected_range: Range<usize>,
    mode: SearchMode,
    apps: Vec<AppEntry>,
    clipboard_history: ClipboardHistory,
    files: Vec<FileEntry>,
    results: Vec<SearchResult>,
    selected: usize,
    visible_start: usize,
    scroll_remainder: Pixels,
    move_count: u64,
    loading: bool,
    files_loading: bool,
    file_request_id: Arc<AtomicU64>,
    loading_icons: HashSet<String>,
    message: Option<String>,
    shortcut_error: Option<String>,
    shortcut_label: String,
    pending_clipboard_action: Option<PendingClipboardAction>,
    marked_range: Option<Range<usize>>,
    last_input_layout: Option<ShapedLine>,
    last_input_bounds: Option<Bounds<Pixels>>,
    caret_blink_started: Instant,
}

impl Spotlight {
    pub fn new(cx: &mut Context<Self>, shortcut_label: String) -> Self {
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
            selected_range: 0..0,
            mode: SearchMode::Applications,
            apps: Vec::new(),
            clipboard_history: ClipboardHistory::default(),
            files: Vec::new(),
            results: Vec::new(),
            selected: 0,
            visible_start: 0,
            scroll_remainder: Pixels::ZERO,
            move_count: 0,
            loading: true,
            files_loading: false,
            file_request_id: Arc::new(AtomicU64::new(0)),
            loading_icons: HashSet::new(),
            message: None,
            shortcut_error: None,
            shortcut_label,
            pending_clipboard_action: None,
            marked_range: None,
            last_input_layout: None,
            last_input_bounds: None,
            caret_blink_started: Instant::now(),
        }
    }

    pub fn activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.query.clear();
        self.selected_range = 0..0;
        self.mode = SearchMode::Applications;
        self.files.clear();
        self.file_request_id.fetch_add(1, Ordering::Relaxed);
        self.files_loading = false;
        self.marked_range = None;
        self.message = None;
        self.pending_clipboard_action = None;
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

    pub fn capture_clipboard(&mut self, text: String, cx: &mut Context<Self>) {
        if self.clipboard_history.capture(text) && self.mode == SearchMode::Clipboard {
            self.refilter();
            cx.notify();
        }
    }

    fn reset_caret_blink(&mut self) {
        self.caret_blink_started = Instant::now();
    }

    fn clear_query(&mut self, cx: &mut Context<Self>) {
        self.query.clear();
        self.selected_range = 0..0;
        self.marked_range = None;
        self.query_changed(cx);
        self.reset_caret_blink();
        cx.notify();
    }

    fn caret_is_visible(&self) -> bool {
        self.caret_blink_started.elapsed().as_millis() % 1_060 < 530
    }

    fn refilter(&mut self) {
        if self.mode == SearchMode::Clipboard {
            self.results = self
                .clipboard_history
                .search(&self.query)
                .into_iter()
                .map(SearchResult::Clipboard)
                .collect();
            self.reset_result_window();
            return;
        }

        if self.query.is_empty() {
            self.results = self
                .apps
                .iter()
                .cloned()
                .map(SearchResult::Application)
                .collect();
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
            self.results = calculator::evaluate(&self.query)
                .map(|result| SearchResult::Calculation {
                    expression: self.query.clone(),
                    result,
                })
                .into_iter()
                .chain(
                    scored
                        .into_iter()
                        .map(|(_, app)| SearchResult::Application(app)),
                )
                .chain(self.files.iter().cloned().map(SearchResult::File))
                .collect();
        }
        self.reset_result_window();
    }

    fn reset_result_window(&mut self) {
        self.selected = 0;
        self.visible_start = 0;
        self.scroll_remainder = Pixels::ZERO;
        self.message = None;
        self.pending_clipboard_action = None;
    }

    fn query_changed(&mut self, cx: &mut Context<Self>) {
        self.files.clear();
        let request_id = self.file_request_id.fetch_add(1, Ordering::Relaxed) + 1;
        let latest_request = self.file_request_id.clone();
        let query = self.query.trim().to_string();
        self.files_loading = !query.is_empty();
        self.refilter();

        if self.mode == SearchMode::Clipboard {
            self.files_loading = false;
            return;
        }

        if query.is_empty() || calculator::evaluate(&query).is_some() {
            self.files_loading = false;
            return;
        }

        let executor = cx.background_executor().clone();
        let timer = executor.clone();
        let task = executor.spawn(async move {
            timer.timer(Duration::from_millis(180)).await;
            (latest_request.load(Ordering::Relaxed) == request_id).then(|| files::search(&query))
        });
        cx.spawn(async move |this, cx| {
            let Some(search_result) = task.await else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                if this.file_request_id.load(Ordering::Relaxed) != request_id {
                    return;
                }
                this.files_loading = false;
                match search_result {
                    Ok(matches) => {
                        let selected = this.selected;
                        let visible_start = this.visible_start;
                        this.files = matches;
                        this.refilter();
                        if !this.results.is_empty() {
                            this.selected = selected.min(this.results.len() - 1);
                            this.visible_start = visible_start.min(this.selected);
                        }
                    }
                    Err(error) => {
                        this.message = Some(format!("File search unavailable: {error}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn move_selection_up(&mut self) {
        move_selection_up(&mut self.selected, &mut self.visible_start);
        self.pending_clipboard_action = None;
        self.move_count += 1;
    }

    fn move_selection_down(&mut self) {
        let visible_limit = self.visible_limit();
        move_selection_down(
            &mut self.selected,
            &mut self.visible_start,
            self.results.len(),
            visible_limit,
        );
        self.keep_selection_visible();
        self.pending_clipboard_action = None;
        self.move_count += 1;
    }

    fn visible_limit(&self) -> usize {
        if self.mode == SearchMode::Clipboard {
            theme::MULTI_GROUP_VISIBLE_RESULTS
        } else {
            visible_result_limit(&self.results, self.visible_start)
        }
    }

    fn keep_selection_visible(&mut self) {
        for _ in 0..2 {
            let visible_limit = self.visible_limit();
            if self.selected < self.visible_start + visible_limit {
                break;
            }
            self.visible_start = self.selected + 1 - visible_limit;
        }
    }

    fn on_results_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let visible_limit = self.visible_limit();
        if self.results.len() <= visible_limit {
            return;
        }

        self.scroll_remainder += event.delta.pixel_delta(px(theme::RESULT_ROW_HEIGHT)).y;
        let threshold = px(theme::RESULT_ROW_HEIGHT / 2.);
        let steps = (self.scroll_remainder.abs() / threshold).floor() as usize;
        if steps == 0 {
            return;
        }

        let scroll_down = self.scroll_remainder < Pixels::ZERO;
        if scroll_down {
            self.scroll_remainder += threshold * steps;
        } else {
            self.scroll_remainder -= threshold * steps;
        }

        if scroll_results(
            &mut self.selected,
            &mut self.visible_start,
            self.results.len(),
            visible_limit,
            scroll_down,
            steps,
        ) {
            self.keep_selection_visible();
            self.move_count += 1;
            cx.notify();
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        match event.keystroke.key.as_ref() {
            "backspace" => {
                if !modifiers.platform {
                    let deleted_selection =
                        delete_selected_text(&mut self.query, &mut self.selected_range);
                    if !deleted_selection {
                        if modifiers.alt {
                            delete_word(&mut self.query);
                        } else {
                            self.query.pop();
                        }
                        let end = self.query.len();
                        self.selected_range = end..end;
                    }
                    self.marked_range = None;
                    self.query_changed(cx);
                    self.reset_caret_blink();
                }
            }
            "escape" => {
                if self.mode == SearchMode::Clipboard {
                    self.show_application_search(cx);
                } else {
                    cx.hide();
                }
            }
            "enter" => {
                self.open_result(self.selected, cx);
            }
            "up" => self.move_selection_up(),
            "down" => self.move_selection_down(),
            _ => {}
        }
        cx.notify();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace(['\r', '\n'], " "), window, cx);
        }
    }

    fn select_all_input(
        &mut self,
        _: &SelectAllInput,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_range = 0..self.query.len();
        self.marked_range = None;
        cx.notify();
    }

    fn open_clipboard_history(
        &mut self,
        _: &OpenClipboardHistory,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.mode == SearchMode::Clipboard {
            self.show_application_search(cx);
        } else {
            self.mode = SearchMode::Clipboard;
            self.query.clear();
            self.selected_range = 0..0;
            self.files.clear();
            self.file_request_id.fetch_add(1, Ordering::Relaxed);
            self.files_loading = false;
            self.marked_range = None;
            self.refilter();
            self.reset_caret_blink();
            cx.notify();
        }
    }

    fn delete_clipboard_entry(
        &mut self,
        _: &DeleteClipboardEntry,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.mode == SearchMode::Clipboard && self.query.is_empty() {
            self.request_delete_selected(cx);
        } else {
            self.clear_query(cx);
        }
    }

    fn clear_clipboard_history(
        &mut self,
        _: &ClearClipboardHistory,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.mode == SearchMode::Clipboard && self.query.is_empty() {
            self.request_clear_clipboard(cx);
        } else if !self.query.is_empty() {
            self.clear_query(cx);
        }
    }

    fn show_application_search(&mut self, cx: &mut Context<Self>) {
        self.mode = SearchMode::Applications;
        self.query.clear();
        self.selected_range = 0..0;
        self.marked_range = None;
        self.refilter();
        self.reset_caret_blink();
        cx.notify();
    }

    fn request_delete_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self
            .results
            .get(self.selected)
            .and_then(SearchResult::clipboard_entry)
            .map(|entry| entry.id)
        else {
            return;
        };
        self.request_delete_clipboard(id, cx);
    }

    fn request_delete_clipboard(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.pending_clipboard_action == Some(PendingClipboardAction::Delete(id)) {
            self.clipboard_history.delete(id);
            self.refilter();
        } else {
            self.pending_clipboard_action = Some(PendingClipboardAction::Delete(id));
        }
        cx.notify();
    }

    fn request_clear_clipboard(&mut self, cx: &mut Context<Self>) {
        if self.clipboard_history.is_empty() {
            return;
        }
        if self.pending_clipboard_action == Some(PendingClipboardAction::Clear) {
            self.clipboard_history.clear();
            self.refilter();
        } else {
            self.pending_clipboard_action = Some(PendingClipboardAction::Clear);
        }
        cx.notify();
    }

    fn queue_visible_icons(&mut self, cx: &mut Context<Self>) {
        let visible_limit = self.visible_limit();
        let visible_apps: Vec<_> = self
            .results
            .iter()
            .skip(self.visible_start)
            .take(visible_limit)
            .filter_map(SearchResult::app)
            .filter(|app| app.icon.is_none())
            .filter_map(|app| Some((app.path.clone(), app.icon_path.clone()?)))
            .filter(|(app_path, _)| self.loading_icons.insert(app_path.clone()))
            .collect();

        for (app_path, icon_path) in visible_apps {
            let task = cx
                .background_executor()
                .spawn(async move { apps::load_icon(&icon_path) });
            cx.spawn(async move |this, cx| {
                let icon = task.await;
                let _ = this.update(cx, |this, cx| {
                    for app in this.apps.iter_mut().filter(|app| app.path == app_path) {
                        app.icon.clone_from(&icon);
                    }
                    for result in &mut this.results {
                        if let SearchResult::Application(app) = result
                            && app.path == app_path
                        {
                            app.icon.clone_from(&icon);
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
        }

        let visible_files: Vec<_> = self
            .results
            .iter()
            .skip(self.visible_start)
            .take(visible_limit)
            .filter_map(|result| match result {
                SearchResult::File(file) if file.icon.is_none() => Some(file.path.clone()),
                _ => None,
            })
            .filter(|path| self.loading_icons.insert(path.clone()))
            .collect();

        for file_path in visible_files {
            let path_for_load = file_path.clone();
            let task = cx
                .background_executor()
                .spawn(async move { files::load_icon(&path_for_load) });
            cx.spawn(async move |this, cx| {
                let icon = task.await;
                let _ = this.update(cx, |this, cx| {
                    for file in this.files.iter_mut().filter(|file| file.path == file_path) {
                        file.icon.clone_from(&icon);
                    }
                    for result in &mut this.results {
                        if let SearchResult::File(file) = result
                            && file.path == file_path
                        {
                            file.icon.clone_from(&icon);
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }

    fn select_result(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.results.len() && self.selected != index {
            self.selected = index;
            self.pending_clipboard_action = None;
            self.move_count += 1;
            cx.notify();
        }
    }

    fn on_result_click(&mut self, index: usize, event: &ClickEvent, cx: &mut Context<Self>) {
        if !event.standard_click() {
            return;
        }
        self.select_result(index, cx);
        if event.click_count() >= 2 {
            self.open_result(index, cx);
        }
    }

    fn open_result(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(result) = self.results.get(index) else {
            return;
        };
        if let Some(value) = result.calculation_result() {
            cx.write_to_clipboard(ClipboardItem::new_string(value.to_string()));
            cx.hide();
            return;
        }
        if let Some(entry) = result.clipboard_entry() {
            cx.write_to_clipboard(ClipboardItem::new_string(entry.text.clone()));
            cx.hide();
            return;
        }
        let name = result.name().to_string();
        let Some(path) = result.path().map(str::to_string) else {
            return;
        };
        match apps::launch(&path) {
            Ok(()) => cx.hide(),
            Err(error) => self.message = Some(format!("Could not open {name}: {error}")),
        }
    }
}

fn move_selection_up(selected: &mut usize, visible_start: &mut usize) {
    *selected = selected.saturating_sub(1);
    if *selected < *visible_start {
        *visible_start = *selected;
    }
}

fn move_selection_down(
    selected: &mut usize,
    visible_start: &mut usize,
    result_count: usize,
    visible_limit: usize,
) {
    if *selected + 1 < result_count {
        *selected += 1;
        if *selected >= *visible_start + visible_limit {
            *visible_start = *selected + 1 - visible_limit;
        }
    }
}

fn visible_result_limit(results: &[SearchResult], visible_start: usize) -> usize {
    let mut kinds = results
        .iter()
        .skip(visible_start)
        .take(theme::MAX_VISIBLE_RESULTS)
        .map(SearchResult::kind);
    let Some(first_kind) = kinds.next() else {
        return theme::MAX_VISIBLE_RESULTS;
    };

    if kinds.any(|kind| kind != first_kind) {
        theme::MULTI_GROUP_VISIBLE_RESULTS
    } else {
        theme::MAX_VISIBLE_RESULTS
    }
}

fn scroll_results(
    selected: &mut usize,
    visible_start: &mut usize,
    result_count: usize,
    visible_limit: usize,
    scroll_down: bool,
    steps: usize,
) -> bool {
    if result_count <= visible_limit || steps == 0 {
        return false;
    }

    let previous_start = *visible_start;
    let max_start = result_count - visible_limit;
    *visible_start = if scroll_down {
        visible_start.saturating_add(steps).min(max_start)
    } else {
        visible_start.saturating_sub(steps)
    };
    if *visible_start == previous_start {
        return false;
    }

    let visible_end = (*visible_start + visible_limit - 1).min(result_count - 1);
    *selected = (*selected).clamp(*visible_start, visible_end);
    true
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

fn delete_selected_text(query: &mut String, selected_range: &mut Range<usize>) -> bool {
    if selected_range.start == selected_range.end {
        return false;
    }
    let start = selected_range.start;
    query.replace_range(selected_range.clone(), "");
    *selected_range = start..start;
    true
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
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
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
            .unwrap_or_else(|| self.selected_range.clone());
        let text = new_text.replace(['\r', '\n'], " ");
        let cursor = range.start + text.len();
        self.query.replace_range(range, &text);
        self.selected_range = cursor..cursor;
        self.marked_range = None;
        self.query_changed(cx);
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
            .unwrap_or_else(|| self.selected_range.clone());
        let start = range.start;
        self.query.replace_range(range, new_text);
        self.marked_range = (!new_text.is_empty()).then_some(start..start + new_text.len());
        let cursor = start + new_text.len();
        self.selected_range = cursor..cursor;
        self.query_changed(cx);
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
    caret: Option<PaintQuad>,
    selection: Option<PaintQuad>,
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
            (
                if spotlight.mode == SearchMode::Clipboard {
                    "Search clipboard history…"
                } else {
                    "Search applications…"
                }
                .into(),
                Theme::PLACEHOLDER.into(),
            )
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
        let (caret, selection) = if spotlight.selected_range.is_empty() {
            (
                Some(fill(
                    Bounds::new(
                        point(caret_x + px(2.), caret_top),
                        size(px(2.), caret_height),
                    ),
                    Theme::CARET,
                )),
                None,
            )
        } else {
            (
                None,
                Some(fill(
                    Bounds::from_corners(
                        point(
                            text_origin.x + line.x_for_index(spotlight.selected_range.start),
                            caret_top,
                        ),
                        point(
                            text_origin.x + line.x_for_index(spotlight.selected_range.end),
                            caret_top + caret_height,
                        ),
                    ),
                    Theme::SELECTED_BG,
                )),
            )
        };
        SearchTextPrepaint {
            line,
            caret,
            selection,
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
        if let Some(selection) = prepaint.selection.as_ref() {
            window.paint_quad(selection.clone());
        }
        let _ = prepaint
            .line
            .paint(prepaint.text_origin, bounds.size.height, window, cx);
        if focus.is_focused(window) {
            window.request_animation_frame();
            if caret_is_visible && let Some(caret) = prepaint.caret.as_ref() {
                window.paint_quad(caret.clone());
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

        let clipboard_mode = self.mode == SearchMode::Clipboard;
        let visible_limit = self.visible_limit();
        let visible_results: Vec<_> = self
            .results
            .iter()
            .enumerate()
            .skip(self.visible_start)
            .take(visible_limit)
            .map(|(index, result)| (index, result.clone()))
            .collect();
        let selected = self.selected;
        let move_count = self.move_count;
        let pending_clipboard_action = self.pending_clipboard_action;
        let mut rows = Vec::new();
        let mut previous_kind = None;
        for (index, result) in visible_results {
            let kind = result.kind();
            if !clipboard_mode && previous_kind != Some(kind) {
                rows.push(ui::section_header(kind.label()).into_any_element());
                previous_kind = Some(kind);
            }
            let clipboard_id = result.clipboard_entry().map(|entry| entry.id);
            let delete_confirmation = clipboard_id.is_some_and(|id| {
                pending_clipboard_action == Some(PendingClipboardAction::Delete(id))
            });
            rows.push(ui::result_row::result_row(
                &result,
                index,
                ui::result_row::ResultRowState {
                    selected: index == selected,
                    move_count,
                    delete_confirmation,
                },
                cx.listener(move |this, hovered, _window, cx| {
                    if *hovered {
                        this.select_result(index, cx);
                    }
                }),
                cx.listener(move |this, event, _window, cx| {
                    this.on_result_click(index, event, cx);
                }),
                cx.listener(move |this, _event, _window, cx| {
                    if let Some(id) = clipboard_id {
                        this.request_delete_clipboard(id, cx);
                    }
                }),
            ));
        }

        let body = if clipboard_mode && self.clipboard_history.is_empty() {
            ui::empty_state("Clipboard history is empty.")
        } else if clipboard_mode && rows.is_empty() {
            ui::empty_state("No clipboard matches.")
        } else if self.loading {
            ui::empty_state("Finding applications…")
        } else if rows.is_empty() && self.files_loading {
            ui::empty_state("Searching files…")
        } else if rows.is_empty() {
            ui::empty_state("No results found.")
        } else {
            ui::results_list(rows)
        };

        div()
            .id("spotlight")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::select_all_input))
            .on_action(cx.listener(Self::open_clipboard_history))
            .on_action(cx.listener(Self::delete_clipboard_entry))
            .on_action(cx.listener(Self::clear_clipboard_history))
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
                    .when(clipboard_mode, |panel| {
                        panel.child(ui::clipboard_toolbar(
                            self.clipboard_history.len(),
                            self.pending_clipboard_action == Some(PendingClipboardAction::Clear),
                            cx.listener(|this, _event, _window, cx| {
                                this.request_clear_clipboard(cx);
                            }),
                        ))
                    })
                    .child(body.on_scroll_wheel(cx.listener(Self::on_results_scroll)))
                    .child(ui::footer(
                        self.message.as_deref().or(self.shortcut_error.as_deref()),
                        &self.shortcut_label,
                        clipboard_mode,
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
            move_selection_down(
                &mut selected,
                &mut visible_start,
                12,
                theme::MAX_VISIBLE_RESULTS,
            );
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
            move_selection_down(
                &mut selected,
                &mut visible_start,
                12,
                theme::MAX_VISIBLE_RESULTS,
            );
        }
        for _ in 0..7 {
            move_selection_up(&mut selected, &mut visible_start);
        }
        assert_eq!(selected, 2);
        assert_eq!(visible_start, 2);
    }

    #[test]
    fn mouse_scroll_moves_window_and_keeps_selection_visible() {
        let mut selected = 0;
        let mut visible_start = 0;

        assert!(scroll_results(
            &mut selected,
            &mut visible_start,
            20,
            theme::MAX_VISIBLE_RESULTS,
            true,
            3,
        ));
        assert_eq!(visible_start, 3);
        assert_eq!(selected, 3);

        assert!(scroll_results(
            &mut selected,
            &mut visible_start,
            20,
            theme::MAX_VISIBLE_RESULTS,
            false,
            2,
        ));
        assert_eq!(visible_start, 1);
        assert_eq!(selected, 3);
    }

    #[test]
    fn mouse_scroll_clamps_at_result_boundaries() {
        let mut selected = 0;
        let mut visible_start = 0;

        assert!(scroll_results(
            &mut selected,
            &mut visible_start,
            10,
            theme::MAX_VISIBLE_RESULTS,
            true,
            99,
        ));
        assert_eq!(visible_start, 3);
        assert_eq!(selected, 3);
        assert!(!scroll_results(
            &mut selected,
            &mut visible_start,
            10,
            theme::MAX_VISIBLE_RESULTS,
            true,
            1,
        ));
    }

    #[test]
    fn single_group_uses_full_result_capacity() {
        let results: Vec<_> = (0..theme::MAX_VISIBLE_RESULTS)
            .map(|index| SearchResult::Calculation {
                expression: index.to_string(),
                result: index.to_string(),
            })
            .collect();

        assert_eq!(
            visible_result_limit(&results, 0),
            theme::MAX_VISIBLE_RESULTS
        );
    }

    #[test]
    fn mixed_groups_reserve_room_for_the_second_header() {
        let results = vec![
            SearchResult::Calculation {
                expression: "1 + 1".to_string(),
                result: "2".to_string(),
            },
            SearchResult::Application(AppEntry {
                name: "Calculator".to_string(),
                path: "/Applications/Calculator.app".to_string(),
                icon_path: None,
                icon: None,
            }),
        ];

        assert_eq!(
            visible_result_limit(&results, 0),
            theme::MULTI_GROUP_VISIBLE_RESULTS
        );
    }

    #[test]
    fn delete_word_handles_unicode() {
        let mut query = "Open 日本語".to_string();
        delete_word(&mut query);
        assert_eq!(query, "Open ");
    }

    #[test]
    fn deleting_a_selection_clears_it_in_one_step() {
        let mut query = "Open 日本語".to_string();
        let mut selected_range = 0..query.len();

        assert!(delete_selected_text(&mut query, &mut selected_range));
        assert!(query.is_empty());
        assert_eq!(selected_range, 0..0);
        assert!(!delete_selected_text(&mut query, &mut selected_range));
    }
}
