use crate::{
    data::{Library, State},
    ui::{
        album::make_album,
        track::{make_tracklist, TrackDisplay},
    },
    widgets::Promised,
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, List},
    Widget, WidgetExt,
};

pub fn make_detail() -> impl Widget<State> {
    Flex::row()
        .must_fill_main_axis(true)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_flex_child(make_saved_albums(), 1.0)
        .with_flex_child(make_saved_tracks(), 1.0)
        .lens(State::library)
}

pub fn make_saved_albums() -> impl Widget<Library> {
    Promised::new(
        || Label::new("Loading"),
        || List::new(make_album),
        || Label::new("Error"),
    )
    .lens(Library::saved_albums)
}

pub fn make_saved_tracks() -> impl Widget<Library> {
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
    .lens(Library::saved_tracks)
}
