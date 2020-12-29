use crate::{
    data::{Ctx, Library, State},
    ui::{
        album::make_album,
        track::{make_tracklist, TrackDisplay},
        utils::{make_error, make_loader},
    },
    widget::Async,
};
use druid::{
    widget::{CrossAxisAlignment, Flex, List},
    LensExt, Widget, WidgetExt,
};

pub fn make_detail() -> impl Widget<State> {
    Flex::row()
        .must_fill_main_axis(true)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_flex_child(make_saved_albums(), 1.0)
        .with_flex_child(make_saved_tracks(), 1.0)
}

fn make_saved_albums() -> impl Widget<State> {
    Async::new(
        || make_loader(),
        || List::new(make_album),
        || make_error().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            State::common_ctx,
            State::library.then(Library::saved_albums.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
}

fn make_saved_tracks() -> impl Widget<State> {
    Async::new(
        || make_loader(),
        || {
            make_tracklist(TrackDisplay {
                number: false,
                title: true,
                artist: true,
                album: true,
            })
        },
        || make_error().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            State::common_ctx,
            State::library.then(Library::saved_tracks.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
}
