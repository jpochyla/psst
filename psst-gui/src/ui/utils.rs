use druid::{theme, widget::SizedBox, Data, Widget, WidgetExt};

pub fn make_placeholder<T: Data>() -> impl Widget<T> {
    SizedBox::empty().background(theme::BACKGROUND_DARK)
}
