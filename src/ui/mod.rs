pub mod result_row;

use std::time::Duration;

use gpui::{div, prelude::*, px, Animation, AnimationExt, AnyElement, Div};

use crate::theme::Theme;

pub fn input_field(query: &str) -> Div {
    div()
        .flex()
        .items_center()
        .px_4()
        .pt_3p5()
        .pb_3()
        .text_size(px(20.))
        .text_color(Theme::INPUT_TEXT)
        .when(query.is_empty(), |el| {
            el.child(div().text_color(Theme::PLACEHOLDER).child("Search applications…"))
        })
        .when(!query.is_empty(), |el| {
            el.child(query.to_string()).child(caret())
        })
}

pub fn results_list(children: Vec<AnyElement>) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .gap_y_0p5()
        .px_2()
        .py_2()
        .children(children)
}

pub fn empty_state() -> Div {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .text_color(Theme::PLACEHOLDER)
        .child("No results")
}

pub fn footer() -> Div {
    div()
        .mt_auto()
        .flex()
        .items_center()
        .justify_end()
        .gap_3()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(Theme::BORDER)
        .text_size(px(12.))
        .text_color(Theme::FOOTER_TEXT)
        .child(hint("↑↓", "Navigate"))
        .child(hint("↵", "Open"))
        .child(hint("esc", "Close"))
}

fn hint(key: &str, label: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(Theme::BORDER)
                .bg(rgb_footer_key())
                .px_1p5()
                .py_0p5()
                .child(key.to_string()),
        )
        .child(label.to_string())
}

fn rgb_footer_key() -> gpui::Rgba {
    gpui::rgb(0x22242b)
}

fn caret() -> impl IntoElement {
    div()
        .w(px(2.))
        .h(px(24.))
        .ml_0p5()
        .rounded_full()
        .bg(Theme::CARET)
        .with_animation(
            "caret-blink",
            Animation::new(Duration::from_millis(1100)).repeat(),
            |el, delta| el.opacity(if delta < 0.5 { 1.0 } else { 0.15 }),
        )
}
