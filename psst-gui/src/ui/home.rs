use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::data::Ctx;
use crate::{
    data::{AppState, Personalized},
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{
    playlist,
    utils::{error_widget, spinner_widget},
};

pub const LOAD_MADE_FOR_YOU: Selector = Selector::new("app.home.load-made-for-your");

pub fn home_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || List::new(playlist::playlist_widget),
        error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::personalized.then(Personalized::made_for_you),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_MADE_FOR_YOU,
        |_| WebApi::global().get_made_for_you(),
        |_, data, d| data.personalized.made_for_you.defer(d),
        |_, data, r| data.personalized.made_for_you.update(r),
    )
}
