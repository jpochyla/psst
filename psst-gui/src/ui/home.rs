use druid::{widget::List, LensExt, Widget, WidgetExt};

use crate::{
    data::{Personalized, AppState},
    ui::playlist,
    ui::utils::{error_widget, spinner_widget},
    webapi::WebApi,
    widget::{Async, AsyncAction},
};

pub fn home_widget() -> impl Widget<AppState> {
    Async::new(
        || spinner_widget(),
        || List::new(|| playlist::playlist_widget()),
        || error_widget(),
    )
    .controller(AsyncAction::new(|_| WebApi::global().get_made_for_you()))
    .lens(AppState::personalized.then(Personalized::made_for_you))
}
