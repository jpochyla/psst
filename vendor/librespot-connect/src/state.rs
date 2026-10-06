pub(super) mod context;
mod handle;
mod metadata;
mod options;
pub(super) mod provider;
mod restrictions;
mod tracks;
mod transfer;

use crate::{
    core::{
        Error, Session, config::DeviceType, date::Date, dealer::protocol::Request,
        spclient::SpClientResult, version,
    },
    model::SpircPlayStatus,
    protocol::{
        connect::{Capabilities, Device, DeviceInfo, MemberType, PutStateReason, PutStateRequest},
        media::AudioQuality,
        player::{
            ContextIndex, ContextPlayerOptions, PlayOrigin, PlayerState, ProvidedTrack,
            Suppressions,
        },
    },
    state::{
        context::{ContextType, ResetContext, StateContext},
        options::ShuffleState,
        provider::{IsProvider, Provider},
    },
};
use log::LevelFilter;
use protobuf::{EnumOrUnknown, MessageField};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

// these limitations are essential, otherwise to many tracks will overload the web-player
const SPOTIFY_MAX_PREV_TRACKS_SIZE: usize = 10;
const SPOTIFY_MAX_NEXT_TRACKS_SIZE: usize = 80;

#[derive(Debug, Error)]
pub(super) enum StateError {
    #[error("the current track couldn't be resolved from the transfer state")]
    CouldNotResolveTrackFromTransfer,
    #[error("context is not available. type: {0:?}")]
    NoContext(ContextType),
    #[error("could not find track {0:?} in context of {1}")]
    CanNotFindTrackInContext(Option<usize>, usize),
    #[error("currently {action} is not allowed because {reason}")]
    CurrentlyDisallowed {
        action: &'static str,
        reason: String,
    },
    #[error("the provided context has no tracks")]
    ContextHasNoTracks,
    #[error("playback of local files is not supported")]
    UnsupportedLocalPlayback,
    #[error("track uri <{0:?}> contains invalid characters")]
    InvalidTrackUri(Option<String>),
}

impl From<StateError> for Error {
    fn from(err: StateError) -> Self {
        use StateError::*;
        match err {
            CouldNotResolveTrackFromTransfer
            | NoContext(_)
            | CanNotFindTrackInContext(_, _)
            | ContextHasNoTracks
            | InvalidTrackUri(_) => Error::failed_precondition(err),
            CurrentlyDisallowed { .. } | UnsupportedLocalPlayback => Error::unavailable(err),
        }
    }
}

/// Configuration of the connect device
#[derive(Debug, Clone)]
pub struct ConnectConfig {
    /// The name of the connect device (default: librespot)
    pub name: String,
    /// The icon type of the connect device (default: [DeviceType::Speaker])
    pub device_type: DeviceType,
    /// Displays the [DeviceType] twice in the ui to show up as a group (default: false)
    pub is_group: bool,
    /// The volume with which the connect device will be initialized (default: 50%)
    pub initial_volume: u16,
    /// Disables the option to control the volume remotely (default: false)
    pub disable_volume: bool,
    /// Number of incremental steps (default: 64)
    pub volume_steps: u16,
}

impl Default for ConnectConfig {
    fn default() -> Self {
        Self {
            name: "librespot".to_string(),
            device_type: DeviceType::Speaker,
            is_group: false,
            initial_volume: u16::MAX / 2,
            disable_volume: false,
            volume_steps: 64,
        }
    }
}

#[derive(Default, Debug)]
pub(super) struct ConnectState {
    /// the entire state that is updated to the remote server
    request: PutStateRequest,

    unavailable_uri: Vec<String>,

    active_since: Option<SystemTime>,
    queue_count: u64,

    // separation is necessary because we could have already loaded
    // the autoplay context but are still playing from the default context
    /// to update the active context use [switch_active_context](ConnectState::set_active_context)
    pub active_context: ContextType,
    fill_up_context: ContextType,

