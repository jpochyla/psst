use super::{theme, utils};
use crate::{
    cmd,
    data::{
        connect::{Device, RemoteRequest},
        AppState, Nav, Playable, PlaybackOrigin, PlaybackPayload, PlaybackState, QueueBehavior,
    },
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};
use druid::{
    widget::{Button, Controller, CrossAxisAlignment, Flex, Label, LineBreaking, List, Scroll},
    Data, Env, Event, EventCtx, LensExt, LifeCycle, LifeCycleCtx, Selector, TimerToken, UpdateCtx,
    Widget, WidgetExt,
};
use serde_json::json;
use std::time::{Duration, Instant};

pub const LOAD: Selector = Selector::new("connect.load");
pub const SELECT: Selector<Option<Device>> = Selector::new("connect.select");
pub const ACTION: Selector<RemoteRequest> = Selector::new("connect.action");
pub const QUEUE: Selector<(String, u64)> = Selector::new("connect.queue");
pub const STATUS: Selector<(String, u64)> = Selector::new("connect.status");

pub fn widget() -> impl Widget<AppState> {
    let devices = Async::new(
        utils::spinner_widget,
        || {
            List::new(|| {
                Button::dynamic(|device: &Device, _| {
                    format!(
                        "{} · {}{}",
                        device.name,
                        device.kind,
                        if device.is_active { " · Activo" } else { "" }
                    )
                })
                .on_click(|ctx, device: &mut Device, _| {
                    ctx.submit_command(SELECT.with(Some(device.clone())))
                })
                .disabled_if(|device: &Device, _| device.id.is_none() || device.is_restricted)
                .expand_width()
                .fix_height(48.0)
                .padding((0.0, 4.0))
            })
        },
        utils::error_widget,
    )
    .lens(AppState::connect.then(crate::data::connect::ConnectState::devices));
    Scroll::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("¿Dónde quieres escuchar?").with_font(theme::UI_FONT_MEDIUM).with_text_size(24.0))
        .with_spacer(12.0)
        .with_child(Label::new("Abre Spotify en tu teléfono, altavoz u otro equipo y conéctalo a tu cuenta para que aparezca aquí.")
            .with_line_break_mode(LineBreaking::WordWrap).expand_width())
        .with_spacer(16.0)
        .with_child(Label::new("El reproductor nativo de Xpotify todavía no se anuncia como receptor de Spotify Connect. Para controlar esta PC desde el teléfono, abre Spotify oficial aquí y selecciona ese equipo en la lista.")
            .with_line_break_mode(LineBreaking::WordWrap).with_text_color(theme::PLACEHOLDER_COLOR).expand_width())
        .with_spacer(12.0)
        .with_child(Button::new("Abrir Spotify en esta PC").on_click(|_, state: &mut AppState, _| {
            match open::that_detached("spotify:") {
                Ok(()) => state.connect.status = "Mantén Spotify abierto con la misma cuenta y actualiza los dispositivos para seleccionar esta PC.".into(),
                Err(_) => state.error_alert("No se pudo abrir Spotify. Instala o abre el cliente oficial en esta PC para usarlo como receptor Connect."),
            }
        }).tooltip("Usar el cliente oficial como receptor controlable desde el teléfono"))
        .with_spacer(16.0)
        .with_child(Button::new("Este equipo · Xpotify").on_click(|ctx, _: &mut AppState, _| ctx.submit_command(SELECT.with(None)))
            .expand_width().fix_height(48.0))
        .with_spacer(12.0)
        .with_child(Label::dynamic(|state: &AppState, _| state.connect.selected.as_ref()
            .map(|device| format!("Seleccionado: {}", device.name)).unwrap_or_else(|| "Seleccionado: este equipo".into()))
            .with_text_color(theme::PLACEHOLDER_COLOR))
        .with_spacer(8.0)
        .with_child(Label::dynamic(|state: &AppState, _| state.connect.status.clone()).with_line_break_mode(LineBreaking::WordWrap).expand_width())
        .with_spacer(16.0).with_child(Button::new("Actualizar dispositivos").on_click(|ctx, _: &mut AppState, _| ctx.submit_command(LOAD)))
        .with_spacer(16.0).with_child(devices)
        .disabled_if(|state: &AppState, _| state.connect.busy)
        .padding(24.0).expand_width()).vertical()
}

