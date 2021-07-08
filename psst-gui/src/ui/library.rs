use crate::{
    data::{AppState, Ctx, Library, SavedAlbums},
    ui::{
        album::album_widget,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, spinner_widget},
    },
    widget::Async,
};
use druid::{widget::List, LensExt, Widget, WidgetExt};

pub fn saved_tracks_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
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
            AppState::common_ctx,
            AppState::library.then(Library::saved_tracks.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
}

pub fn saved_albums_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || List::new(album_widget).lens(Ctx::map(SavedAlbums::albums)),
        || error_widget().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::library.then(Library::saved_albums.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
}
