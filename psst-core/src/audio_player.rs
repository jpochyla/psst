use crate::{
    audio_file::{AudioFile, AudioPath, FileAudioSource},
    audio_key::AudioKey,
    audio_output::{AudioOutputRemote, AudioSample, AudioSource},
    cache::CacheHandle,
    cdn::CdnHandle,
    error::Error,
    item_id::{ItemId, ItemIdType},
    metadata::{Fetch, ToAudioPath},
    protocol::metadata::Track,
    session::SessionHandle,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    mem,
    sync::{Arc, Mutex},
    thread,
    thread::JoinHandle,
    time::Duration,
};

const PREVIOUS_TRACK_THRESHOLD: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct PlaybackConfig {
    pub bitrate: usize,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self { bitrate: 320 }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PlaybackItem {
    pub item_id: ItemId,
}

impl PlaybackItem {
    fn load(
        &self,
        session: SessionHandle,
        cdn: CdnHandle,
        cache: CacheHandle,
        config: &PlaybackConfig,
    ) -> Result<LoadedPlaybackItem, Error> {
        let path = load_audio_path(self.item_id, &session, &cache, &config)?;
        let key = load_audio_key(&path, &session, &cache)?;
        let file = AudioFile::open(path, cdn, cache)?;
        let source = file.audio_source(key)?;
        Ok(LoadedPlaybackItem { file, source })
    }
}

fn load_audio_path(
    item_id: ItemId,
    session: &SessionHandle,
    cache: &CacheHandle,
    config: &PlaybackConfig,
) -> Result<AudioPath, Error> {
    match item_id.id_type {
        ItemIdType::Track => {
            load_audio_path_from_track_or_alternative(item_id, session, cache, config)
        }
        ItemIdType::Podcast => unimplemented!(),
        ItemIdType::Unknown => unimplemented!(),
    }
}

fn load_audio_path_from_track_or_alternative(
    item_id: ItemId,
    session: &SessionHandle,
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

fn get_country_code(session: &SessionHandle, cache: &CacheHandle) -> Option<String> {
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
    session: &SessionHandle,
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
    session: &SessionHandle,
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
}

pub struct Player {
    state: PlayerState,
    preload: PreloadState,
    session: SessionHandle,
    cdn: CdnHandle,
    cache: CacheHandle,
    config: PlaybackConfig,
    queue: Queue,
    event_sender: Sender<PlayerEvent>,
    audio_source: Arc<Mutex<PlayerAudioSource>>,
    audio_output_remote: AudioOutputRemote,
}

impl Player {
    pub fn new(
        session: SessionHandle,
        cdn: CdnHandle,
        cache: CacheHandle,
        config: PlaybackConfig,
        audio_output_remote: AudioOutputRemote,
    ) -> (Self, Receiver<PlayerEvent>) {
        let (event_sender, event_receiver) = unbounded();
        let audio_source = {
            let event_sender = event_sender.clone();
            Arc::new(Mutex::new(PlayerAudioSource::new(event_sender)))
        };
        (
            Self {
                session,
                cdn,
                cache,
                config,
                event_sender,
                audio_source,
                audio_output_remote,
                state: PlayerState::Stopped,
                preload: PreloadState::None,
                queue: Queue::new(),
            },
            event_receiver,
        )
    }

    pub fn audio_source(&self) -> Arc<Mutex<impl AudioSource>> {
        self.audio_source.clone()
    }

    pub fn event_sender(&self) -> Sender<PlayerEvent> {
        self.event_sender.clone()
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
            PlayerEvent::Playing { duration, path } => {
                self.handle_playing(duration, path);
            }
            PlayerEvent::Finished { .. } => {
                self.handle_finished();
            }
            PlayerEvent::Paused { .. } => {}
            PlayerEvent::Started { .. } => {}
            PlayerEvent::Loading { .. } => {}
        };
    }

    fn handle_command(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::LoadQueue { items, position } => self.load_queue(items, position),
            PlayerCommand::LoadAndPlay { item } => self.load_and_play(item),
            PlayerCommand::Preload { item } => self.preload(item),
            PlayerCommand::Pause => self.pause(),
            PlayerCommand::Resume => self.resume(),
            PlayerCommand::Previous => self.previous(),
            PlayerCommand::Next => self.next(),
            PlayerCommand::Stop => self.stop(),
            PlayerCommand::Seek { position } => self.seek(position),
            PlayerCommand::Configure { config } => self.configure(config),
        }
    }

    fn handle_loaded(&mut self, item: PlaybackItem, result: Result<LoadedPlaybackItem, Error>) {
        match self.state {
            PlayerState::Loading {
                item: requested_item,
                ..
            } if item == requested_item => match result {
                Ok(loaded_item) => {
                    self.play_loaded(loaded_item);
                }
                Err(err) => {
                    log::error!("error while opening: {}", err);
                    self.stop();
                }
            },
            _ => {
                log::info!("stale open result received, ignoring");
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

    fn handle_playing(&mut self, progress: Duration, path: AudioPath) {
        const PRELOAD_BEFORE_END_OF_TRACK: Duration = Duration::from_secs(30);

        if let PlayerState::Playing { duration, .. } = &mut self.state {
            *duration = progress;
        }
        if let Some(&item_to_preload) = self.queue.get_next() {
            let time_until_end_of_track = path.duration.checked_sub(progress).unwrap_or_default();
            if time_until_end_of_track <= PRELOAD_BEFORE_END_OF_TRACK {
                self.preload(item_to_preload);
            }
        }
    }

    fn handle_finished(&mut self) {
        self.next();
    }

    fn load_queue(&mut self, items: Vec<PlaybackItem>, position: usize) {
        self.queue.items = items;
        self.queue.position = position;
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
                let result = item.load(session, cdn, cache, &config);
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
            loading_handle,
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
                let result = item.load(session, cdn, cache, &config);
                event_sender
                    .send(PlayerEvent::Preloaded { item, result })
                    .expect("Failed to send PlayerEvent::Preloaded");
            }
        });
        self.preload = PreloadState::Preloading {
            item,
            loading_handle,
        };
    }

    fn play_loaded(&mut self, loaded_item: LoadedPlaybackItem) {
        log::info!("starting playback");
        let path = loaded_item.file.path();
        self.event_sender
            .send(PlayerEvent::Started { path })
            .expect("Failed to send PlayerEvent::Started");
        self.state = PlayerState::Playing {
            path,
            duration: Duration::default(),
        };
        self.audio_source
            .lock()
            .expect("Failed to acquire audio source lock")
            .play_now(loaded_item);
        self.audio_output_remote.resume();
    }

    fn pause(&mut self) {
        match mem::replace(&mut self.state, PlayerState::Invalid) {
            PlayerState::Playing { path, duration } | PlayerState::Paused { path, duration } => {
                log::info!("pausing playback");
                self.event_sender
                    .send(PlayerEvent::Paused { path })
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
                self.state = PlayerState::Playing { path, duration };
                self.audio_output_remote.resume();
            }
            _ => {
                log::warn!("invalid state transition");
            }
        }
    }

    fn previous(&mut self) {
        if self.is_near_playback_start() {
            self.queue.previous();
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
        self.queue.next();
        if let Some(&item) = self.queue.get_current() {
            self.load_and_play(item);
        } else {
            self.stop();
        }
    }

    fn stop(&mut self) {
        self.state = PlayerState::Stopped;
        self.audio_output_remote.pause();
        self.queue.clear();
    }

    fn seek(&mut self, position: Duration) {
        // TODO: Clear audio output buffer.
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
    Previous,
    Next,
    Stop,
    Seek {
        position: Duration,
    },
    Configure {
        config: PlaybackConfig,
    },
}

pub enum PlayerEvent {
    Command(PlayerCommand),
    Loading {
        item: PlaybackItem,
    },
    Loaded {
        item: PlaybackItem,
        result: Result<LoadedPlaybackItem, Error>,
    },
    Preloaded {
        item: PlaybackItem,
        result: Result<LoadedPlaybackItem, Error>,
    },
    Started {
        path: AudioPath,
    },
    Playing {
        path: AudioPath,
        duration: Duration,
    },
    Paused {
        path: AudioPath,
    },
    Finished,
}

enum PlayerState {
    Loading {
        item: PlaybackItem,
        loading_handle: JoinHandle<()>,
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
        loading_handle: JoinHandle<()>,
    },
    Preloaded {
        item: PlaybackItem,
        loaded_item: LoadedPlaybackItem,
    },
    None,
}

const OUTPUT_CHANNELS: u8 = 2;
const OUTPUT_SAMPLE_RATE: u32 = 44100;
const PROGRESS_PRECISION_SAMPLES: u64 = (OUTPUT_SAMPLE_RATE / 10) as u64;

struct PlayerAudioSource {
    current: Option<LoadedPlaybackItem>,
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
            let pos_secs = position.as_secs_f64();
            let pcm_frame = pos_secs * OUTPUT_SAMPLE_RATE as f64;
            let samples = pcm_frame * OUTPUT_CHANNELS as f64;
            current.source.seek(pcm_frame as u64);
            self.samples = samples as u64;
        }
    }

    fn play_now(&mut self, item: LoadedPlaybackItem) {
        self.current.replace(item);
        self.samples = 0;
    }
}

