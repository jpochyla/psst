use std::time::Duration;

use druid::{Selector, WidgetId};

use crate::{
    data::{Nav, PlaybackPayload, QueueBehavior, TrackId},
    ui::find::Find,
};

// Widget IDs

pub const WIDGET_SEARCH_INPUT: WidgetId = WidgetId::reserved(1);

// Common

pub const SHOW_MAIN: Selector = Selector::new("app.show-main");
pub const SET_FOCUS: Selector = Selector::new("app.set-focus");
pub const COPY: Selector<String> = Selector::new("app.copy-to-clipboard");

// Find

pub const TOGGLE_FINDER: Selector = Selector::new("app.show-finder");
pub const FIND_IN_PLAYLIST: Selector<Find> = Selector::new("find-in-playlist");

// Session

pub const SESSION_CONNECT: Selector = Selector::new("app.session-connect");

// Navigation

pub const NAVIGATE: Selector<Nav> = Selector::new("app.navigates");
pub const NAVIGATE_BACK: Selector<usize> = Selector::new("app.navigate-back");

// Playback state

pub const PLAYBACK_LOADING: Selector<TrackId> = Selector::new("app.playback-loading");
pub const PLAYBACK_PLAYING: Selector<(TrackId, Duration)> = Selector::new("app.playback-playing");
pub const PLAYBACK_PROGRESS: Selector<Duration> = Selector::new("app.playback-progress");
pub const PLAYBACK_PAUSING: Selector = Selector::new("app.playback-pausing");
pub const PLAYBACK_RESUMING: Selector = Selector::new("app.playback-resuming");
pub const PLAYBACK_BLOCKED: Selector = Selector::new("app.playback-blocked");
pub const PLAYBACK_STOPPED: Selector = Selector::new("app.playback-stopped");

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
