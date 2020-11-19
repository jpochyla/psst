use crate::{ui::theme, widget::icons};
use druid::{
    widget::{Flex, Label, SizedBox, Spinner},
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

pub fn make_error<T: Data>() -> impl Widget<T> {
    Flex::row()
        .with_child(
            icons::ERROR
                .scale((theme::grid(2.0), theme::grid(2.0)))
                .with_color(theme::GREY_4),
        )
        .with_default_spacer()
        .with_child(Label::new("Failed to load.").with_text_color(theme::GREY_4))
        .padding((0.0, theme::grid(6.0)))
        .center()
}