impl AudioSource for PlayerAudioSource {
    fn channels(&self) -> u8 {
        OUTPUT_CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        OUTPUT_SAMPLE_RATE
    }
}

impl Iterator for PlayerAudioSource {
    type Item = AudioSample;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO:
        //  We could move all this into the decoder struct, use position information
        //  from the decoding and do all this work only per-packet, not per-sample.
        if let Some(current) = &mut self.current {
            let sample = current.source.next();
            if sample.is_some() {
                // We report playback progress every `PROGRESS_PRECISION_SAMPLES`th sample.
                if self.samples % PROGRESS_PRECISION_SAMPLES == 0 {
                    self.event_sender
                        .send(PlayerEvent::Playing {
                            duration: Duration::from_secs_f64(
                                self.samples as f64
                                    / OUTPUT_SAMPLE_RATE as f64
                                    / OUTPUT_CHANNELS as f64,
                            ),
                            path: current.file.path(),
                        })
                        .expect("Failed to send PlayerEvent::Playing");
                }
                self.samples += 1;
                sample
            } else {
                // Current source ended, report audio end.
                self.event_sender
                    .send(PlayerEvent::Finished)
                    .expect("Failed to send PlayerEvent::Finished");
                self.current.take();
                self.samples = 0;
                None
            }
        } else {
            None
        }
    }
}

struct Queue {
    items: Vec<PlaybackItem>,
    position: usize,
}

impl Queue {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            position: 0,
        }
    }

    fn clear(&mut self) {
        self.position = 0;
        self.items.clear();
    }

    fn previous(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    fn next(&mut self) {
        self.position += 1;
    }

    fn get_current(&self) -> Option<&PlaybackItem> {
        self.items.get(self.position)
    }

    fn get_next(&self) -> Option<&PlaybackItem> {
        self.items.get(self.position + 1)
    }
}
