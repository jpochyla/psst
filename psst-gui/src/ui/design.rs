//! Shared native surfaces and actions, following the selected light/dark theme.
use druid::{widget::Container, Color, Data, Widget, WidgetExt};

use super::theme;

pub fn card<T: Data>(child: impl Widget<T> + 'static) -> impl Widget<T> {
    Container::new(child.padding(24.0))
        .background(theme::GREY_700)
        .border(theme::GREY_500, 1.0)
        .rounded(14.0)
}

pub fn primary<T: Data>(child: impl Widget<T> + 'static) -> impl Widget<T> {
    child.env_scope(|env, _| {
        env.set(theme::BUTTON_LIGHT, Color::rgb8(43, 211, 123));
        env.set(theme::BUTTON_DARK, Color::rgb8(32, 187, 106));
        env.set(theme::TEXT_COLOR, Color::rgb8(13, 36, 24));
        env.set(theme::BORDER_LIGHT, Color::rgb8(32, 187, 106));
        env.set(theme::BORDER_DARK, Color::rgb8(32, 187, 106));
    })
}
