use std::sync::Arc;

use druid::im::Vector;
use druid::widget::{Button, Either, Flex, Label, LineBreaking, Scroll};
use druid::Insets;
use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::data::{Artist, Ctx, HomeDetail, MixedView, NowPlaying, Show, Track, TrackLines, WithCtx};
use crate::widget::Empty;
use crate::{
    data::AppState,
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{album, artist, playable, show, theme, track, utils};
use super::{
    playlist,
    utils::{error_widget, spinner_widget},
};

pub const LOAD_LYRICS: Selector<NowPlaying> = Selector::new("app.home.load_lyrics");

pub fn lyrics_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(user_top_tracks_widget())
}

fn user_top_tracks_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || {List::new(|| {
            Label::raw()
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(Ctx::data().then(TrackLines::words))
                .expand_width()
                .padding(Insets::uniform_xy(theme::grid(2.0), theme::grid(0.6)))
                .link()
        })},
        error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::lyrics,
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_LYRICS,
        |t| WebApi::global().get_lyrics(t.item.id().to_base62(), t.cover_image_url(250.0, 250.0).unwrap().to_string()),
        |_, data, _| data.lyrics.defer(()),
        |_, data, r| data.lyrics.update(((), r.1)),
    )
}