use crate::{error::Error, ui::theme, widget::icons};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, SizedBox, Spinner},
    Data, Widget, WidgetExt,
};

pub fn make_placeholder<T: Data>() -> impl Widget<T> {
    SizedBox::empty().background(theme::BACKGROUND_DARK)
}

pub fn make_loader<T: Data>() -> impl Widget<T> {
    Spinner::new()
        .with_color(theme::GREY_4)
        .padding((0.0, theme::grid(6.0)))
        .center()
}

pub fn make_error() -> impl Widget<Error> {
    let icon = icons::SAD_FACE
        .scale((theme::grid(3.0), theme::grid(3.0)))
        .with_color(theme::PLACEHOLDER_COLOR);
    let error = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::new("Error:")
                .with_font(theme::UI_FONT_MEDIUM)
                .with_text_color(theme::PLACEHOLDER_COLOR),
        )
        .with_child(
            Label::dynamic(|err: &Error, _| err.to_string())
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .with_text_color(theme::PLACEHOLDER_COLOR),
        );
    Flex::row()
        .with_child(icon)
        .with_default_spacer()
        .with_child(error)
        .padding((0.0, theme::grid(6.0)))
        .center()
}
