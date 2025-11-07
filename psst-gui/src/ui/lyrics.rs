use std::sync::Arc;

use druid::widget::{
    Container, Controller, CrossAxisAlignment, Flex, Label, LineBreaking, List, Painter, Scroll,
};
use druid::RenderContext;
use druid::{
    Env, Event, EventCtx, Insets, LensExt, LifeCycle, LifeCycleCtx, Selector, UpdateCtx, Widget,
    WidgetExt,
};

use crate::cmd;
use crate::data::{AppState, CommonCtx, Ctx, NowPlaying, Playable, TrackLines};
use crate::widget::MyWidgetExt;
use crate::{webapi::WebApi, widget::Async};

use super::theme;
use super::utils;

pub const SHOW_LYRICS: Selector<NowPlaying> = Selector::new("app.home.show_lyrics");

type LyricItem = Ctx<Arc<CommonCtx>, TrackLines>;

const FALLBACK_LINE_DURATION_MS: u64 = 5_000;

pub fn lyrics_widget() -> impl Widget<AppState> {
    Scroll::new(
        Container::new(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with_default_spacer()
                .with_child(track_info_widget())
                .with_spacer(theme::grid(2.0))
                .with_child(track_lyrics_widget()),
        )
        .fix_width(400.0)
        .center(),
    )
    .vertical()
}

fn track_info_widget() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            Label::dynamic(|data: &AppState, _| {
                data.playback.now_playing.as_ref().map_or_else(
                    || "No track playing".to_string(),
                    |np| match &np.item {
                        Playable::Track(track) => track.name.clone().to_string(),
                        _ => "Unknown track".to_string(),
                    },
                )
            })
            .with_font(theme::UI_FONT_MEDIUM)
            .with_text_size(theme::TEXT_SIZE_LARGE),
        )
        .with_spacer(theme::grid(0.5))
        .with_child(
            Label::dynamic(|data: &AppState, _| {
                data.playback.now_playing.as_ref().map_or_else(
                    || "".to_string(),
                    |np| match &np.item {
                        Playable::Track(track) => {
                            format!("{} - {}", track.artist_name(), track.album_name())
                        }
                        _ => "".to_string(),
                    },
                )
            })
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR),
        )
}

fn track_lyrics_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || List::new(lyric_line_widget),
        || Label::new("No lyrics found for this track").center(),
    )
    .lens(Ctx::make(AppState::common_ctx, AppState::lyrics).then(Ctx::in_promise()))
    .on_command_async(
        SHOW_LYRICS,
        |t| WebApi::global().get_lyrics(t.item.id().to_base62()),
        |_, data, _| data.lyrics.defer(()),
        |_, data, r| data.lyrics.update(((), r.1)),
    )
}

fn lyric_line_widget() -> impl Widget<LyricItem> {
    let label = Label::dynamic(|data: &LyricItem, _| data.data.words.clone())
        .with_line_break_mode(LineBreaking::WordWrap)
        .expand_width()
        .center();

    let background = Painter::new(|ctx, data: &LyricItem, env| {
        if is_current_line(data) {
            let rect = ctx
                .size()
                .to_rect()
                .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));
            ctx.fill(rect, &env.get(theme::PRIMARY_DARK));
        }
    });

    Container::new(label)
        .padding(Insets::uniform_xy(theme::grid(1.0), theme::grid(0.5)))
        .background(background)
        .env_scope(|env, data| {
            if is_current_line(data) {
                env.set(theme::TEXT_COLOR, env.get(theme::FOREGROUND_LIGHT));
                env.set(theme::UI_FONT, env.get(theme::UI_FONT_MEDIUM));
            } else {
                env.set(theme::TEXT_COLOR, env.get(theme::GREY_300));
            }
        })
        .on_left_click(|ctx, _, data: &mut LyricItem, _| {
            if let Some(start) = parse_millis(&data.data.start_time_ms) {
                if start > 0 {
                    ctx.submit_command(cmd::SKIP_TO_POSITION.with(start));
                }
            }
        })
        .controller(LyricHighlightController::default())
}

fn parse_millis(value: &str) -> Option<u64> {
    value.trim().parse().ok()
}

fn line_time_range(line: &TrackLines) -> Option<(u64, u64)> {
    let start = parse_millis(&line.start_time_ms)?;
    let end = match parse_millis(&line.end_time_ms) {
        Some(end) if end > start => end,
        Some(_) => start.saturating_add(FALLBACK_LINE_DURATION_MS),
        None => u64::MAX,
    };
    Some((start, end))
}

fn is_current_line(data: &LyricItem) -> bool {
    let Some(progress) = data.ctx.playback_progress else {
        return false;
    };
    let Some((start, end)) = line_time_range(&data.data) else {
        return false;
    };
    progress >= start && progress < end
}

#[derive(Default)]
struct LyricHighlightController {
    should_scroll: bool,
}

impl<W: Widget<LyricItem>> Controller<LyricItem, W> for LyricHighlightController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut LyricItem,
        env: &Env,
    ) {
        if let Event::AnimFrame(_) = event {
            if self.should_scroll {
                ctx.scroll_to_view();
                self.should_scroll = false;
            }
        }
        child.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &LyricItem,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) && is_current_line(data) {
            self.should_scroll = true;
            ctx.request_anim_frame();
        }
        child.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &LyricItem,
        data: &LyricItem,
        env: &Env,
    ) {
        let was_current = is_current_line(old_data);
        let is_current = is_current_line(data);
        if was_current != is_current {
            ctx.request_paint();
            ctx.request_layout();
            if is_current {
                self.should_scroll = true;
                ctx.request_anim_frame();
            }
        }
        child.update(ctx, old_data, data, env);
    }
}
