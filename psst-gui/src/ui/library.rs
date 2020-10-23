use crate::{
    ctx::Ctx,
    data::{Library, State},
    ui::{
        album::make_album,
        track::{make_tracklist, TrackDisplay},
    },
    widgets::Promised,
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, List},
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
    Promised::new(
        || Label::new("Loading"),
        || List::new(make_album),
        || Label::new("Error"),
    )
    .lens(State::library.then(Library::saved_albums))
}

fn make_saved_tracks() -> impl Widget<State> {
    Promised::new(
        || Label::new("Loading"),
        || {
            make_tracklist(TrackDisplay {
                number: false,
                title: true,
                artist: true,
                album: true,
            })
        },
        || Label::new("Error"),
    )
    .lens(
        Ctx::make(
            State::track_context(),
            State::library.then(Library::saved_tracks),
        )
        .then(Ctx::in_promise()),
    )
}
