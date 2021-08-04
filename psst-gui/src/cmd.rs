use crate::{
    data::{
        Album, AlbumLink, AudioAnalysis, Nav, PlaybackPayload, QueueBehavior, Recommendations,
        RecommendationsRequest, SearchResults, Track, TrackId,
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

// Navigation

pub const NAVIGATE: Selector<Nav> = Selector::new("app.navigates");
pub const NAVIGATE_BACK: Selector<usize> = Selector::new("app.navigate-back");

// Search

pub const LOAD_SEARCH_RESULTS: Selector<Arc<str>> = Selector::new("app.load-search-results");
pub const UPDATE_SEARCH_RESULTS: Selector<Result<SearchResults, Error>> =
    Selector::new("app.update-search-results");
pub const OPEN_LINK: Selector<Result<Nav, Error>> = Selector::new("app.open-link");

// Recommend

pub const LOAD_RECOMMENDATIONS: Selector<RecommendationsRequest> =
    Selector::new("app.load-recommendations");
pub const UPDATE_RECOMMENDATIONS: Selector<Result<Recommendations, Error>> =
    Selector::new("app.update-recommendations");

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
