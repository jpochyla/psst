use std::sync::Arc;

use druid::im::Vector;
use druid::widget::{Either, Flex, Label, Scroll};
use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::data::{Artist, Ctx, HomeDetail, MixedView, Show, Track, TrackLines, WithCtx};
use crate::widget::Empty;
use crate::{
    data::AppState,
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{album, artist, playable, show, theme, track};
use super::{
    playlist,
    utils::{error_widget, spinner_widget},
};

pub const LOAD_LYRICS: Selector = Selector::new("app.home.load-made-for-your");

pub fn lyrics_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(user_top_tracks_widget())
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
    Label::new(format!("{:?}", WebApi::global().get_lyrics("0Xf3chg0IH3ivgHfSRDImY".to_string(),  "i.scdn.co%2Fimage%2Fab67616d0000b2730e60d57a785d7c4f8418c8a3".to_string()).unwrap()[0]))
}
