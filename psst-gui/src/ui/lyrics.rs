use druid::{
    widget::{
        prelude::*, Button, Checkbox, Controller, CrossAxisAlignment, Flex, Label, LineBreaking,
        List,
    },
    LensExt, Selector, Widget, WidgetExt,
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
    let heading = Label::dynamic(|data: &AppState, _| {
        data.playback
            .now_playing
            .as_ref()
            .map(|np| np.item.name().to_string())
            .unwrap_or_else(|| "Reproduce una canci\u{00f3}n para ver su letra".into())
    })
    .with_font(theme::UI_FONT_MEDIUM)
    .with_text_size(16.0)
    .with_line_break_mode(LineBreaking::WordWrap)
    .context_menu(super::queue::current_menu);
    let controls = Flex::row()
        .with_child(
            Button::new("Actualizar letra")
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
        .with_spacer(12.0)
        .with_child(
            Checkbox::new("Seguir letra")
                .lens(AppState::playback.then(Playback::lyrics_follow))
                .tooltip("Desactiva el seguimiento para leer otra parte de la letra"),
        );
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(heading.padding((12.0, 8.0)).expand_width())
        .with_child(controls.padding((12.0, 8.0)))
        .with_flex_child(
            druid::widget::Scroll::new(track_lyrics_widget().padding((12.0, 8.0)))
                .vertical()
                .expand_width(),
            1.0,
        )
        .expand_width()
}

fn track_lyrics_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || {
            let lines = List::new(|| {
                Label::raw()
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_font(theme::UI_FONT_MEDIUM)
                    .with_text_size(22.0)
                    .lens(Ctx::data().then(TrackLines::words))
                    .expand_width()
                    .center()
                    .padding((12.0, 10.0))
                    .link()
                    .rounded(theme::BUTTON_BORDER_RADIUS)
                    .active(|line: &Ctx<Playback, TrackLines>, _: &Env| active(line))
                    .on_left_click(|ctx, _, line, _| {
                        if let Ok(position) = line.data.start_time_ms.parse::<u64>() {
                            ctx.submit_command(cmd::SKIP_TO_POSITION.with(position));
                        }
                    })
                    .env_scope(|env, line: &Ctx<Playback, TrackLines>| {
                        env.set(
                            theme::TEXT_COLOR,
                            env.get(if active(line) {
                                theme::BLUE_200
                            } else {
                                theme::PLACEHOLDER_COLOR
                            }),
                        );
                    })
                    .controller(FollowLine::default())
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

fn active(line: &Ctx<Playback, TrackLines>) -> bool {
    let Some(np) = &line.ctx.now_playing else {
        return false;
    };
    line_active(&line.data, np.progress.as_millis())
}
fn line_active(line: &TrackLines, progress: u128) -> bool {
    let (Ok(start), Ok(end)) = (
        line.start_time_ms.parse::<u128>(),
        line.end_time_ms.parse::<u128>(),
    ) else {
        return false;
    };
    progress >= start && progress < end
}

#[derive(Default)]
struct FollowLine {
    pending: bool,
}
impl<W: Widget<Ctx<Playback, TrackLines>>> Controller<Ctx<Playback, TrackLines>, W> for FollowLine {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Ctx<Playback, TrackLines>,
        env: &Env,
    ) {
        if matches!(event, Event::AnimFrame(_)) && self.pending {
            self.pending = false;
            if data.ctx.lyrics_follow && active(data) {
                let area = ctx.size().to_rect().inflate(0.0, 72.0);
                ctx.scroll_area_to_view(area);
            }
        }
        child.event(ctx, event, data, env);
    }
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Ctx<Playback, TrackLines>,
        env: &Env,
    ) {
        child.lifecycle(ctx, event, data, env);
        if matches!(event, LifeCycle::WidgetAdded | LifeCycle::Size(_))
            && data.ctx.lyrics_follow
            && active(data)
        {
            self.pending = true;
            ctx.request_anim_frame();
        }
    }
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old: &Ctx<Playback, TrackLines>,
        data: &Ctx<Playback, TrackLines>,
        env: &Env,
    ) {
        child.update(ctx, old, data, env);
        if data.ctx.lyrics_follow && active(data) && (!active(old) || !old.ctx.lyrics_follow) {
            self.pending = true;
            ctx.request_anim_frame();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seek_and_boundaries_select_one_line_and_plain_lyrics_do_not_follow() {
        let line = TrackLines {
            start_time_ms: "3000".into(),
            end_time_ms: "6000".into(),
            words: "example".into(),
        };
        assert!(!line_active(&line, 2999));
        assert!(line_active(&line, 3000));
        assert!(line_active(&line, 5999));
        assert!(!line_active(&line, 6000));
        assert!(!line_active(&line, 1000));
        let plain = TrackLines {
            start_time_ms: "-1".into(),
            end_time_ms: "-1".into(),
            ..line
        };
        assert!(!line_active(&plain, 4000));
    }
}
