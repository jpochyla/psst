use std::sync::Arc;

use druid::im::Vector;
use druid::widget::{Flex, Label, Scroll};
use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::data::{Ctx, HomeDetail, Track, WithCtx};
use crate::{
    data::AppState,
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{artist, playable, theme, track};
use super::{
    playlist,
    utils::{error_widget, spinner_widget},
};

pub const LOAD_MADE_FOR_YOU: Selector = Selector::new("app.home.load-made-for-your");

pub fn home_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Label::new("Made for you").with_text_size(theme::grid(2.5)).align_left().padding((theme::grid(1.5), 0.0)))
        .with_default_spacer()
        .with_child(made_for_you_widget())
        .with_default_spacer()
        .with_child(Label::new("Your top artists").with_text_size(theme::grid(2.5)).align_left().padding((theme::grid(1.5), 0.0)))
        .with_default_spacer()
        .with_child(user_top_artists_widget())
        .with_default_spacer()
        .with_child(Label::new("Your top tracks").with_text_size(theme::grid(2.5)).align_left().padding((theme::grid(1.5), 0.0)))
        .with_default_spacer()
        .with_child(user_top_tracks_widget())
}

fn made_for_you_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || Scroll::new(
            List::new(
                    || playlist::horizontal_playlist_widget(false, true)
                ).horizontal()
            ).horizontal(),
                // TODO Add a function which allows people to scroll with their scroll wheel!!!
        error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::home_detail.then(HomeDetail::made_for_you),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_MADE_FOR_YOU,
        |_| WebApi::global().get_made_for_you(),
        |_, data, d| data.home_detail.made_for_you.defer(d),
        |_, data, r| data.home_detail.made_for_you.update(r),
    )
}

fn user_top_artists_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || Scroll::new(
            List::new(
                    || artist::horizontal_recommended_artist_widget()
                ).horizontal()
                // TODO Add a function which allows people to scroll with their scroll wheel!!!
            ).horizontal(),
        error_widget,
    )
    .lens(
        AppState::home_detail.then(HomeDetail::user_top_artists)
    )
    .on_command_async(
        LOAD_MADE_FOR_YOU,
        |_| WebApi::global().get_user_top_artist(),
        |_, data, d| data.home_detail.user_top_artists.defer(d),
        |_, data, r| data.home_detail.user_top_artists.update(r),
    )

}

fn top_tracks_widget() -> impl Widget<WithCtx<Vector<Arc<Track>>>> {
    playable::list_widget(playable::Display {
        track: track::Display {
            title: true,
            album: true,
            popularity: true,
            cover: true,
            ..track::Display::empty()
        },
    })
}

fn user_top_tracks_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || top_tracks_widget(),
        error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::home_detail.then(HomeDetail::user_top_tracks),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_MADE_FOR_YOU,
        |_| WebApi::global().get_user_top_tracks(),
        |_, data, d| data.home_detail.user_top_tracks.defer(d),
        |_, data, r| data.home_detail.user_top_tracks.update(r),
    )
}