use druid::{
    commands,
    widget::{Either, Flex, Label},
    Widget, WidgetExt,
};

use crate::{
    data::{State, UserProfile},
    ui::theme,
    webapi::WebApi,
    widget::{Async, AsyncAction, Empty, LinkExt},
};

pub fn user_widget() -> impl Widget<State> {
    let is_connected = Either::new(
        // TODO: Avoid the locking here.
        |state: &State, _| state.session.is_connected(),
        Label::new("Connected")
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL),
        Label::new("Disconnected")
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL),
    );

    let user_profile = Async::new(
        || Empty,
        || {
            Label::raw()
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(UserProfile::display_name)
        },
        || Empty,
    )
    .controller(AsyncAction::new(|_| WebApi::global().get_user_profile()))
    .lens(State::user_profile);

    Flex::column()
        .with_child(is_connected)
        .with_default_spacer()
        .with_child(user_profile)
        .padding((theme::grid(2.0), theme::grid(1.5)))
        .expand_width()
        .link()
        .on_click(|ctx, _, _| {
            ctx.submit_command(commands::SHOW_PREFERENCES);
        })
}
