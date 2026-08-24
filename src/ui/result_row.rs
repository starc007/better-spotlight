use gpui::{Animation, AnimationExt, AnyElement, Div, Rgba, div, img, prelude::*, px};

use crate::apps::AppEntry;
use crate::theme::{self, Theme};

pub fn result_row(app: &AppEntry, selected: bool, move_count: u64) -> AnyElement {
    let row = div()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_1p5()
        .rounded_lg()
        .text_size(px(15.))
        .when(selected, |el| el.text_color(Theme::SELECTED_TEXT))
        .when(!selected, |el| el.text_color(Theme::RESULT_TEXT))
        .child(icon_element(app))
        .child(app.name.clone());

    if selected {
        row.with_animation(
            ("row-select", move_count as usize),
            Animation::new(std::time::Duration::from_millis(140)),
            |row: Div, delta| {
                let ease = 1.0 - (1.0 - delta) * (1.0 - delta);
                row.bg(fade_in(Theme::SELECTED_BG, ease))
            },
        )
        .into_any_element()
    } else {
        row.into_any_element()
    }
}

/// Returns `color` with its alpha channel set to `alpha`.
fn fade_in(color: Rgba, alpha: f32) -> Rgba {
    Rgba { a: alpha, ..color }
}

fn icon_element(app: &AppEntry) -> impl IntoElement {
    match &app.icon {
        Some(icon) => div()
            .size(px(theme::ICON_SIZE))
            .flex_shrink_0()
            .child(img(icon.clone()).size(px(theme::ICON_SIZE)).rounded(px(7.))),
        None => fallback_tile(&app.name),
    }
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
