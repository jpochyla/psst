use std::sync::Arc;

use druid::{widget::List, LensExt, Widget, WidgetExt};

use crate::{
    data::{AppState, CommonCtx, Ctx, Library, SavedAlbums, SavedTracks},
    ui::{
        album::album_widget,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, spinner_widget},
    },
    webapi::WebApi,
    widget::Async,
};

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
    .on_deferred(|c: &Ctx<Arc<CommonCtx>, ()>| {
        WebApi::global()
            .get_saved_tracks()
            .map(SavedTracks::new)
            .map(|tracks| c.replace(tracks))
            .map_err(|err| c.replace(err))
    })
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
    .on_deferred(|c: &Ctx<Arc<CommonCtx>, ()>| {
        WebApi::global()
            .get_saved_albums()
            .map(SavedAlbums::new)
            .map(|albums| c.replace(albums))
            .map_err(|err| c.replace(err))
    })
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::library.then(Library::saved_albums.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
}
