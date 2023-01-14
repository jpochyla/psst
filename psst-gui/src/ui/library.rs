use std::sync::Arc;

use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::{
    cmd,
    data::{
        Album, AlbumLink, AppState, Ctx, Library, SavedAlbums, SavedShows, SavedTracks, Show,
        ShowLink, Track, TrackId,
    },
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{album, playable, show, track, utils};

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
    .on_command_async(
        SAVE_TRACK,
        |t| WebApi::global().save_track(&t.id.0.to_base62()),
        |_, data, t| {
            data.with_library_mut(|library| {
                library.add_track(t);
            });
        },
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Track added to library.")
            }
        },
    )
    .on_command_async(
        UNSAVE_TRACK,
        |i| WebApi::global().unsave_track(&i.0.to_base62()),
        |_, data, i| {
            data.with_library_mut(|library| {
                library.remove_track(&i);
            });
        },
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Track removed from library.")
            }
        },
    )
}

pub fn saved_albums_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || List::new(album::album_widget).lens(Ctx::map(SavedAlbums::albums)),
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
    .on_command_async(
        SAVE_ALBUM,
        |a| WebApi::global().save_album(&a.id),
        |_, data, a| {
            data.with_library_mut(move |library| {
                library.add_album(a);
            });
        },
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Album added to library.");
            }
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
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Album removed from library.");
            }
        },
    )
}

pub fn saved_shows_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || List::new(show::show_widget).lens(Ctx::map(SavedShows::shows)),
        utils::error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::library.then(Library::saved_shows.in_arc()),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_SHOWS,
        |_| WebApi::global().get_saved_shows().map(SavedShows::new),
        |_, data, _| {
            data.with_library_mut(|library| {
                library.saved_shows.defer_default();
            });
        },
        |_, data, r| {
            data.with_library_mut(|library| {
                library.saved_shows.update(r);
            });
        },
    )
    .on_command_async(
        SAVE_SHOW,
        |a| WebApi::global().save_show(&a.id),
        |_, data, s| {
            data.with_library_mut(move |library| {
                library.add_show(s);
            });
        },
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Show added to library.");
            }
        },
    )
    .on_command_async(
        UNSAVE_SHOW,
        |l| WebApi::global().unsave_show(&l.id),
        |_, data, l| {
            data.with_library_mut(|library| {
                library.remove_show(&l.id);
            });
        },
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Show removed from library.");
            }
        },
    )
}
