use crate::data::{Album, Artist, AudioAnalysis, Navigation, PlaybackReport, Playlist, Track};
use druid::{im::Vector, Selector};
use std::sync::Arc;

// Common

pub const SET_FOCUS: Selector = Selector::new("app.set-focus");
pub const COPY_TO_CLIPBOARD: Selector<String> = Selector::new("app.copy-to-clipboard");

// Session

pub const SESSION_CONNECTED: Selector = Selector::new("app.session-connected");
pub const SESSION_LOST: Selector = Selector::new("app.session-lost");

// Navigation

pub const NAVIGATE_TO: Selector<Navigation> = Selector::new("app.navigate-to");
pub const NAVIGATE_BACK: Selector = Selector::new("app.navigate-back");

// Search

pub const GOTO_SEARCH_RESULTS: Selector<String> = Selector::new("app.goto-search-results");
pub const UPDATE_SEARCH_RESULTS: Selector<(Vector<Artist>, Vector<Album>, Vector<Arc<Track>>)> =
    Selector::new("app.update-search-results");

// Library

pub const GOTO_LIBRARY: Selector = Selector::new("app.goto-library");
pub const LOAD_PLAYLISTS: Selector = Selector::new("app.load-playlists");
pub const UPDATE_PLAYLISTS: Selector<Vector<Playlist>> = Selector::new("app.update-playlists");
pub const UPDATE_SAVED_ALBUMS: Selector<Vector<Album>> = Selector::new("app.update-saved-albums");
pub const UPDATE_SAVED_TRACKS: Selector<Vector<Arc<Track>>> =
    Selector::new("app.update-saved-tracks");

pub const SAVE_TRACK: Selector<String> = Selector::new("app.save-track");
pub const UNSAVE_TRACK: Selector<String> = Selector::new("app.unsave-track");

// Album detail

pub const GOTO_ALBUM_DETAIL: Selector<String> = Selector::new("app.goto-album-detail");
pub const UPDATE_ALBUM_DETAIL: Selector<Album> = Selector::new("app.update-album-detail");

// Artist detail

pub const GOTO_ARTIST_DETAIL: Selector<String> = Selector::new("app.goto-artist-detail");
pub const UPDATE_ARTIST_DETAIL: Selector<Artist> = Selector::new("app.update-artist-detail");
pub const UPDATE_ARTIST_ALBUMS: Selector<Vector<Album>> = Selector::new("app.update-artist-album");
pub const UPDATE_ARTIST_TOP_TRACKS: Selector<Vector<Arc<Track>>> =
    Selector::new("app.update-artist-top_tracks");

// Playlist detail

pub const GOTO_PLAYLIST_DETAIL: Selector<Playlist> = Selector::new("app.goto-playlist-detail");
pub const UPDATE_PLAYLIST_TRACKS: Selector<Vector<Arc<Track>>> =
    Selector::new("app.update-playlist-tracks");

// Playback state

pub const PLAYBACK_PLAYING: Selector<PlaybackReport> = Selector::new("app.playback-playing");
pub const PLAYBACK_PAUSED: Selector = Selector::new("app.playback-paused");

pub const LOAD_AUDIO_ANALYSIS: Selector<String> = Selector::new("app.load-audio-analysis");
pub const UPDATE_AUDIO_ANALYSIS: Selector<AudioAnalysis> =
    Selector::new("app.update-audio-analysis");

// Playback control

pub const PLAY_TRACK_AT: Selector<usize> = Selector::new("app.play-index");
pub const PLAY_TRACKS: Selector<(Vector<Arc<Track>>, usize)> = Selector::new("app.play-tracks");
pub const PLAY_PREVIOUS: Selector = Selector::new("app.play-previous");
pub const PLAY_PAUSE: Selector = Selector::new("app.play-pause");
pub const PLAY_RESUME: Selector = Selector::new("app.play-resume");
pub const PLAY_NEXT: Selector = Selector::new("app.play-next");
pub const SEEK_TO_FRACTION: Selector<f64> = Selector::new("app.seek-to-fraction");
