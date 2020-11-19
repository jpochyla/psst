use crate::{
    data::{Ctx, Library, State},
    ui::{
        album::make_album,
        track::{make_tracklist, TrackDisplay},
        utils::{make_error, make_loader},
    },
    widget::Promised,
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
    Promised::new(|| make_loader(), || List::new(make_album), || make_error())
        .lens(State::library.then(Library::saved_albums))
}

fn make_saved_tracks() -> impl Widget<State> {
    Promised::new(
        || make_loader(),
        || {
            make_tracklist(TrackDisplay {
                number: false,
                title: true,
                artist: true,
                album: true,
            })
        },
        || make_error(),
    )
    .lens(
        Ctx::make(State::track_ctx, State::library.then(Library::saved_tracks))
            .then(Ctx::in_promise()),
    )
}