/// Request handlers live on the root, so changing routes never loses a device response.
pub fn controller(widget: impl Widget<AppState> + 'static) -> impl Widget<AppState> {
    widget.controller(ConnectController::default())
        .on_command_async(LOAD, |_| WebApi::global().get_devices(),
            |_, state, _| state.connect.devices.defer(()),
            |_, state, result| state.connect.devices.update(result))
        .on_command_async(ACTION, |request| {
            // Serialize mutations; each transfer completes before its play request.
            static LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
            let _lock = LOCK.lock();
            WebApi::global().remote_action(&request)
        }, |_, state, request| {
            if request.epoch == state.connect.epoch && state.connect.selected.is_some() { state.connect.busy = true; }
        },
        |ctx, state, (request, result)| {
            if request.epoch != state.connect.epoch { return; }
            state.connect.busy = false;
            match result {
                Ok(()) => {
                    state.connect.status = "Dispositivo conectado".into();
                    if matches!(request.kind.as_str(), "play" | "resume" | "switch-play") { state.connect.pending_start = false; }
                    if request.kind == "pause" { state.playback.state = PlaybackState::Paused; }
                    request_status(ctx, state);
                }
                Err(error) => {
                    state.connect.status = format!("{error}. Si Spotify indica permisos insuficientes, vuelve a conectar tu cuenta en Ajustes.");
                    if request.kind == "transfer" || request.kind == "switch-play" {
                        state.connect.selected = None;
                        state.connect.pending_start = false;
                        if let Some(volume) = state.connect.local_volume.take() { state.playback.volume = volume; }
                        ctx.submit_command(cmd::RESTORE_LOCAL_PLAYBACK);
                    }
                    state.error_alert(state.connect.status.clone());
                }
            }
        })
        .on_command_async(STATUS, |_| WebApi::global().remote_playback(),
            |_, state, _| state.connect.polling = true,
            |ctx, state, ((device_id, epoch), result)| {
                state.connect.polling = false;
                if epoch != state.connect.epoch || state.connect.selected.as_ref().and_then(|d| d.id.as_ref()) != Some(&device_id) { return; }
                if state.connect.pending_start && result.is_ok() {
                    state.connect.status = format!("Conectado a {}. Pulsa Reproducir para continuar.", state.connect.selected.as_ref().unwrap().name);
                    return;
                }
                match result {
                    Ok(Some(remote)) if remote.device.id.as_ref() == Some(&device_id) => {
                        if state.connect.pending_start {
                            state.connect.status = format!("Conectado a {}. Pulsa Reproducir para continuar.", remote.device.name);
                            return;
                        }
                        state.connect.status = format!("Escuchando en {}", remote.device.name);
                        if let Some(track) = remote.item {
                            let changed = state.playback.now_playing.as_ref().is_none_or(|np| np.item.id() != track.id.0);
                            let origin = if state.playback.queue.iter().any(|entry| entry.item.id() == track.id.0) { state.playback.now_playing.as_ref().map(|np| np.origin.clone()).unwrap_or(PlaybackOrigin::Home) } else { PlaybackOrigin::Home };
                            state.start_playback(Playable::Track(track), origin, Duration::from_millis(remote.progress_ms.unwrap_or(0)));
                            state.playback.state = if remote.is_playing { PlaybackState::Playing } else { PlaybackState::Paused };
                            if changed || state.nav == Nav::Queue { ctx.submit_command(QUEUE.with((device_id.clone(), epoch))); }
                            if changed && state.nav == Nav::Lyrics {
                                ctx.submit_command(super::lyrics::SHOW_LYRICS.with(state.playback.now_playing.clone().unwrap()));
                            }
                        }
                    }
                    Ok(_) => { state.connect.status = "Spotify no está reproduciendo en el dispositivo seleccionado".into(); state.playback.state = PlaybackState::Paused; }
                    Err(error) => state.connect.status = format!("No se pudo actualizar el dispositivo: {error}"),
                }
            })
        .on_command_async(QUEUE, |_| WebApi::global().remote_queue(), |_, _, _| {},
            |_, state, ((device_id, epoch), result)| {
                if epoch != state.connect.epoch || state.connect.selected.as_ref().and_then(|d| d.id.as_ref()) != Some(&device_id) { return; }
                if let Ok(items) = result { state.playback.up_next = items.into_iter().map(|item| crate::data::QueueEntry { item, origin: PlaybackOrigin::Home }).collect(); }
            })
}