    /// the context from which we play, is used to top up prev and next tracks
    context: Option<StateContext>,
    /// seed extracted in [ConnectState::handle_initial_transfer] and used in [ConnectState::finish_transfer]
    transfer_shuffle: Option<ShuffleState>,

    /// a context to keep track of the autoplay context
    autoplay_context: Option<StateContext>,

    /// The volume adjustment per step when handling individual volume adjustments.
    pub volume_step_size: u16,
}

impl ConnectState {
    pub fn new(cfg: ConnectConfig, session: &Session) -> Self {
        let volume_step_size = u16::MAX.checked_div(cfg.volume_steps).unwrap_or(1024);

        let device_info = DeviceInfo {
            can_play: true,
            volume: cfg.initial_volume.into(),
            name: cfg.name,
            device_id: session.device_id().to_string(),
            device_type: EnumOrUnknown::new(cfg.device_type.into()),
            device_software_version: version::SEMVER.to_string(),
            spirc_version: version::SPOTIFY_SPIRC_VERSION.to_string(),
            client_id: session.client_id(),
            is_group: cfg.is_group,
            capabilities: MessageField::some(Capabilities {
                volume_steps: cfg.volume_steps.into(),
                disable_volume: cfg.disable_volume,

                gaia_eq_connect_id: true,
                can_be_player: true,
                needs_full_player_state: true,
                is_observable: true,
                is_controllable: true,
                hidden: false,

                supports_gzip_pushes: true,
                // todo: enable after logout handling is implemented, see spirc logout_request
                supports_logout: false,
                supported_types: vec![
                    "audio/episode".into(),
                    "audio/track".into(),
                    "audio/local".into(),
                ],
                supports_playlist_v2: true,
                supports_transfer_command: true,
                supports_command_request: true,
                supports_set_options_command: true,

                is_voice_enabled: false,
                restrict_to_local: false,
                connect_disabled: false,
                supports_rename: false,
                supports_external_episodes: false,
                supports_set_backend_metadata: false,
                supports_hifi: MessageField::none(),
                // that "AI" dj thingy only available to specific regions/users
                supports_dj: false,
                supports_rooms: false,
                // AudioQuality::HIFI is available, further investigation necessary
                supported_audio_quality: EnumOrUnknown::new(AudioQuality::VERY_HIGH),

                command_acks: true,

                ..Default::default()
            }),
            ..Default::default()
        };

        let mut state = Self {
            request: PutStateRequest {
                member_type: EnumOrUnknown::new(MemberType::CONNECT_STATE),
                put_state_reason: EnumOrUnknown::new(PutStateReason::PLAYER_STATE_CHANGED),
                device: MessageField::some(Device {
                    device_info: MessageField::some(device_info),
                    player_state: MessageField::some(PlayerState {
                        session_id: session.session_id(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            volume_step_size,
            ..Default::default()
        };
        state.reset();
        state
    }

    fn reset(&mut self) {
        self.set_active(false);
        self.queue_count = 0;

        // preserve the session_id
        let session_id = self.player().session_id.clone();

        self.device_mut().player_state = MessageField::some(PlayerState {
            session_id,
            is_system_initiated: true,
            playback_speed: 1.,
            play_origin: MessageField::some(PlayOrigin::new()),
            suppressions: MessageField::some(Suppressions::new()),
            options: MessageField::some(ContextPlayerOptions::new()),
            // + 1, so that we have a buffer where we can swap elements
            prev_tracks: Vec::with_capacity(SPOTIFY_MAX_PREV_TRACKS_SIZE + 1),
            next_tracks: Vec::with_capacity(SPOTIFY_MAX_NEXT_TRACKS_SIZE + 1),
            ..Default::default()
        });
    }

    fn device_mut(&mut self) -> &mut Device {
        self.request
            .device
            .as_mut()
            .expect("the request is always available")
    }

    fn player_mut(&mut self) -> &mut PlayerState {
        self.device_mut()
            .player_state
            .as_mut()
            .expect("the player_state has to be always given")
    }

    pub fn device_info(&self) -> &DeviceInfo {
        &self.request.device.device_info
    }

    pub fn player(&self) -> &PlayerState {
        &self.request.device.player_state
    }

    pub fn is_active(&self) -> bool {
        self.request.is_active
    }

    /// Returns the `is_playing` value as perceived by other connect devices
    ///
    /// see [ConnectState::set_status]
    pub fn is_playing(&self) -> bool {
        let player = self.player();
        player.is_playing && !player.is_paused
    }

    /// Returns the `is_paused` state value as perceived by other connect devices
    ///
    /// see [ConnectState::set_status]
    pub fn is_pause(&self) -> bool {
        let player = self.player();
        player.is_playing && player.is_paused && player.is_buffering
    }

    pub fn set_volume(&mut self, volume: u32) {
        self.device_mut()
            .device_info
            .as_mut()
            .expect("the device_info has to be always given")
            .volume = volume;
    }

    pub fn set_last_command(&mut self, command: Request) {
        self.request.last_command_message_id = command.message_id;
        self.request.last_command_sent_by_device_id = command.sent_by_device_id;
    }

    pub fn set_now(&mut self, now: u64) {
        self.request.client_side_timestamp = now;

        if let Some(active_since) = self.active_since {
            if let Ok(active_since_duration) = active_since.duration_since(UNIX_EPOCH) {
                match active_since_duration.as_millis().try_into() {
                    Ok(active_since_ms) => self.request.started_playing_at = active_since_ms,
                    Err(why) => warn!("couldn't update active since because {why}"),
                }
            }
        }
    }

    pub fn set_active(&mut self, value: bool) {
        if value {
            if self.request.is_active {
                return;
            }

            self.request.is_active = true;
            self.active_since = Some(SystemTime::now())
        } else {
            self.request.is_active = false;
            self.active_since = None
        }
    }

    pub fn set_origin(&mut self, origin: PlayOrigin) {
        self.player_mut().play_origin = MessageField::some(origin)
    }

    pub fn set_session_id(&mut self, session_id: String) {
        self.player_mut().session_id = session_id;
    }

    pub(crate) fn set_status(&mut self, status: &SpircPlayStatus) {
        let player = self.player_mut();
        player.is_paused = matches!(
            status,
            SpircPlayStatus::LoadingPause { .. }
                | SpircPlayStatus::Paused { .. }
                | SpircPlayStatus::Stopped
        );

        if player.is_paused {
            player.playback_speed = 0.;
        } else {
            player.playback_speed = 1.;
        }

        // desktop and mobile require all 'states' set to true, when we are paused,
        // otherwise the play button (desktop) is grayed out or the preview (mobile) can't be opened
        player.is_buffering = player.is_paused
            || matches!(
                status,
                SpircPlayStatus::LoadingPause { .. } | SpircPlayStatus::LoadingPlay { .. }
            );
        player.is_playing = player.is_paused
            || matches!(
                status,
                SpircPlayStatus::LoadingPlay { .. } | SpircPlayStatus::Playing { .. }
            );

        debug!(
            "updated connect play status playing: {}, paused: {}, buffering: {}",
            player.is_playing, player.is_paused, player.is_buffering
        );

        self.update_restrictions()
    }

    /// index is 0 based, so the first track is index 0
    pub fn update_current_index(&mut self, f: impl Fn(&mut ContextIndex)) {
        match self.player_mut().index.as_mut() {
            Some(player_index) => f(player_index),
            None => {
                let mut new_index = ContextIndex::new();
                f(&mut new_index);
                self.player_mut().index = MessageField::some(new_index)
            }
        }
    }

    pub fn update_position(&mut self, position_ms: u32, timestamp: i64) {
        let player = self.player_mut();
        player.position_as_of_timestamp = position_ms.into();
        player.timestamp = timestamp;
    }

    pub fn update_duration(&mut self, duration: u32) {
        self.player_mut().duration = duration.into()
    }

    pub fn update_queue_revision(&mut self) {
        let mut state = DefaultHasher::new();
        self.next_tracks()
            .iter()
            .for_each(|t| t.uri.hash(&mut state));
        self.player_mut().queue_revision = state.finish().to_string()
    }

    pub fn reset_playback_to_position(&mut self, new_index: Option<usize>) -> Result<(), Error> {
        debug!(
            "reset_playback with active ctx <{:?}> fill_up ctx <{:?}>",
            self.active_context, self.fill_up_context
        );

        let new_index = new_index.unwrap_or(0);
        self.update_current_index(|i| i.track = new_index as u32);
        self.update_context_index(self.active_context, new_index + 1)?;
        self.fill_up_context = self.active_context;

        if !self.current_track(|t| t.is_queue() || self.is_skip_track(t, None)) {
            self.set_current_track(new_index)?;
        }

        self.clear_prev_track();

        if new_index > 0 {
            let context = self.get_context(self.active_context)?;

            let before_new_track = context.tracks.len() - new_index;
            self.player_mut().prev_tracks = context
                .tracks
                .iter()
                .rev()
                .skip(before_new_track)
                .take(SPOTIFY_MAX_PREV_TRACKS_SIZE)
                .rev()
                .cloned()
                .collect();
            debug!("has {} prev tracks", self.prev_tracks().len())
        }

        self.clear_next_tracks();
        self.fill_up_next_tracks()?;
        self.update_restrictions();

        Ok(())
    }

    fn mark_as_unavailable_for_match(track: &mut ProvidedTrack, uri: &str) {
        if track.uri == uri {
            debug!("Marked <{}:{}> as unavailable", track.provider, track.uri);
            track.set_provider(Provider::Unavailable);
        }
    }

    pub fn update_position_in_relation(&mut self, timestamp: i64) {
        let player = self.player_mut();

        let diff = timestamp - player.timestamp;
        player.position_as_of_timestamp += diff;

        if log::max_level() >= LevelFilter::Debug {
            let pos = Duration::from_millis(player.position_as_of_timestamp as u64);
            let time = Date::from_timestamp_ms(timestamp)
                .map(|d| d.time().to_string())
                .unwrap_or_else(|_| timestamp.to_string());

            let sec = pos.as_secs();
            let (min, sec) = (sec / 60, sec % 60);
            debug!("update position to {min}:{sec:0>2} at {time}");
        }

        player.timestamp = timestamp;
    }

    pub async fn became_inactive(&mut self, session: &Session) -> SpClientResult {
        self.reset();
        self.reset_context(ResetContext::Completely);

        session.spclient().put_connect_state_inactive(false).await
    }

    async fn send_with_reason(
        &mut self,
        session: &Session,
        reason: PutStateReason,
    ) -> SpClientResult {
        let prev_reason = self.request.put_state_reason;

        self.request.put_state_reason = EnumOrUnknown::new(reason);
        let res = self.send_state(session).await;

        self.request.put_state_reason = prev_reason;
        res
    }

    /// Notifies the remote server about a new device
    pub async fn notify_new_device_appeared(&mut self, session: &Session) -> SpClientResult {
        self.send_with_reason(session, PutStateReason::NEW_DEVICE)
            .await
    }

    /// Notifies the remote server about a new volume
    pub async fn notify_volume_changed(&mut self, session: &Session) -> SpClientResult {
        self.send_with_reason(session, PutStateReason::VOLUME_CHANGED)
            .await
    }

    /// Sends the connect state for the connect session to the remote server
    pub async fn send_state(&self, session: &Session) -> SpClientResult {
        session
            .spclient()
            .put_connect_state_request(&self.request)
            .await
    }
}
