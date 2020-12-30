use crate::{error::Error, ui::theme, widget::icons};
use druid::{
    kurbo::Line,
    widget::{BackgroundBrush, CrossAxisAlignment, Flex, Label, Painter, SizedBox, Spinner},
    Data, RenderContext, Widget, WidgetExt,
};

pub enum Border {
    Top,
    Bottom,
}

impl Border {
    pub fn widget<T: Data>(self) -> impl Into<BackgroundBrush<T>> {
        Painter::new(move |ctx, _, _| {
            let h = 1.0;
            let y = match self {
                Self::Top => 0.0,
                Self::Bottom => ctx.size().height - h,
            };
            let line = Line::new((0.0, y), (ctx.size().width, y));
            ctx.stroke(line, &theme::GREY_6, h);
        })
    }
}

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
