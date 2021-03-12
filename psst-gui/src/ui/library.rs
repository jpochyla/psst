use crate::{
    data::{Ctx, Library, State},
    ui::{
        album::album_widget,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, spinner_widget},
    },
    widget::Async,
};
use druid::{
    widget::{CrossAxisAlignment, Flex, List},
    LensExt, Widget, WidgetExt,
};

pub fn detail_widget() -> impl Widget<State> {
    Flex::row()
        .must_fill_main_axis(true)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_flex_child(saved_albums_widget(), 1.0)
        .with_flex_child(saved_tracks_widget(), 1.0)
}

fn saved_albums_widget() -> impl Widget<State> {
    Async::new(
        || spinner_widget(),
        || List::new(album_widget),
        || error_widget().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            State::common_ctx,
            State::library.then(Library::saved_albums.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
}

fn saved_tracks_widget() -> impl Widget<State> {
    Async::new(
        || spinner_widget(),
        || {
            tracklist_widget(TrackDisplay {
                title: true,
                artist: true,
                album: true,
                ..TrackDisplay::empty()
            })
        },
        || error_widget().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            State::common_ctx,
            State::library.then(Library::saved_tracks.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
}
