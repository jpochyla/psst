use std::sync::Arc;

use druid::{widget::Flex, LensExt, Selector, Widget, WidgetExt};

use crate::{
    cmd,
    data::{
        Album, AlbumLink, AppState, Ctx, Library, SavedAlbums, SavedTracks, Show, ShowLink, Track,
        TrackId,
    },
    ui::home::{shows_that_you_might_like, your_shows},
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{album, playable, track, utils};

pub const LOAD_TRACKS: Selector = Selector::new("app.library.load-tracks");
pub const LOAD_ALBUMS: Selector = Selector::new("app.library.load-albums");
pub const LOAD_SHOWS: Selector = Selector::new("app.library.load-shows");

pub const SAVE_TRACK: Selector<Arc<Track>> = Selector::new("app.library.save-track");
pub const UNSAVE_TRACK: Selector<TrackId> = Selector::new("app.library.unsave-track");

pub const SAVE_ALBUM: Selector<Arc<Album>> = Selector::new("app.library.save-album");
pub const UNSAVE_ALBUM: Selector<AlbumLink> = Selector::new("app.library.unsave-album");

pub const SAVE_SHOW: Selector<Arc<Show>> = Selector::new("app.library.save-show");
pub const UNSAVE_SHOW: Selector<ShowLink> = Selector::new("app.library.unsave-show");

pub fn saved_tracks_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || {
            playable::list_widget_with_find(
                playable::Display {
                    track: track::Display {
                        title: true,
                        artist: true,
                        album: true,
                        cover: true,
                        ..track::Display::empty()
                    },
                },
                cmd::FIND_IN_SAVED_TRACKS,
            )
        },
        utils::error_widget,
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
}

pub fn saved_albums_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || {
            super::grid::with_context(|| album::album_widget(true).boxed())
                .lens(Ctx::map(SavedAlbums::albums))
        },
        utils::error_widget,
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
}

pub fn saved_shows_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(your_shows())
        .with_child(shows_that_you_might_like())
}

// These commands must be available from every browsing/playback route.
pub fn mutation_controller(widget: impl Widget<AppState> + 'static) -> impl Widget<AppState> {
    widget
        .on_command_async(
            SAVE_TRACK,
            |track| WebApi::global().save_track(&track.id.0.to_base62()),
            |_, _, _| {},
            |_, data, (track, result)| match result {
                Ok(()) => {
                    data.with_library_mut(|library| library.add_track(track));
                    data.info_alert("Track added to library.");
                }
                Err(error) => data.error_alert(error),
            },
        )
        .on_command_async(
            UNSAVE_TRACK,
            |id| WebApi::global().unsave_track(&id.0.to_base62()),
            |_, _, _| {},
            |_, data, (id, result)| match result {
                Ok(()) => {
                    data.with_library_mut(|library| library.remove_track(&id));
                    data.info_alert("Track removed from library.");
                }
                Err(error) => data.error_alert(error),
            },
        )
        .on_command_async(
            SAVE_ALBUM,
            |album| WebApi::global().save_album(&album.id),
            |_, _, _| {},
            |_, data, (album, result)| match result {
                Ok(()) => {
                    data.with_library_mut(|library| library.add_album(album));
                    data.info_alert("Album added to library.");
                }
                Err(error) => data.error_alert(error),
            },
        )
        .on_command_async(
            UNSAVE_ALBUM,
            |album| WebApi::global().unsave_album(&album.id),
            |_, _, _| {},
            |_, data, (album, result)| match result {
                Ok(()) => {
                    data.with_library_mut(|library| library.remove_album(&album.id));
                    data.info_alert("Album removed from library.");
                }
                Err(error) => data.error_alert(error),
            },
        )
        .on_command_async(
            SAVE_SHOW,
            |show| WebApi::global().save_show(&show.id),
            |_, _, _| {},
            |_, data, (show, result)| match result {
                Ok(()) => {
                    data.with_library_mut(|library| library.add_show(show));
                    data.info_alert("Show added to library.");
                }
                Err(error) => data.error_alert(error),
            },
        )
        .on_command_async(
            UNSAVE_SHOW,
            |show| WebApi::global().unsave_show(&show.id),
            |_, _, _| {},
            |_, data, (show, result)| match result {
                Ok(()) => {
                    data.with_library_mut(|library| library.remove_show(&show.id));
                    data.info_alert("Show removed from library.");
                }
                Err(error) => data.error_alert(error),
            },
        )
}
