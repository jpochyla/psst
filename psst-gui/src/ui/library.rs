use std::sync::Arc;

use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::{
    data::{Album, AlbumLink, AppState, Ctx, Library, SavedAlbums, SavedTracks, Track, TrackId},
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{
    album::album_widget,
    track::{tracklist_widget, TrackDisplay},
    utils::{error_widget, spinner_widget},
};

pub const LOAD_TRACKS: Selector = Selector::new("app.library.load-tracks");
pub const LOAD_ALBUMS: Selector = Selector::new("app.library.load-albums");

pub const SAVE_TRACK: Selector<Arc<Track>> = Selector::new("app.library.save-track");
pub const UNSAVE_TRACK: Selector<TrackId> = Selector::new("app.library.unsave-track");

pub const SAVE_ALBUM: Selector<Arc<Album>> = Selector::new("app.library.save-album");
pub const UNSAVE_ALBUM: Selector<AlbumLink> = Selector::new("app.library.unsave-album");

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
        error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::library.then(Library::saved_tracks.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_TRACKS,
        |_| WebApi::global().get_saved_tracks().map(SavedTracks::new),
        |_, data, _| {
            data.with_library_mut(|library| {
                library.saved_tracks.defer_default();
            });
        },
        |_, data, r| {
            data.with_library_mut(|library| {
                library.saved_tracks.update(r);
            });
        },
    )
    .on_command_async(
        SAVE_TRACK,
        |t| WebApi::global().save_track(&t.id.to_base62()),
        |_, data, t| {
            data.with_library_mut(|library| {
                library.add_track(t);
            });
        },
        |_, _, _| {
            // TODO: Handle failure.
        },
    )
    .on_command_async(
        UNSAVE_TRACK,
        |i| WebApi::global().unsave_track(&i.to_base62()),
        |_, data, i| {
            data.with_library_mut(|library| {
                library.remove_track(&i);
            });
        },
        |_, _, _| {
            // TODO: Handle failure.
        },
    )
}

pub fn saved_albums_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || List::new(album_widget).lens(Ctx::map(SavedAlbums::albums)),
        error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::library.then(Library::saved_albums.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_ALBUMS,
        |_| WebApi::global().get_saved_albums().map(SavedAlbums::new),
        |_, data, _| {
            data.with_library_mut(|library| {
                library.saved_albums.defer_default();
            });
        },
        |_, data, r| {
            data.with_library_mut(|library| {
                library.saved_albums.update(r);
            });
        },
    )
    .on_command_async(
        SAVE_ALBUM,
        |a| WebApi::global().save_album(&a.id),
        |_, data, a| {
            data.with_library_mut(move |library| {
                library.add_album(a);
            });
        },
        |_, _, _| {
            // TODO: Handle failure.
        },
    )
    .on_command_async(
        UNSAVE_ALBUM,
        |l| WebApi::global().unsave_album(&l.id),
        |_, data, l| {
            data.with_library_mut(|library| {
                library.remove_album(&l.id);
            });
        },
        |_, _, _| {
            // TODO: Handle failure.
        },
    )
}