fn request_status(ctx: &mut EventCtx, state: &AppState) {
    if state.connect.selected.is_none() {
        return;
    }
    if WebApi::global().rate_limit_error().is_some() {
        return;
    }
    if !state.connect.polling {
        if let Some(id) = state
            .connect
            .selected
            .as_ref()
            .and_then(|device| device.id.clone())
        {
            ctx.submit_command(STATUS.with((id, state.connect.epoch)));
        }
    }
}

fn action(state: &AppState, kind: &str, body: String, value: String) -> Option<RemoteRequest> {
    Some(RemoteRequest {
        device_id: state.connect.selected.as_ref()?.id.clone()?,
        epoch: state.connect.epoch,
        kind: kind.into(),
        body,
        value,
    })
}

pub fn playback_body(payload: &PlaybackPayload, progress: u64) -> Result<String, String> {
    let current = payload
        .items
        .get(payload.position)
        .ok_or("No hay una canción seleccionada")?;
    let uri = current
        .id()
        .to_uri()
        .ok_or("Esta canción no tiene un identificador de Spotify")?;
    let context = match &payload.origin {
        PlaybackOrigin::Playlist(link) => Some(format!("spotify:playlist:{}", link.id)),
        PlaybackOrigin::Album(link) => Some(format!("spotify:album:{}", link.id)),
        _ => None,
    };
    Ok(if let Some(context) = context {
        json!({"context_uri":context,"offset":{"uri":uri},"position_ms":progress}).to_string()
    } else {
        let uris: Vec<_> = payload
            .items
            .iter()
            .skip(payload.position)
            .filter_map(|item| item.id().to_uri())
            .take(100)
            .collect();
        json!({"uris":uris,"offset":{"position":0},"position_ms":progress}).to_string()
    })
}

fn current_payload(state: &AppState) -> Option<(PlaybackPayload, u64)> {
    let np = state.playback.now_playing.as_ref()?;
    let position = state
        .playback
        .queue
        .iter()
        .position(|entry| entry.item.id() == np.item.id());
    let (items, position) = if let Some(position) = position {
        (
            state
                .playback
                .queue
                .iter()
                .map(|entry| entry.item.clone())
                .collect(),
            position,
        )
    } else {
        (druid::im::vector![np.item.clone()], 0)
    };
    Some((
        PlaybackPayload {
            origin: np.origin.clone(),
            items,
            position,
        },
        np.progress.as_millis() as u64,
    ))
}

struct ConnectController {
    timer: TimerToken,
    volume_timer: TimerToken,
    next_poll: Instant,
    last_tick: Instant,
}

impl Default for ConnectController {
    fn default() -> Self {
        Self {
            timer: TimerToken::INVALID,
            volume_timer: TimerToken::INVALID,
            next_poll: Instant::now(),
            last_tick: Instant::now(),
        }
    }
}

