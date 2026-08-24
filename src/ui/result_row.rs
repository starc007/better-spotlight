use gpui::{
    Animation, AnimationExt, AnyElement, App, ClickEvent, Div, Rgba, Window, div, img, prelude::*,
    px,
};

use crate::apps::AppEntry;
use crate::results::SearchResult;
use crate::theme::{self, Theme};

pub fn result_row(
    result: &SearchResult,
    index: usize,
    selected: bool,
    move_count: u64,
    on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let row = div()
        .id(("result-row", index))
        .flex()
        .items_center()
        .gap_3()
        .h(px(44.))
        .px_3()
        .py_1p5()
        .rounded_lg()
        .cursor_pointer()
        .active(|style| style.opacity(0.82))
        .on_hover(on_hover)
        .on_click(on_click)
        .text_size(px(15.))
        .when(selected, |el| el.text_color(Theme::SELECTED_TEXT))
        .when(!selected, |el| el.text_color(Theme::RESULT_TEXT))
        .child(result_icon(result))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .line_height(px(17.))
                        .child(result.name().to_string()),
                )
                .when_some(result.subtitle(), |text, subtitle| {
                    text.child(
                        div()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(11.))
                            .line_height(px(13.))
                            .text_color(if selected {
                                Theme::SELECTED_TEXT
                            } else {
                                Theme::RESULT_META
                            })
                            .child(subtitle.to_string()),
                    )
                }),
        );

    if selected {
        row.with_animation(
            ("row-select", move_count as usize),
            Animation::new(std::time::Duration::from_millis(140)),
            |row, delta| {
                let ease = 1.0 - (1.0 - delta) * (1.0 - delta);
                row.bg(fade_in(Theme::SELECTED_BG, ease))
            },
        )
        .into_any_element()
    } else {
        row.into_any_element()
    }
}

fn result_icon(result: &SearchResult) -> Div {
    match result {
        SearchResult::Application(app) => icon_element(app),
        SearchResult::File(file) => file_tile(&file.name),
    }
}

/// Returns `color` with its alpha channel set to `alpha`.
fn fade_in(color: Rgba, alpha: f32) -> Rgba {
    Rgba { a: alpha, ..color }
}

fn icon_element(app: &AppEntry) -> Div {
    match &app.icon {
        Some(icon) => div()
            .size(px(theme::ICON_SIZE))
            .flex_shrink_0()
            .child(img(icon.clone()).size(px(theme::ICON_SIZE)).rounded(px(7.))),
        None => fallback_tile(&app.name),
    }
}

fn file_tile(name: &str) -> Div {
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.chars().take(3).collect::<String>().to_uppercase())
        .filter(|extension| !extension.is_empty())
        .unwrap_or_else(|| "FILE".to_string());

    div()
        .flex_shrink_0()
        .size(px(theme::ICON_SIZE))
        .rounded(px(7.))
        .border_1()
        .border_color(Theme::BORDER)
        .bg(gpui::rgb(0x22242b))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(if extension.len() > 3 { 8. } else { 9. }))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(Theme::RESULT_META)
        .child(extension)
}

/// Letter tile shown when an app bundle has no decodable icon.
fn fallback_tile(name: &str) -> Div {
    let initial = name.chars().next().unwrap_or('?').to_uppercase();
    let color = theme::icon_fallback_color(name);

    div()
        .flex_shrink_0()
        .size(px(theme::ICON_SIZE))
        .rounded(px(7.))
        .bg(color)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(16.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(gpui::rgb(0xffffff))
        .child(initial.to_string())
}
