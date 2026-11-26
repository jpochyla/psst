use crate::{
    core::dealer::protocol::SkipTo, protocol::context_player_options::ContextPlayerOptionOverrides,
};

use std::ops::Deref;

/// Request for loading playback
#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub(super) context: PlayContext,
    pub(super) options: LoadRequestOptions,
}

impl Deref for LoadRequest {
    type Target = LoadRequestOptions;

    fn deref(&self) -> &Self::Target {
        &self.options
    }
}

#[derive(Debug, Clone)]
pub(super) enum PlayContext {
    Uri(String),
    Tracks(Vec<String>),
}

/// The parameters for creating a load request
#[derive(Debug, Default, Clone)]
pub struct LoadRequestOptions {
    /// Whether the given tracks should immediately start playing, or just be initially loaded.
    pub start_playing: bool,
    /// Start the playback at a specific point of the track.
    ///
    /// The provided value is used as milliseconds. Providing a value greater
    /// than the track duration will start the track at the beginning.
    pub seek_to: u32,
    /// Options that decide how the context starts playing
    pub context_options: Option<LoadContextOptions>,
    /// Decides the starting position in the given context.
    ///
    /// If the provided item doesn't exist or is out of range,
    /// the playback starts at the beginning of the context.
    ///
    /// If `None` is provided and `shuffle` is `true`, a random track is played, otherwise the first
    pub playing_track: Option<PlayingTrack>,
}

/// The options which decide how the playback is started
///
/// Separated into an `enum` to exclude the other variants from being used
/// simultaneously, as they are not compatible.
#[derive(Debug, Clone)]
pub enum LoadContextOptions {
    /// Starts the context with options
    Options(Options),
    /// Starts the playback as the autoplay variant of the context
    ///
    /// This is the same as finishing a context and
    /// automatically continuing playback of similar tracks
    Autoplay,
}

/// The available options that indicate how to start the context
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// Start the context in shuffle mode
    pub shuffle: bool,
    /// Start the context in repeat mode
    pub repeat: bool,
    /// Start the context, repeating the first track until skipped or manually disabled
    pub repeat_track: bool,
}

impl From<ContextPlayerOptionOverrides> for Options {
    fn from(value: ContextPlayerOptionOverrides) -> Self {
        Self {
            shuffle: value.shuffling_context.unwrap_or_default(),
            repeat: value.repeating_context.unwrap_or_default(),
            repeat_track: value.repeating_track.unwrap_or_default(),
        }
    }
}

impl LoadRequest {
    /// Create a load request from a `context_uri`
    ///
    /// For supported `context_uri` see [`SpClient::get_context`](librespot_core::spclient::SpClient::get_context)
    ///
    /// Equivalent to using [`/me/player/play`](https://developer.spotify.com/documentation/web-api/reference/start-a-users-playback)
    /// and providing `context_uri`
    pub fn from_context_uri(context_uri: String, options: LoadRequestOptions) -> Self {
        Self {
            context: PlayContext::Uri(context_uri),
            options,
        }
    }

    /// Create a load request from a set of `tracks`
    ///
    /// Equivalent to using [`/me/player/play`](https://developer.spotify.com/documentation/web-api/reference/start-a-users-playback)
    /// and providing `uris`
    pub fn from_tracks(tracks: Vec<String>, options: LoadRequestOptions) -> Self {
        Self {
            context: PlayContext::Tracks(tracks),
            options,
        }
    }
}

/// An item that represent a track to play
#[derive(Debug, Clone)]
pub enum PlayingTrack {
    /// Represent the track at a given index.
    Index(u32),
    /// Represent the uri of a track.
    Uri(String),
    #[doc(hidden)]
    /// Represent an internal identifier from spotify.
    ///
    /// The internal identifier is not the id contained in the uri. And rather
    /// an unrelated id probably unique in spotify's internal database. But that's
    /// just speculation.
    ///
    /// This identifier is not available by any public api. It's used for varies in
    /// any spotify client, like sorting, displaying which track is currently played
    /// and skipping to a track. Mobile uses it pretty intensively but also web and
    /// desktop seem to make use of it.
    Uid(String),
}

impl TryFrom<SkipTo> for PlayingTrack {
    type Error = ();

    fn try_from(value: SkipTo) -> Result<Self, Self::Error> {
        // order of checks is important, as the index can be 0, but still has an uid or uri provided,
        // so we only use the index as last resort
        if let Some(uri) = value.track_uri {
            Ok(PlayingTrack::Uri(uri))
        } else if let Some(uid) = value.track_uid {
            Ok(PlayingTrack::Uid(uid))
        } else if let Some(index) = value.track_index {
            Ok(PlayingTrack::Index(index))
        } else {
            Err(())
        }
    }
}

#[derive(Debug)]
pub(super) enum SpircPlayStatus {
    Stopped,
    LoadingPlay {
        position_ms: u32,
    },
    LoadingPause {
        position_ms: u32,
    },
    Playing {
        nominal_start_time: i64,
        preloading_of_next_track_triggered: bool,
    },
    Paused {
        position_ms: u32,
        preloading_of_next_track_triggered: bool,
    },
}
