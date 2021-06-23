use crate::{
    data::{
        Album, AlbumLink, Artist, ArtistAlbums, ArtistLink, AudioAnalysis, Cached, Nav,
        PlaybackPayload, PlaylistLink, QueueBehavior, SearchResults, Track, TrackId,
    },
    error::Error,
};
use druid::{im::Vector, Selector, WidgetId};
use std::{sync::Arc, time::Duration};

// Widget IDs

pub const WIDGET_SEARCH_INPUT: WidgetId = WidgetId::reserved(1);

// Common

pub const SHOW_MAIN: Selector = Selector::new("app.show-main");
pub const SET_FOCUS: Selector = Selector::new("app.set-focus");
pub const COPY: Selector<String> = Selector::new("app.copy-to-clipboard");

// Session

pub const SESSION_CONNECT: Selector = Selector::new("app.session-connect");
pub const SESSION_CONNECTED: Selector = Selector::new("app.session-connected");
pub const SESSION_DISCONNECTED: Selector = Selector::new("app.session-disconnected");

// Navigation

pub const NAVIGATE: Selector<Nav> = Selector::new("app.navigates");
pub const NAVIGATE_BACK: Selector<usize> = Selector::new("app.navigate-back");

// Search

pub const LOAD_SEARCH_RESULTS: Selector<String> = Selector::new("app.load-search-results");
pub const UPDATE_SEARCH_RESULTS: Selector<Result<SearchResults, Error>> =
    Selector::new("app.update-search-results");

// Library

pub const LOAD_SAVED_TRACKS: Selector = Selector::new("app.load-saved-tracks");
pub const LOAD_SAVED_ALBUMS: Selector = Selector::new("app.load-saved-albums");
pub const UPDATE_SAVED_ALBUMS: Selector<Result<Vector<Arc<Album>>, Error>> =
    Selector::new("app.update-saved-albums");
pub const UPDATE_SAVED_TRACKS: Selector<Result<Vector<Arc<Track>>, Error>> =
    Selector::new("app.update-saved-tracks");
pub const SAVE_TRACK: Selector<Arc<Track>> = Selector::new("app.save-track");
pub const UNSAVE_TRACK: Selector<TrackId> = Selector::new("app.unsave-track");
pub const SAVE_ALBUM: Selector<Arc<Album>> = Selector::new("app.save-album");
pub const UNSAVE_ALBUM: Selector<AlbumLink> = Selector::new("app.unsave-album");

// Album detail

pub const LOAD_ALBUM_DETAIL: Selector<AlbumLink> = Selector::new("app.load-album-detail");
pub const UPDATE_ALBUM_DETAIL: Selector<(AlbumLink, Result<Cached<Arc<Album>>, Error>)> =
    Selector::new("app.update-album-detail");

// Artist detail

pub const LOAD_ARTIST_DETAIL: Selector<ArtistLink> = Selector::new("app.load-artist-detail");
pub const UPDATE_ARTIST_DETAIL: Selector<(ArtistLink, Result<Artist, Error>)> =
    Selector::new("app.update-artist-detail");
pub const UPDATE_ARTIST_ALBUMS: Selector<(ArtistLink, Result<ArtistAlbums, Error>)> =
    Selector::new("app.update-artist-album");
pub const UPDATE_ARTIST_TOP_TRACKS: Selector<(ArtistLink, Result<Vector<Arc<Track>>, Error>)> =
    Selector::new("app.update-artist-top_tracks");
pub const UPDATE_ARTIST_RELATED: Selector<(ArtistLink, Result<Cached<Vector<Artist>>, Error>)> =
    Selector::new("app.update-artist-related");

// Playlist detail

pub const LOAD_PLAYLIST_DETAIL: Selector<PlaylistLink> = Selector::new("app.load-playlist-detail");
pub const UPDATE_PLAYLIST_TRACKS: Selector<(PlaylistLink, Result<Vector<Arc<Track>>, Error>)> =
    Selector::new("app.update-playlist-tracks");

// Playback state

pub const PLAYBACK_LOADING: Selector<TrackId> = Selector::new("app.playback-loading");
pub const PLAYBACK_PLAYING: Selector<(TrackId, Duration)> = Selector::new("app.playback-playing");
pub const PLAYBACK_PROGRESS: Selector<Duration> = Selector::new("app.playback-progress");
pub const PLAYBACK_PAUSING: Selector = Selector::new("app.playback-pausing");
pub const PLAYBACK_RESUMING: Selector = Selector::new("app.playback-resuming");
pub const PLAYBACK_BLOCKED: Selector = Selector::new("app.playback-blocked");
pub const PLAYBACK_STOPPED: Selector = Selector::new("app.playback-stopped");
pub const UPDATE_AUDIO_ANALYSIS: Selector<(TrackId, Result<AudioAnalysis, Error>)> =
    Selector::new("app.update-audio-analysis");

// Playback control

pub const PLAY_TRACK_AT: Selector<usize> = Selector::new("app.play-index");
pub const PLAY_TRACKS: Selector<PlaybackPayload> = Selector::new("app.play-tracks");
pub const PLAY_PREVIOUS: Selector = Selector::new("app.play-previous");
pub const PLAY_PAUSE: Selector = Selector::new("app.play-pause");
pub const PLAY_RESUME: Selector = Selector::new("app.play-resume");
pub const PLAY_NEXT: Selector = Selector::new("app.play-next");
pub const PLAY_STOP: Selector = Selector::new("app.play-stop");
pub const PLAY_QUEUE_BEHAVIOR: Selector<QueueBehavior> = Selector::new("app.play-queue-behavior");
pub const PLAY_SEEK: Selector<f64> = Selector::new("app.play-seek");
