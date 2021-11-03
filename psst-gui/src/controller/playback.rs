use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

use crossbeam_channel::Sender;
use druid::{
    im::Vector,
    widget::{prelude::*, Controller},
    Code, ExtEventSink, InternalLifeCycle, KbKey, WindowHandle,
};
use psst_core::{
    audio::{normalize::NormalizationLevel, output::AudioOutput},
    cache::Cache,
    cdn::Cdn,
    player::{item::PlaybackItem, PlaybackConfig, Player, PlayerCommand, PlayerEvent},
    session::SessionService,
};
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};

use crate::{
    cmd,
    data::{
        AppState, Config, Playback, PlaybackOrigin, PlaybackState, QueueBehavior, QueuedTrack,
        TrackId,
    },
};

pub struct PlaybackController {
    sender: Option<Sender<PlayerEvent>>,
    thread: Option<JoinHandle<()>>,
    output: Option<AudioOutput>,
    media_controls: Option<MediaControls>,
}

impl PlaybackController {
    pub fn new() -> Self {
        Self {
            sender: None,
            thread: None,
            output: None,
            media_controls: None,
        }
    }

    fn open_audio_output_and_start_threads(
        &mut self,
        session: SessionService,
        config: PlaybackConfig,
        event_sink: ExtEventSink,
        widget_id: WidgetId,
        #[allow(unused_variables)] window: &WindowHandle,
    ) {
        let output = AudioOutput::open().unwrap();
        let cache_dir = Config::cache_dir().unwrap();
        let proxy_url = Config::proxy();
        let player = Player::new(
            session.clone(),
            Cdn::new(session, proxy_url.as_deref()).unwrap(),
            Cache::new(cache_dir).unwrap(),
            config,
            &output,
        );
        let sender = player.sender();

        let thread = thread::spawn(move || {
            Self::service_events(player, event_sink, widget_id);
        });

        let hwnd = {
            #[cfg(target_os = "windows")]
            {
                use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
                let handle = match window.raw_window_handle() {
                    RawWindowHandle::Windows(h) => h,
                    _ => unreachable!(),
                };
                Some(handle.hwnd)
            }
            #[cfg(not(target_os = "windows"))]
            None
        };

        let config = PlatformConfig {
            dbus_name: "psst",
            display_name: "Psst",
            hwnd,
        };
        let mut media_controls = MediaControls::new(config).unwrap();
        media_controls
            .attach({
                let sender = sender.clone();
                move |event| {
                    Self::handle_media_control_event(event, &sender);
                }
            })
            .unwrap();

        self.sender.replace(sender);
        self.thread.replace(thread);
        self.output.replace(output);
        self.media_controls.replace(media_controls);
    }

    fn service_events(mut player: Player, event_sink: ExtEventSink, widget_id: WidgetId) {
        for event in player.receiver() {
            // Forward events that affect the UI state to the UI thread.
            match &event {
                PlayerEvent::Loading { item } => {
                    let item: TrackId = item.item_id.into();
                    event_sink
                        .submit_command(cmd::PLAYBACK_LOADING, item, widget_id)
                        .unwrap();
                }
                PlayerEvent::Playing { path, position } => {
                    let item: TrackId = path.item_id.into();
                    let progress = position.to_owned();
                    event_sink
                        .submit_command(cmd::PLAYBACK_PLAYING, (item, progress), widget_id)
                        .unwrap();
                }
                PlayerEvent::Pausing { .. } => {
                    event_sink
                        .submit_command(cmd::PLAYBACK_PAUSING, (), widget_id)
                        .unwrap();
                }
                PlayerEvent::Resuming { .. } => {
                    event_sink
                        .submit_command(cmd::PLAYBACK_RESUMING, (), widget_id)
                        .unwrap();
                }
                PlayerEvent::Position { position, .. } => {
                    let progress = position.to_owned();
                    event_sink
                        .submit_command(cmd::PLAYBACK_PROGRESS, progress, widget_id)
                        .unwrap();
                }
                PlayerEvent::Blocked { .. } => {
                    event_sink
                        .submit_command(cmd::PLAYBACK_BLOCKED, (), widget_id)
                        .unwrap();
                }
                PlayerEvent::Stopped => {
                    event_sink
                        .submit_command(cmd::PLAYBACK_STOPPED, (), widget_id)
                        .unwrap();
                }
                _ => {}
            }

            // Let the player react to its internal events.
            player.handle(event);
        }
    }

