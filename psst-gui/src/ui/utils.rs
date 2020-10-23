use crate::ui::theme;
use druid::{
    widget::{Label, SizedBox, Spinner},
    Data, Widget, WidgetExt,
};

pub fn make_placeholder<T: Data>() -> impl Widget<T> {
    SizedBox::empty().background(theme::BACKGROUND_DARK)
}

pub fn make_loader<T: Data>() -> impl Widget<T> {
    Spinner::new().with_color(theme::GREY_4).center()
}

pub fn make_error<T: Data>() -> impl Widget<T> {
    Label::new("Error").center()
}
