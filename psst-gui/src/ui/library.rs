use crate::{
    data::{Library, State},
    ui::{
        album::make_album,
        track::{make_tracklist, TrackDisplay},
    },
    widgets::Maybe,
};
use druid::{
    widget::{CrossAxisAlignment, Flex, List},
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
    Maybe::or_empty(|| List::new(make_album)).lens(Library::saved_albums)
}

pub fn make_saved_tracks() -> impl Widget<Library> {
    Maybe::or_empty(|| {
        make_tracklist(TrackDisplay {
            title: true,
            artist: true,
            album: true,
        })
    })
    .lens(Library::saved_tracks)
}
