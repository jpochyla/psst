use druid::{widget::List, LensExt, Widget, WidgetExt};

use crate::{
    data::{AppState, Personalized},
    ui::{
        playlist,
        utils::{error_widget, spinner_widget},
    },
    webapi::WebApi,
    widget::Async,
};

pub fn home_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || List::new(playlist::playlist_widget),
        error_widget,
    )
    .on_deferred(|_| WebApi::global().get_made_for_you())
    .lens(AppState::personalized.then(Personalized::made_for_you))
}
