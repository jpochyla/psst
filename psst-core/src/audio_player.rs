use std::{
    mem,
    sync::{Arc, Mutex},
    thread,
    thread::JoinHandle,
    time::Duration,
};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::{
    audio_file::{AudioFile, AudioPath, FileAudioSource},
    audio_key::AudioKey,
    audio_normalize::NormalizationLevel,
    audio_output::{AudioOutputRemote, AudioSample, AudioSource},
    audio_queue::{Queue, QueueBehavior},
    cache::CacheHandle,
    cdn::CdnHandle,
    error::Error,
    item_id::{ItemId, ItemIdType},
    metadata::{Fetch, ToAudioPath},
    protocol::metadata::Track,
    session::SessionService,
};

const PREVIOUS_TRACK_THRESHOLD: Duration = Duration::from_secs(3);
const STOP_AFTER_CONSECUTIVE_LOADING_FAILURES: usize = 3;

#[derive(Clone)]
pub struct PlaybackConfig {
    pub bitrate: usize,
    pub pregain: f32,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            bitrate: 320,
            pregain: 3.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PlaybackItem {
    pub item_id: ItemId,
    pub norm_level: NormalizationLevel,
}

impl PlaybackItem {
    fn load(
        &self,
        session: &SessionService,
        cdn: CdnHandle,
        cache: CacheHandle,
        config: &PlaybackConfig,
    ) -> Result<LoadedPlaybackItem, Error> {
        let path = load_audio_path(self.item_id, session, &cache, config)?;
        let key = load_audio_key(&path, session, &cache)?;
        let file = AudioFile::open(path, cdn, cache)?;
        let (source, norm_data) = file.audio_source(key)?;
        let norm_factor = norm_data.factor_for_level(self.norm_level, config.pregain);
        Ok(LoadedPlaybackItem {
            file,
            source,
            norm_factor,
        })
    }
}

fn load_audio_path(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<AudioPath, Error> {
    match item_id.id_type {
        ItemIdType::Track => {
            load_audio_path_from_track_or_alternative(item_id, session, cache, config)
        }
        ItemIdType::Podcast | ItemIdType::Unknown => unimplemented!(),
    }
}

fn load_audio_path_from_track_or_alternative(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<AudioPath, Error> {
    let track = load_track(item_id, session, cache)?;
    let country = get_country_code(session, cache);
    let path = match country {
        Some(user_country) if track.is_restricted_in_region(&user_country) => {
            // The track is regionally restricted and is unavailable.  Let's try to find an
            // alternative track.
            let alt_id = track
                .find_allowed_alternative(&user_country)
                .ok_or(Error::AudioFileNotFound)?;
            let alt_track = load_track(alt_id, session, cache)?;
            let alt_path = alt_track
                .to_audio_path(config.bitrate)
                .ok_or(Error::AudioFileNotFound)?;
            // We've found an alternative track with a fitting audio file.  Let's cheat a
            // little and pretend we've obtained it from the requested track.
            // TODO: We should be honest and display the real track information.
            AudioPath {
                item_id,
                ..alt_path
            }
        }
        _ => {
            // Either we do not have a country code loaded or the track is available, return
            // it.
            track
                .to_audio_path(config.bitrate)
                .ok_or(Error::AudioFileNotFound)?
        }
    };
    Ok(path)
}

fn get_country_code(session: &SessionService, cache: &CacheHandle) -> Option<String> {
    if let Some(cached_country_code) = cache.get_country_code() {
        Some(cached_country_code)
    } else {
        let country_code = session.connected().ok()?.get_country_code()?;
        if let Err(err) = cache.save_country_code(&country_code) {
            log::warn!("failed to save country code to cache: {:?}", err);
        }
        Some(country_code)
    }
}

fn load_track(
    item_id: ItemId,
    session: &SessionService,
    cache: &CacheHandle,
) -> Result<Track, Error> {
    if let Some(cached_track) = cache.get_track(item_id) {
        Ok(cached_track)
    } else {
        let track = Track::fetch(session, item_id)?;
        if let Err(err) = cache.save_track(item_id, &track) {
            log::warn!("failed to save track to cache: {:?}", err);
        }
        Ok(track)
    }
}

fn load_audio_key(
    path: &AudioPath,
    session: &SessionService,
    cache: &CacheHandle,
) -> Result<AudioKey, Error> {
    if let Some(cached_key) = cache.get_audio_key(path.item_id, path.file_id) {
        Ok(cached_key)
    } else {
        let key = session
            .connected()?
            .get_audio_key(path.item_id, path.file_id)?;
        if let Err(err) = cache.save_audio_key(path.item_id, path.file_id, &key) {
            log::warn!("failed to save audio key to cache: {:?}", err);
        }
        Ok(key)
    }
}

pub struct LoadedPlaybackItem {
    file: AudioFile,
    source: FileAudioSource,
    norm_factor: f32,
}

pub struct Player {
    state: PlayerState,
    preload: PreloadState,
    session: SessionService,
    cdn: CdnHandle,
    cache: CacheHandle,
    config: PlaybackConfig,
    queue: Queue,
    event_sender: Sender<PlayerEvent>,
    event_receiver: Receiver<PlayerEvent>,
    audio_source: Arc<Mutex<PlayerAudioSource>>,
    audio_output_remote: AudioOutputRemote,
    consecutive_loading_failures: usize,
}

impl Player {
    pub fn new(
        session: SessionService,
        cdn: CdnHandle,
        cache: CacheHandle,
        config: PlaybackConfig,
        audio_output_remote: AudioOutputRemote,
    ) -> Self {
        let (event_sender, event_receiver) = unbounded();
        let audio_source = {
            let event_sender = event_sender.clone();
            Arc::new(Mutex::new(PlayerAudioSource::new(event_sender)))
        };
        Self {
            session,
            cdn,
            cache,
            config,
            event_sender,
            event_receiver,
            audio_source,
            audio_output_remote,
            state: PlayerState::Stopped,
            preload: PreloadState::None,
            queue: Queue::new(),
            consecutive_loading_failures: 0,
        }
    }

    pub fn audio_source(&self) -> Arc<Mutex<impl AudioSource>> {
        self.audio_source.clone()
    }

    pub fn event_sender(&self) -> Sender<PlayerEvent> {
        self.event_sender.clone()
    }

    pub fn event_receiver(&self) -> Receiver<PlayerEvent> {
        self.event_receiver.clone()
    }

    pub fn handle(&mut self, event: PlayerEvent) {
        match event {
            PlayerEvent::Command(cmd) => {
                self.handle_command(cmd);
            }
            PlayerEvent::Loaded { item, result } => {
                self.handle_loaded(item, result);
            }
            PlayerEvent::Preloaded { item, result } => {
                self.handle_preloaded(item, result);
            }
            PlayerEvent::Progress { duration, path } => {
                self.handle_progress(duration, path);
            }
            PlayerEvent::Finished { .. } => {
                self.handle_finished();
            }
            PlayerEvent::Loading { .. }
            | PlayerEvent::Playing { .. }
            | PlayerEvent::Pausing { .. }
            | PlayerEvent::Resuming { .. }
            | PlayerEvent::Stopped { .. }
            | PlayerEvent::Blocked => {}
        };
    }

    fn handle_command(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::LoadQueue { items, position } => self.load_queue(items, position),
            PlayerCommand::LoadAndPlay { item } => self.load_and_play(item),
            PlayerCommand::Preload { item } => self.preload(item),
            PlayerCommand::Pause => self.pause(),
            PlayerCommand::Resume => self.resume(),
            PlayerCommand::PauseOrResume => self.pause_or_resume(),
            PlayerCommand::Previous => self.previous(),
            PlayerCommand::Next => self.next(),
            PlayerCommand::Stop => self.stop(),
            PlayerCommand::Seek { position } => self.seek(position),
            PlayerCommand::Configure { config } => self.configure(config),
            PlayerCommand::SetQueueBehavior { behavior } => self.queue.set_behaviour(behavior),
            PlayerCommand::SetVolume { volume } => self.set_volume(volume),
        }
    }

    fn handle_loaded(&mut self, item: PlaybackItem, result: Result<LoadedPlaybackItem, Error>) {
        match self.state {
            PlayerState::Loading {
                item: requested_item,
                ..
            } if item == requested_item => match result {
                Ok(loaded_item) => {
                    self.consecutive_loading_failures = 0;
                    self.play_loaded(loaded_item);
                }
                Err(err) => {
                    self.consecutive_loading_failures += 1;
                    if self.consecutive_loading_failures < STOP_AFTER_CONSECUTIVE_LOADING_FAILURES {
                        log::error!("skipping, error while loading: {}", err);
                        self.next();
                    } else {
                        log::error!("stopping, error while loading: {}", err);
                        self.stop();
                    }
                }
            },
            _ => {
                log::info!("stale load result received, ignoring");
            }
        }
    }

    fn handle_preloaded(&mut self, item: PlaybackItem, result: Result<LoadedPlaybackItem, Error>) {
        match self.preload {
            PreloadState::Preloading {
                item: requested_item,
                ..
            } if item == requested_item => match result {
                Ok(loaded_item) => {
                    log::info!("preloaded audio file");
                    self.preload = PreloadState::Preloaded { item, loaded_item };
                }
                Err(err) => {
                    log::error!("failed to preload audio file, error while opening: {}", err);
                    self.preload = PreloadState::None;
                }
            },
            _ => {
                log::info!("stale preload result received, ignoring");
            }
        }
    }

    fn handle_progress(&mut self, progress: Duration, path: AudioPath) {
        match &mut self.state {
            PlayerState::Playing { duration, .. } | PlayerState::Paused { duration, .. } => {
                *duration = progress;
            }
            _ => {
                log::warn!("received unexpected progress report");
            }
        }
        const PRELOAD_BEFORE_END_OF_TRACK: Duration = Duration::from_secs(30);
        if let Some(&item_to_preload) = self.queue.get_following() {
            let time_until_end_of_track = path.duration.checked_sub(progress).unwrap_or_default();
            if time_until_end_of_track <= PRELOAD_BEFORE_END_OF_TRACK {
                self.preload(item_to_preload);
            }
        }
    }

    fn handle_finished(&mut self) {
        self.queue.skip_to_following();
        if let Some(&item) = self.queue.get_current() {
            self.load_and_play(item);
        } else {
            self.stop();
        }
    }

    fn load_queue(&mut self, items: Vec<PlaybackItem>, position: usize) {
        self.queue.fill(items, position);
        if let Some(&item) = self.queue.get_current() {
            self.load_and_play(item);
        } else {
            self.stop();
        }
    }

    fn load_and_play(&mut self, item: PlaybackItem) {
        // Check if the item is already preloaded, and if so, take it out of the
        // preloader state, and start the playback.
        match mem::replace(&mut self.preload, PreloadState::None) {
            PreloadState::Preloaded {
                item: preloaded_item,
                loaded_item,
            } if preloaded_item == item => {
                self.play_loaded(loaded_item);
                return;
            }
            preloading_or_none => {
                // Restore the preloader to the previous state.
                // TODO: If the item is being preloaded, extract the loading handle.
                self.preload = preloading_or_none;
            }
        }
        // Item is not preloaded yet, load it in a background thread.
        let loading_handle = thread::spawn({
            let event_sender = self.event_sender.clone();
            let session = self.session.clone();
            let cdn = self.cdn.clone();
            let cache = self.cache.clone();
            let config = self.config.clone();
            move || {
                let result = item.load(&session, cdn, cache, &config);
                event_sender
                    .send(PlayerEvent::Loaded { item, result })
                    .expect("Failed to send PlayerEvent::Loaded");
            }
        });
        // Make sure the output is paused, so any currently playing item is stopped.
        self.audio_output_remote.pause();
        self.event_sender
            .send(PlayerEvent::Loading { item })
            .expect("Failed to send PlayerEvent::Loading");
        self.state = PlayerState::Loading {
            item,
            _loading_handle: loading_handle,
        };
    }

    fn preload(&mut self, item: PlaybackItem) {
        if self.is_in_preload(item) {
            return;
        }
        let loading_handle = thread::spawn({
            let event_sender = self.event_sender.clone();
            let session = self.session.clone();
            let cdn = self.cdn.clone();
            let cache = self.cache.clone();
            let config = self.config.clone();
            move || {
                let result = item.load(&session, cdn, cache, &config);
                event_sender
                    .send(PlayerEvent::Preloaded { item, result })
                    .expect("Failed to send PlayerEvent::Preloaded");
            }
        });
        self.preload = PreloadState::Preloading {
            item,
            _loading_handle: loading_handle,
        };
    }

    fn set_volume(&mut self, volume: f64) {
        self.audio_output_remote.set_volume(volume);
    }

    fn play_loaded(&mut self, loaded_item: LoadedPlaybackItem) {
        log::info!("starting playback");
        let path = loaded_item.file.path();
        let duration = Duration::default();
        self.audio_source
            .lock()
            .expect("Failed to acquire audio source lock")
            .play_now(loaded_item);
        self.event_sender
            .send(PlayerEvent::Playing { path, duration })
            .expect("Failed to send PlayerEvent::Playing");
        self.state = PlayerState::Playing { path, duration };
        self.audio_output_remote.resume();
    }

    fn pause(&mut self) {
        match mem::replace(&mut self.state, PlayerState::Invalid) {
            PlayerState::Playing { path, duration } | PlayerState::Paused { path, duration } => {
                log::info!("pausing playback");
                self.event_sender
                    .send(PlayerEvent::Pausing { path, duration })
                    .expect("Failed to send PlayerEvent::Paused");
                self.state = PlayerState::Paused { path, duration };
                self.audio_output_remote.pause();
            }
            _ => {
                log::warn!("invalid state transition");
            }
        }
    }

    fn resume(&mut self) {
        match mem::replace(&mut self.state, PlayerState::Invalid) {
            PlayerState::Playing { path, duration } | PlayerState::Paused { path, duration } => {
                log::info!("resuming playback");
                self.event_sender
                    .send(PlayerEvent::Resuming { path, duration })
                    .expect("Failed to send PlayerEvent::Resuming");
                self.state = PlayerState::Playing { path, duration };
                self.audio_output_remote.resume();
            }
            _ => {
                log::warn!("invalid state transition");
            }
        }
    }

    fn pause_or_resume(&mut self) {
        match &self.state {
            PlayerState::Playing { .. } => self.pause(),
            PlayerState::Paused { .. } => self.resume(),
            _ => {
                // Do nothing.
            }
        }
    }

    fn previous(&mut self) {
        if self.is_near_playback_start() {
            self.queue.skip_to_previous();
            if let Some(&item) = self.queue.get_current() {
                self.load_and_play(item);
            } else {
                self.stop();
            }
        } else {
            self.seek(Duration::default());
        }
    }

    fn next(&mut self) {
        self.queue.skip_to_next();
        if let Some(&item) = self.queue.get_current() {
            self.load_and_play(item);
        } else {
            self.stop();
        }
    }

    fn stop(&mut self) {
        self.event_sender
            .send(PlayerEvent::Stopped)
            .expect("Failed to send PlayerEvent::Stopped");
        self.state = PlayerState::Stopped;
        self.audio_output_remote.pause();
        self.queue.clear();
        self.consecutive_loading_failures = 0;
    }

    fn seek(&mut self, position: Duration) {
        self.audio_source
            .lock()
            .expect("Failed to acquire audio source lock")
            .seek(position);
    }

    fn configure(&mut self, config: PlaybackConfig) {
        self.config = config;
    }

    fn is_near_playback_start(&self) -> bool {
        match self.state {
            PlayerState::Playing { duration, .. } | PlayerState::Paused { duration, .. } => {
                duration < PREVIOUS_TRACK_THRESHOLD
            }
            _ => false,
        }
    }

    fn is_in_preload(&self, item: PlaybackItem) -> bool {
        match self.preload {
            PreloadState::Preloading { item: p_item, .. }
            | PreloadState::Preloaded { item: p_item, .. } => p_item == item,
            _ => false,
        }
    }
}

pub enum PlayerCommand {
    LoadQueue {
        items: Vec<PlaybackItem>,
        position: usize,
    },
    LoadAndPlay {
        item: PlaybackItem,
    },
    Preload {
        item: PlaybackItem,
    },
    Pause,
    Resume,
    PauseOrResume,
    Previous,
    Next,
    Stop,
    Seek {
        position: Duration,
    },
    Configure {
        config: PlaybackConfig,
    },
    SetQueueBehavior {
        behavior: QueueBehavior,
    },
    /// Change playback volume to a value in 0.0..=1.0 range.
    SetVolume {
        volume: f64,
    },
}

pub enum PlayerEvent {
    Command(PlayerCommand),
    /// Track has started loading.  `Loaded` follows.
    Loading {
        item: PlaybackItem,
    },
    /// Track loading either succeeded or failed.  `Playing` follows in case of
    /// success.
    Loaded {
        item: PlaybackItem,
        result: Result<LoadedPlaybackItem, Error>,
    },
    /// Next item in queue has been either successfully preloaded or failed to
    /// preload.
    Preloaded {
        item: PlaybackItem,
        result: Result<LoadedPlaybackItem, Error>,
    },
    /// Player has started playing new track.  `Progress` events will follow.
    Playing {
        path: AudioPath,
        duration: Duration,
    },
    /// Player is in a paused state.  `Resuming` might follow.
    Pausing {
        path: AudioPath,
        duration: Duration,
    },
    /// Player is resuming playback of a track.  `Progress` events will follow.
    Resuming {
        path: AudioPath,
        duration: Duration,
    },
    /// Player is either reacting to a seek event in a paused or playing state,
    /// or track is naturally progressing during playback.
    Progress {
        path: AudioPath,
        duration: Duration,
    },
    /// Player would like to continue playing, but is blocked, waiting for I/O.
    Blocked,
    /// Player has finished playing a track.  `Loading` or `Playing` might
    /// follow if the queue is not empty, `Stopped` will follow if it is.
    Finished,
    /// The queue is empty.
    Stopped,
}

enum PlayerState {
    Loading {
        item: PlaybackItem,
        _loading_handle: JoinHandle<()>,
    },
    Playing {
        path: AudioPath,
        duration: Duration,
    },
    Paused {
        path: AudioPath,
        duration: Duration,
    },
    Stopped,
    Invalid,
}

enum PreloadState {
    Preloading {
        item: PlaybackItem,
        _loading_handle: JoinHandle<()>,
    },
    Preloaded {
        item: PlaybackItem,
        loaded_item: LoadedPlaybackItem,
    },
    None,
}

const OUTPUT_CHANNELS: u8 = 2;
const OUTPUT_SAMPLE_RATE: u32 = 44100;
const PROGRESS_PRECISION_SAMPLES: u64 = OUTPUT_SAMPLE_RATE as u64 * OUTPUT_CHANNELS as u64; // 1 second.

struct CurrentPlaybackItem {
    file: AudioFile,
    source: FileAudioSource,
    norm_factor: f32,
}

struct PlayerAudioSource {
    current: Option<CurrentPlaybackItem>,
    event_sender: Sender<PlayerEvent>,
    samples: u64,
}

impl PlayerAudioSource {
    fn new(event_sender: Sender<PlayerEvent>) -> Self {
        Self {
            event_sender,
            current: None,
            samples: 0,
        }
    }

    fn seek(&mut self, position: Duration) {
        if let Some(current) = &mut self.current {
            let seconds = position.as_secs_f64();
            let frames = seconds * f64::from(OUTPUT_SAMPLE_RATE);
            let samples = frames * f64::from(OUTPUT_CHANNELS);
            current.source.seek(frames as u64);
            self.samples = samples as u64;
            self.report_audio_position();
        }
    }

    fn play_now(&mut self, item: LoadedPlaybackItem) {
        self.current.replace(CurrentPlaybackItem {
            norm_factor: item.norm_factor,
            source: item.source,
            file: item.file,
        });
        self.samples = 0;
    }

    fn next_sample(&mut self) -> Option<AudioSample> {
        if let Some(current) = &mut self.current {
            let sample = current.source.next();
            if sample.is_some() {
                self.samples += 1;
            } else {
                self.samples = 0;
            }
            sample
        } else {
            None
        }
    }

    fn report_audio_position(&self) {
        if let Some(current) = &self.current {
            let duration = Duration::from_secs_f64(
                self.samples as f64 / f64::from(OUTPUT_SAMPLE_RATE) / f64::from(OUTPUT_CHANNELS),
            );
            let path = current.file.path();
            self.event_sender
                .send(PlayerEvent::Progress { duration, path })
                .expect("Failed to send PlayerEvent::Progress");
        }
    }

    fn report_audio_end(&self) {
        self.event_sender
            .send(PlayerEvent::Finished)
            .expect("Failed to send PlayerEvent::Finished");
    }
}

impl AudioSource for PlayerAudioSource {
    fn channels(&self) -> u8 {
        OUTPUT_CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        OUTPUT_SAMPLE_RATE
    }

    fn normalization_factor(&self) -> Option<f32> {
        self.current.as_ref().map(|current| current.norm_factor)
    }
}

impl Iterator for PlayerAudioSource {
    type Item = AudioSample;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.next_sample();
        if sample.is_some() {
            // Report audio progress.
            if self.samples % PROGRESS_PRECISION_SAMPLES == 0 {
                self.report_audio_position();
            }
        } else {
            // We're at the end of track.  If we still have the source, drop it and report.
            // Player will pause the audio output and we will stop getting polled
            // eventually.
            if self.current.take().is_some() {
                self.report_audio_end();
            }
        }
        sample
    }
}
