use gpui::{div, img, prelude::*, px, Div};

use crate::apps::AppEntry;
use crate::theme::{self, Theme};

pub fn result_row(app: &AppEntry, selected: bool) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_1p5()
        .rounded_lg()
        .text_size(px(15.))
        .when(selected, |el| {
            el.bg(Theme::SELECTED_BG)
                .text_color(Theme::SELECTED_TEXT)
        })
        .when(!selected, |el| el.text_color(Theme::RESULT_TEXT))
        .child(icon_element(app))
        .child(app.name.clone())
}

fn icon_element(app: &AppEntry) -> impl IntoElement {
    match &app.icon {
        Some(icon) => div()
            .size(px(theme::ICON_SIZE))
            .flex_shrink_0()
            .child(
                img(icon.clone())
                    .size(px(theme::ICON_SIZE))
                    .rounded(px(7.)),
            ),
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