    fn handle_media_control_event(event: MediaControlEvent, sender: &Sender<PlayerEvent>) {
        let cmd = match event {
            MediaControlEvent::Play => PlayerEvent::Command(PlayerCommand::Resume),
            MediaControlEvent::Pause => PlayerEvent::Command(PlayerCommand::Pause),
            MediaControlEvent::Toggle => PlayerEvent::Command(PlayerCommand::PauseOrResume),
            MediaControlEvent::Next => PlayerEvent::Command(PlayerCommand::Next),
            MediaControlEvent::Previous => PlayerEvent::Command(PlayerCommand::Previous),
            _ => {
                return;
            }
        };
        sender.send(cmd).unwrap();
    }

    fn update_media_control_playback(&mut self, playback: &Playback) {
        if let Some(media_controls) = self.media_controls.as_mut() {
            let progress = playback
                .now_playing
                .as_ref()
                .map(|now_playing| MediaPosition(now_playing.progress));
            media_controls
                .set_playback(match playback.state {
                    PlaybackState::Loading | PlaybackState::Stopped => MediaPlayback::Stopped,
                    PlaybackState::Playing => MediaPlayback::Playing { progress },
                    PlaybackState::Paused => MediaPlayback::Paused { progress },
                })
                .unwrap();
        }
    }

    fn update_media_control_metadata(&mut self, playback: &Playback) {
        if let Some(media_controls) = self.media_controls.as_mut() {
            let title = playback.now_playing.as_ref().map(|p| p.item.name.clone());
            let album = playback.now_playing.as_ref().map(|p| p.item.album_name());
            let artist = playback.now_playing.as_ref().map(|p| p.item.artist_name());
            let duration = playback.now_playing.as_ref().map(|p| p.item.duration);
            let cover_url = playback
                .now_playing
                .as_ref()
                .and_then(|p| p.cover_image_url(512.0, 512.0));
            media_controls
                .set_metadata(MediaMetadata {
                    title: title.as_deref(),
                    album: album.as_deref(),
                    artist: artist.as_deref(),
                    duration,
                    cover_url,
                })
                .unwrap();
        }
    }

    fn send(&mut self, event: PlayerEvent) {
        self.sender.as_mut().unwrap().send(event).unwrap();
    }

    fn play(&mut self, items: &Vector<QueuedTrack>, position: usize) {
        let items = items
            .iter()
            .map(|queued| PlaybackItem {
                item_id: *queued.track.id,
                norm_level: match queued.origin {
                    PlaybackOrigin::Album(_) => NormalizationLevel::Album,
                    _ => NormalizationLevel::Track,
                },
            })
            .collect();
        self.send(PlayerEvent::Command(PlayerCommand::LoadQueue {
            items,
            position,
        }));
    }

    fn pause(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Pause));
    }

    fn resume(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Resume));
    }

    fn pause_or_resume(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::PauseOrResume));
    }

    fn previous(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Previous));
    }

    fn next(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Next));
    }

    fn stop(&mut self) {
        self.send(PlayerEvent::Command(PlayerCommand::Stop));
    }

    fn seek(&mut self, position: Duration) {
        self.send(PlayerEvent::Command(PlayerCommand::Seek { position }));
    }

    fn set_volume(&mut self, volume: f64) {
        self.send(PlayerEvent::Command(PlayerCommand::SetVolume { volume }));
    }

    fn set_queue_behavior(&mut self, behavior: QueueBehavior) {
        self.send(PlayerEvent::Command(PlayerCommand::SetQueueBehavior {
            behavior: match behavior {
                QueueBehavior::Sequential => psst_core::player::queue::QueueBehavior::Sequential,
                QueueBehavior::Random => psst_core::player::queue::QueueBehavior::Random,
                QueueBehavior::LoopTrack => psst_core::player::queue::QueueBehavior::LoopTrack,
                QueueBehavior::LoopAll => psst_core::player::queue::QueueBehavior::LoopAll,
            },
        }));
    }
}

