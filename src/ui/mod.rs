pub mod result_row;

use gpui::{AnyElement, Div, div, prelude::*, px};

use crate::theme::Theme;

pub fn input_field(content: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .px_4()
        .pt_3p5()
        .pb_3()
        .text_size(px(20.))
        .text_color(Theme::INPUT_TEXT)
        .child(content)
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

pub fn section_header(label: &str) -> Div {
    div()
        .h(px(18.))
        .flex()
        .items_center()
        .px_3()
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(Theme::FOOTER_TEXT)
        .child(label.to_string())
}

pub fn empty_state(message: &str) -> Div {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .text_color(Theme::PLACEHOLDER)
        .child(message.to_string())
}

pub fn footer(message: Option<&str>, shortcut: &str) -> Div {
    let footer = div()
        .mt_auto()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(Theme::BORDER)
        .text_size(px(12.))
        .text_color(Theme::FOOTER_TEXT);

    footer
        .when_some(message, |footer, message| {
            footer.child(
                div()
                    .flex_1()
                    .text_color(gpui::rgb(0xff7b72))
                    .child(message.to_string()),
            )
        })
        .when(message.is_none(), |footer| footer.child(div().flex_1()))
        .child(hint(shortcut, "Toggle"))
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
