use druid::{
    widget::{Button, CrossAxisAlignment, Flex, Label, LineBreaking, List},
    Env, LensExt, Selector, Widget, WidgetExt,
};

use super::{theme, utils};
use crate::{
    cmd,
    data::{AppState, Ctx, Lyrics, NowPlaying, Playback, TrackLines},
    error::Error,
    widget::{Async, MyWidgetExt},
};

pub const SHOW_LYRICS: Selector<NowPlaying> = Selector::new("app.home.show_lyrics");

pub fn lyrics_widget() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            Label::new("Letra")
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .with_text_color(theme::PLACEHOLDER_COLOR),
        )
        .with_spacer(12.0)
        .with_child(
            Label::dynamic(|data: &AppState, _| {
                data.playback
                    .now_playing
                    .as_ref()
                    .map(|np| np.item.name().to_string())
                    .unwrap_or_else(|| "Reproduce una canción para ver su letra".into())
            })
            .with_font(theme::UI_FONT_MEDIUM)
            .with_text_size(theme::TEXT_SIZE_LARGE)
            .with_line_break_mode(LineBreaking::WordWrap),
        )
        .with_spacer(8.0)
        .with_child(
            Label::dynamic(|data: &AppState, _| {
                data.playback
                    .now_playing
                    .as_ref()
                    .and_then(|np| np.item.track())
                    .map(|track| track.artist_name().to_string())
                    .unwrap_or_default()
            })
            .with_text_color(theme::PLACEHOLDER_COLOR),
        )
        .with_spacer(16.0)
        .with_child(
            Button::new("Volver a obtener letra")
                .on_click(|ctx, data: &mut AppState, _| {
                    if let Some(np) = &data.playback.now_playing {
                        ctx.submit_command(SHOW_LYRICS.with(np.clone()));
                    }
                })
                .disabled_if(|data: &AppState, _| {
                    data.playback
                        .now_playing
                        .as_ref()
                        .and_then(|np| np.item.track())
                        .is_none()
                }),
        )
        .with_spacer(24.0)
        .with_child(track_lyrics_widget())
        .padding((24.0, 32.0))
        .expand_width()
}

fn track_lyrics_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || {
            let lines = List::new(|| {
                Label::raw()
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(18.0)
                    .lens(Ctx::data().then(TrackLines::words))
                    .expand_width()
                    .center()
                    .padding((12.0, 10.0))
                    .link()
                    .rounded(theme::BUTTON_BORDER_RADIUS)
                    .active(|line: &Ctx<Playback, TrackLines>, _: &Env| {
                        let Some(np) = &line.ctx.now_playing else {
                            return false;
                        };
                        let (Ok(start), Ok(end)) = (
                            line.data.start_time_ms.parse::<u128>(),
                            line.data.end_time_ms.parse::<u128>(),
                        ) else {
                            return false;
                        };
                        let progress = np.progress.as_millis();
                        progress >= start && progress < end
                    })
                    .on_left_click(|ctx, _, line, _| {
                        if let Ok(position) = line.data.start_time_ms.parse::<u64>() {
                            ctx.submit_command(cmd::SKIP_TO_POSITION.with(position));
                        }
                    })
            })
            .lens(Ctx::map(Lyrics::lines));
            Flex::column()
                .with_child(
                    Label::raw()
                        .with_text_size(theme::TEXT_SIZE_SMALL)
                        .with_text_color(theme::PLACEHOLDER_COLOR)
                        .with_line_break_mode(LineBreaking::WordWrap)
                        .lens(Ctx::data().then(Lyrics::notice)),
                )
                .with_spacer(20.0)
                .with_child(lines)
                .expand_width()
        },
        || {
            Label::dynamic(|err: &Error, _| format!("No se pudo obtener la letra. {err}"))
                .with_line_break_mode(LineBreaking::WordWrap)
                .center()
        },
    )
    .lens(Ctx::make(AppState::playback, AppState::lyrics).then(Ctx::in_promise()))
    .on_command_async(
        SHOW_LYRICS,
        |np| match np.item.track() {
            Some(track) => crate::webapi::lyrics::fetch(track),
            None => Err(Error::WebApiError(
                "Las letras están disponibles para canciones, no para podcasts.".into(),
            )),
        },
        |_, data, np| data.lyrics.defer(np.item.id().to_base62()),
        |_, data, (np, result)| data.lyrics.update((np.item.id().to_base62(), result)),
    )
}
