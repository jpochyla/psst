use druid::widget::{Container, CrossAxisAlignment, Flex, Label, LineBreaking, List, Scroll};
use druid::{Insets, LensExt, Selector, Widget, WidgetExt};

use crate::cmd;
use crate::data::{AppState, Ctx, NowPlaying, Playable, TrackLines};
use crate::data::CommonCtx;
use crate::widget::MyWidgetExt;
use crate::{webapi::WebApi, widget::Async};

use super::theme;
use super::utils;

use std::sync::Arc;
use druid::im::Vector;

pub const SHOW_LYRICS: Selector<NowPlaying> = Selector::new("app.home.show_lyrics");

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
        || {
            List::new(|| {
                Label::raw()
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .lens(Ctx::data().then(TrackLines::words))
                    .expand_width()
                    .center()
                    .padding(Insets::uniform_xy(theme::grid(1.0), theme::grid(0.5)))
                    .link()
                    .active(|c: &Ctx<Arc<CommonCtx>, TrackLines>, _env| {
                        let current_progress = c.ctx.current_progress().as_millis() as f64;
                        let start_ms = c.data.start_time_ms.parse::<f64>().unwrap_or(0.0);
                        let parsed_end = c.data.end_time_ms.parse::<f64>().unwrap_or(0.0);
                        let end_ms = if parsed_end > start_ms { parsed_end } else { start_ms + 800.0 };
                        current_progress >= start_ms && current_progress < end_ms
                    })
                    .rounded(theme::BUTTON_BORDER_RADIUS)
                    .env_scope(|env, _| {
                        let active = env.get(theme::BLUE_100).with_alpha(0.25);
                        env.set(theme::LINK_ACTIVE_COLOR, active);
                    })
                    .on_update(|ctx, old, new, _env| {
                        let is_line_active = |ctx: &Arc<CommonCtx>, line: &TrackLines| {
                            let current_progress = ctx.current_progress().as_millis() as f64;
                            let start_ms = line.start_time_ms.parse::<f64>().unwrap_or(0.0);
                            let parsed_end = line.end_time_ms.parse::<f64>().unwrap_or(0.0);
                            let end_ms = if parsed_end > start_ms { parsed_end } else { start_ms + 800.0 };
                            current_progress >= start_ms && current_progress < end_ms
                        };

                        let was_active = is_line_active(&old.ctx, &old.data);
                        let is_active = is_line_active(&new.ctx, &new.data);

                        if !was_active && is_active {
                            ctx.scroll_to_view();
                        }
                    })
                    .on_left_click(|ctx, _, c, _| {
                        if c.data.start_time_ms.parse::<u64>().unwrap() != 0 {
                            ctx.submit_command(
                                cmd::SKIP_TO_POSITION
                                    .with(c.data.start_time_ms.parse::<u64>().unwrap()),
                            )
                        }
                    })
            })
        },
        || Label::new("No lyrics found for this track").center(),
    )
    .lens(Ctx::make(AppState::common_ctx, AppState::lyrics).then(Ctx::in_promise()))
    .on_command_async(
        SHOW_LYRICS,
        |t| WebApi::global().get_lyrics(t.item.id().to_base62()),
        |_, data, _| data.lyrics.defer(()),
        |_, data, r| {
            data.reset_lyrics_offset();
            let processed = match r.1 {
                Ok(lines) => {
                    let mut out = Vector::new();
                    let len = lines.len();
                    for idx in 0..len {
                        let mut l = lines[idx].clone();
                        let end_zero = l.end_time_ms.parse::<u64>().unwrap_or(0) == 0;
                        if end_zero {
                            if idx + 1 < len {
                                l.end_time_ms = lines[idx + 1].start_time_ms.clone();
                            } else {
                                if let Ok(start) = l.start_time_ms.parse::<u64>() {
                                    l.end_time_ms = (start + 800).to_string();
                                }
                            }
                        }
                        out.push_back(l);
                    }
                    Ok(out)
                }
                Err(e) => Err(e),
            };
            data.lyrics.update(((), processed));
        },
    )
}