impl<W: Widget<AppState>> Controller<AppState, W> for ConnectController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        state: &mut AppState,
        env: &Env,
    ) {
        if let Event::Timer(token) = event {
            if *token == self.timer {
                if state.connect.selected.is_some()
                    && state.playback.state == PlaybackState::Playing
                {
                    if let Some(np) = &mut state.playback.now_playing {
                        np.progress = (np.progress
                            + self.last_tick.elapsed().min(Duration::from_secs(2)))
                        .min(np.item.duration());
                    }
                }
                self.last_tick = Instant::now();
                if !state.connect.busy && Instant::now() >= self.next_poll {
                    request_status(ctx, state);
                    self.next_poll = Instant::now()
                        + Duration::from_secs(if state.playback.state == PlaybackState::Playing {
                            15
                        } else {
                            60
                        });
                }
                self.timer = ctx.request_timer(Duration::from_secs(1));
            } else if *token == self.volume_timer {
                self.volume_timer = TimerToken::INVALID;
                if let Some(request) = action(
                    state,
                    "volume",
                    String::new(),
                    ((state.playback.volume * 100.0).round() as u8).to_string(),
                ) {
                    ctx.submit_command(ACTION.with(request));
                }
            }
        }
        if let Event::Command(command) = event {
            if let Some(selected) = command.get(SELECT) {
                if state.connect.busy {
                    ctx.set_handled();
                    return;
                }
                let was_playing = state.playback.state == PlaybackState::Playing;
                let payload = current_payload(state);
                if selected.is_none() {
                    if let Some(request) = action(state, "pause", String::new(), String::new()) {
                        ctx.submit_command(ACTION.with(request));
                    }
                    state.connect.epoch += 1;
                    state.connect.selected = None;
                    state.connect.pending_start = false;
                    if let Some(volume) = state.connect.local_volume.take() {
                        state.playback.volume = volume;
                    }
                    state.connect.busy = false;
                    state.connect.status =
                        "Listo en este equipo. Pulsa Reproducir para continuar.".into();
                    state.capture_resume();
                    ctx.submit_command(cmd::RESTORE_LOCAL_PLAYBACK);
                } else {
                    if state.connect.selected.is_none() {
                        state.connect.local_volume = Some(state.playback.volume);
                    }
                    state.connect.epoch += 1;
                    state.connect.selected = selected.clone();
                    if let Some(volume) = selected.as_ref().and_then(|device| device.volume_percent)
                    {
                        state.playback.volume = volume.min(100) as f64 / 100.0;
                    }
                    state.connect.status = "Conectando…".into();
                    ctx.submit_command(cmd::SUSPEND_LOCAL_PLAYBACK);
                    let body = payload
                        .and_then(|(payload, progress)| playback_body(&payload, progress).ok())
                        .unwrap_or_default();
                    state.connect.pending_start = !was_playing && !body.is_empty();
                    state.playback.up_next.clear();
                    if let Some(id) = state
                        .connect
                        .selected
                        .as_ref()
                        .and_then(|device| device.id.clone())
                    {
                        ctx.submit_command(QUEUE.with((id, state.connect.epoch)));
                    }
                    if let Some(request) = action(
                        state,
                        if was_playing && !body.is_empty() {
                            "switch-play"
                        } else {
                            "transfer"
                        },
                        body,
                        String::new(),
                    ) {
                        ctx.submit_command(ACTION.with(request));
                    }
                }
                state.playback.state = PlaybackState::Paused;
                ctx.set_handled();
                return;
            }
            if state.connect.selected.is_some() {
                if let Some((index, expected)) = command.get(cmd::PLAY_UPCOMING) {
                    if state
                        .playback
                        .up_next
                        .get(*index)
                        .is_some_and(|entry| entry.item.id() == *expected)
                    {
                        ctx.submit_command(
                            cmd::PLAY_TRACKS.with(PlaybackPayload {
                                origin: state
                                    .playback
                                    .now_playing
                                    .as_ref()
                                    .map(|np| np.origin.clone())
                                    .unwrap_or(PlaybackOrigin::Home),
                                items: state
                                    .playback
                                    .up_next
                                    .iter()
                                    .map(|entry| entry.item.clone())
                                    .collect(),
                                position: *index,
                            }),
                        );
                    }
                    ctx.set_handled();
                    return;
                }
                if command.is(cmd::PLAYBACK_LOADING)
                    || command.is(cmd::PLAYBACK_PLAYING)
                    || command.is(cmd::PLAYBACK_PROGRESS)
                    || command.is(cmd::PLAYBACK_PAUSING)
                    || command.is(cmd::PLAYBACK_RESUMING)
                    || command.is(cmd::PLAYBACK_STOPPED)
                    || command.is(cmd::QUEUE_CHANGED)
                    || command.is(cmd::PLAYBACK_BLOCKED)
                {
                    ctx.set_handled();
                    return;
                }
                if let Some(track) = command.get(cmd::OPEN_MUSIC_VIDEO) {
                    match open::that_detached(super::video::search_url(track)) {
                        Ok(()) if state.playback.state == PlaybackState::Playing => {
                            ctx.submit_command(cmd::PLAY_PAUSE)
                        }
                        Ok(()) => {}
                        Err(_) => state
                            .error_alert("No se pudo abrir el navegador para buscar el videoclip."),
                    }
                    ctx.set_handled();
                    return;
                }
                if state.connect.busy
                    && (command.is(cmd::PLAY_TRACKS)
                        || command.is(cmd::PLAY_RESUME)
                        || command.is(cmd::PLAY_QUEUE_BEHAVIOR))
                {
                    ctx.set_handled();
                    return;
                }
                let mut kind = None;
                let mut body = String::new();
                let mut value = String::new();
                if let Some(payload) = command.get(cmd::PLAY_TRACKS) {
                    match playback_body(payload, 0) {
                        Ok(json) => body = json,
                        Err(error) => {
                            state.error_alert(error);
                            ctx.set_handled();
                            return;
                        }
                    }
                    state.playback.queue = payload
                        .items
                        .iter()
                        .map(|item| crate::data::QueueEntry {
                            item: item.clone(),
                            origin: payload.origin.clone(),
                        })
                        .collect();
                    if let Some(item) = payload.items.get(payload.position) {
                        state.start_playback(item.clone(), payload.origin.clone(), Duration::ZERO);
                        state.playback.state = PlaybackState::Loading;
                    }
                    kind = Some("play");
                } else if command.is(cmd::PLAY_TOGGLE) {
                    ctx.submit_command(if state.playback.state == PlaybackState::Playing {
                        cmd::PLAY_PAUSE
                    } else {
                        cmd::PLAY_RESUME
                    });
                    ctx.set_handled();
                    return;
                } else if command.is(cmd::PLAY_RESUME) {
                    if state.connect.pending_start {
                        if let Some((payload, progress)) = current_payload(state) {
                            body = playback_body(&payload, progress).unwrap_or_default();
                        }
                    }
                    kind = Some(if body.is_empty() { "resume" } else { "play" });
                } else if command.is(cmd::PLAY_PAUSE) || command.is(cmd::PLAY_STOP) {
                    kind = Some("pause");
                } else if command.is(cmd::PLAY_NEXT) {
                    kind = Some("next");
                } else if command.is(cmd::PLAY_PREVIOUS) {
                    kind = Some("previous");
                } else if let Some(position) = command.get(cmd::SKIP_TO_POSITION) {
                    kind = Some("seek");
                    value = position.to_string();
                } else if let Some(fraction) = command.get(cmd::PLAY_SEEK) {
                    if let Some(np) = &state.playback.now_playing {
                        kind = Some("seek");
                        value = ((np.item.duration().as_millis() as f64 * fraction.clamp(0.0, 1.0))
                            as u64)
                            .to_string();
                    }
                } else if let Some((_, item)) = command.get(cmd::ADD_TO_QUEUE) {
                    kind = Some("queue");
                    value = item.item_id.to_uri().unwrap_or_default();
                } else if let Some(behavior) = command.get(cmd::PLAY_QUEUE_BEHAVIOR) {
                    state.playback.queue_behavior = *behavior;
                    body = (*behavior == QueueBehavior::Random).to_string();
                    kind = Some("behavior");
                    value = match behavior {
                        QueueBehavior::LoopTrack => "track",
                        QueueBehavior::LoopAll => "context",
                        _ => "off",
                    }
                    .into();
                }
                if let Some(kind) = kind {
                    if state.connect.busy && kind != "pause" {
                        ctx.set_handled();
                        return;
                    }
                    if let Some(request) = action(state, kind, body, value) {
                        ctx.submit_command(ACTION.with(request));
                    }
                    ctx.set_handled();
                    return;
                }
            }
        }
        child.event(ctx, event, state, env);
    }
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        state: &AppState,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) {
            self.timer = ctx.request_timer(Duration::from_secs(1));
        }
        child.lifecycle(ctx, event, state, env);
    }
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old: &AppState,
        state: &AppState,
        env: &Env,
    ) {
        if state.connect.selected.is_some()
            && old.connect.selected.as_ref().and_then(|d| d.id.as_ref())
                == state.connect.selected.as_ref().and_then(|d| d.id.as_ref())
            && !old.playback.volume.same(&state.playback.volume)
            && self.volume_timer == TimerToken::INVALID
        {
            self.volume_timer = ctx.request_timer(Duration::from_millis(500));
        }
        child.update(ctx, old, state, env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_play_preserves_playlist_track_and_position() {
        let track: crate::data::Track = serde_json::from_value(json!({
            "id":"5lfWrciYtohtIMVDVZd0Rf", "name":"Example", "artists":[],
            "duration_ms":240000,"disc_number":1,"track_number":1,"explicit":false,"is_local":false
        }))
        .unwrap();
        let payload = PlaybackPayload {
            items: druid::im::vector![Playable::Track(std::sync::Arc::new(track))],
            position: 0,
            origin: PlaybackOrigin::Playlist(crate::data::PlaylistLink {
                id: "example".into(),
                name: "Example".into(),
            }),
        };
        let value: serde_json::Value =
            serde_json::from_str(&playback_body(&payload, 26000).unwrap()).unwrap();
        assert_eq!(value["context_uri"], "spotify:playlist:example");
        assert_eq!(
            value["offset"]["uri"],
            "spotify:track:5lfWrciYtohtIMVDVZd0Rf"
        );
        assert_eq!(value["position_ms"], 26000);
        let invalid = PlaybackPayload {
            position: 1,
            ..payload
        };
        assert!(playback_body(&invalid, 0).is_err());
    }
}