impl<W> Controller<AppState, W> for PlaybackController
where
    W: Widget<AppState>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(cmd::SET_FOCUS) => {
                ctx.request_focus();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_LOADING) => {
                let item = cmd.get_unchecked(cmd::PLAYBACK_LOADING);

                if let Some(queued) = data.queued_track(item) {
                    data.loading_playback(queued.track, queued.origin);
                    self.update_media_control_playback(&data.playback);
                    self.update_media_control_metadata(&data.playback);
                } else {
                    log::warn!("loaded item not found in playback queue");
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_PLAYING) => {
                let (item, progress) = cmd.get_unchecked(cmd::PLAYBACK_PLAYING);
                log::info!("playing");

                if let Some(queued) = data.queued_track(item) {
                    data.start_playback(queued.track, queued.origin, progress.to_owned());
                    self.update_media_control_playback(&data.playback);
                    self.update_media_control_metadata(&data.playback);
                } else {
                    log::warn!("played item not found in playback queue");
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_PROGRESS) => {
                let progress = cmd.get_unchecked(cmd::PLAYBACK_PROGRESS);
                data.progress_playback(progress.to_owned());
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_PAUSING) => {
                data.pause_playback();
                self.update_media_control_playback(&data.playback);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_RESUMING) => {
                data.resume_playback();
                self.update_media_control_playback(&data.playback);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_BLOCKED) => {
                data.block_playback();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAYBACK_STOPPED) => {
                data.stop_playback();
                self.update_media_control_playback(&data.playback);
                ctx.set_handled();
            }
            //
            Event::Command(cmd) if cmd.is(cmd::PLAY_TRACKS) => {
                let payload = cmd.get_unchecked(cmd::PLAY_TRACKS);
                data.playback.queue = payload
                    .tracks
                    .iter()
                    .map(|track| QueuedTrack {
                        origin: payload.origin.to_owned(),
                        track: track.to_owned(),
                    })
                    .collect();
                self.play(&data.playback.queue, payload.position);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_PAUSE) => {
                self.pause();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_RESUME) => {
                self.resume();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_PREVIOUS) => {
                self.previous();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_NEXT) => {
                self.next();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_STOP) => {
                self.stop();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_QUEUE_BEHAVIOR) => {
                let behavior = cmd.get_unchecked(cmd::PLAY_QUEUE_BEHAVIOR);
                data.set_queue_behavior(behavior.to_owned());
                self.set_queue_behavior(behavior.to_owned());
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::PLAY_SEEK) => {
                if let Some(now_playing) = &data.playback.now_playing {
                    let fraction = cmd.get_unchecked(cmd::PLAY_SEEK);
                    let position =
                        Duration::from_secs_f64(now_playing.item.duration.as_secs_f64() * fraction);
                    self.seek(position);
                }
                ctx.set_handled();
            }
            //
            Event::KeyDown(key) if key.code == Code::Space => {
                self.pause_or_resume();
                ctx.set_handled();
            }
            Event::KeyDown(key) if key.code == Code::ArrowRight => {
                self.next();
                ctx.set_handled();
            }
            Event::KeyDown(key) if key.code == Code::ArrowLeft => {
                self.previous();
                ctx.set_handled();
            }
            Event::KeyDown(key) if key.key == KbKey::Character("+".to_string()) => {
                data.playback.volume = (data.playback.volume + 0.1).min(1.0);
                ctx.set_handled();
            }
            Event::KeyDown(key) if key.key == KbKey::Character("-".to_string()) => {
                data.playback.volume = (data.playback.volume - 0.1).max(0.0);
                ctx.set_handled();
            }
            //
            _ => child.event(ctx, event, data, env),
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &AppState,
        env: &Env,
    ) {
        match event {
            LifeCycle::WidgetAdded => {
                self.open_audio_output_and_start_threads(
                    data.session.clone(),
                    data.config.playback(),
                    ctx.get_external_handle(),
                    ctx.widget_id(),
                    ctx.window(),
                );
                self.set_volume(data.playback.volume);

                // Request focus so we can receive keyboard events.
                ctx.submit_command(cmd::SET_FOCUS.to(ctx.widget_id()));
            }
            LifeCycle::Internal(InternalLifeCycle::RouteFocusChanged { new: None, .. }) => {
                // Druid doesn't have any "ambient focus" concept, so we catch the situation
                // when the focus is being lost and sign up to get focused ourselves.
                ctx.submit_command(cmd::SET_FOCUS.to(ctx.widget_id()));
            }
            _ => {}
        }
        child.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &Env,
    ) {
        if !old_data.playback.volume.same(&data.playback.volume) {
            self.set_volume(data.playback.volume);
        }
        child.update(ctx, old_data, data, env);
    }
}
