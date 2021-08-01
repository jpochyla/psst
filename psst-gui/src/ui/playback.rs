use crate::{
    cmd,
    data::{
        AppState, AudioAnalysis, NowPlaying, Playback, PlaybackOrigin, PlaybackState, Promise,
        QueueBehavior, Track,
    },
    ui::theme,
    widget::{icons, Empty, LinkExt, Maybe},
};
use druid::{
    kurbo::{Affine, BezPath},
    widget::{CrossAxisAlignment, Either, Flex, Label, LineBreaking, Spinner, ViewSwitcher},
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LensExt, LifeCycle, LifeCycleCtx,
    MouseButton, PaintCtx, Point, Rect, RenderContext, Size, UpdateCtx, Widget, WidgetExt,
};
use icons::SvgIcon;
use itertools::Itertools;
use std::{sync::Arc, time::Duration};

use super::utils;

pub fn panel_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Maybe::or_empty(SeekBar::new).lens(Playback::now_playing))
        .with_child(
            Flex::row()
                .must_fill_main_axis(true)
                .with_flex_child(
                    Maybe::or_empty(playback_item_widget).lens(Playback::now_playing),
                    1.0,
                )
                .with_flex_child(
                    Either::new(
                        |playback, _| playback.now_playing.is_some(),
                        player_widget(),
                        Empty,
                    ),
                    1.0,
                ),
        )
        .lens(AppState::playback)
}

fn playback_item_widget() -> impl Widget<NowPlaying> {
    let track_name = Label::raw()
        .with_line_break_mode(LineBreaking::Clip)
        .with_font(theme::UI_FONT_MEDIUM)
        .lens(NowPlaying::item.then(Track::name.in_arc()));

    let track_artist = Label::dynamic(|track: &Arc<Track>, _| track.artist_name())
        .with_line_break_mode(LineBreaking::Clip)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .lens(NowPlaying::item);

    let track_origin = ViewSwitcher::new(
        |origin: &PlaybackOrigin, _| origin.clone(),
        |origin, _, _| {
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with_flex_child(
                    Label::dynamic(|origin: &PlaybackOrigin, _| origin.to_string())
                        .with_line_break_mode(LineBreaking::Clip)
                        .with_text_size(theme::TEXT_SIZE_SMALL),
                    1.0,
                )
                .with_spacer(theme::grid(0.25))
                .with_child(
                    match origin {
                        PlaybackOrigin::Library => &icons::HEART,
                        PlaybackOrigin::Album { .. } => &icons::ALBUM,
                        PlaybackOrigin::Artist { .. } => &icons::ARTIST,
                        PlaybackOrigin::Playlist { .. } => &icons::PLAYLIST,
                        PlaybackOrigin::Search { .. } => &icons::SEARCH,
                    }
                    .scale(theme::ICON_SIZE_SMALL),
                )
                .boxed()
        },
    )
    .lens(NowPlaying::origin);

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(track_name)
        .with_spacer(2.0)
        .with_child(track_artist)
        .with_spacer(2.0)
        .with_child(track_origin)
        .padding(theme::grid(2.0))
        .expand_width()
        .link()
        .on_ex_click(|ctx, _, now_playing, _| {
            ctx.submit_command(cmd::NAVIGATE.with(now_playing.origin.to_nav()));
        })
}

fn player_widget() -> impl Widget<Playback> {
    Flex::row()
        .with_child(
            small_button_widget(&icons::SKIP_BACK)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_PREVIOUS)),
        )
        .with_default_spacer()
        .with_child(player_play_pause_widget())
        .with_default_spacer()
        .with_child(
            small_button_widget(&icons::SKIP_FORWARD)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_NEXT)),
        )
        .with_default_spacer()
        .with_child(queue_behavior_widget())
        .with_default_spacer()
        .with_child(Maybe::or_empty(durations_widget).lens(Playback::now_playing))
}

fn player_play_pause_widget() -> impl Widget<Playback> {
    ViewSwitcher::new(
        |playback: &Playback, _| playback.state,
        |&state, _, _| match state {
            PlaybackState::Loading => Spinner::new()
                .with_color(theme::GREY_400)
                .fix_size(theme::grid(3.0), theme::grid(3.0))
                .padding(theme::grid(1.0))
                .link()
                .circle()
                .border(theme::GREY_600, 1.0)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_STOP))
                .boxed(),
            PlaybackState::Playing => icons::PAUSE
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .link()
                .circle()
                .border(theme::GREY_500, 1.0)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_PAUSE))
                .boxed(),
            PlaybackState::Paused => icons::PLAY
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .link()
                .circle()
                .border(theme::GREY_500, 1.0)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_RESUME))
                .boxed(),
            PlaybackState::Stopped => Empty.boxed(),
        },
    )
}

fn queue_behavior_widget() -> impl Widget<Playback> {
    ViewSwitcher::new(
        |playback: &Playback, _| playback.queue_behavior.to_owned(),
        |behavior, _, _| {
            let button = |svg: &SvgIcon| {
                faded_button_widget(svg)
                    .on_click(|ctx: &mut EventCtx, playback: &mut Playback, _| {
                        let new_behavior = match playback.queue_behavior {
                            QueueBehavior::Sequential => QueueBehavior::Random,
                            QueueBehavior::Random => QueueBehavior::LoopTrack,
                            QueueBehavior::LoopTrack => QueueBehavior::LoopAll,
                            QueueBehavior::LoopAll => QueueBehavior::Sequential,
                        };
                        ctx.submit_command(cmd::PLAY_QUEUE_BEHAVIOR.with(new_behavior));
                    })
                    .boxed()
            };
            match behavior {
                QueueBehavior::Sequential => button(&icons::PLAY_SEQUENTIAL),
                QueueBehavior::Random => button(&icons::PLAY_SHUFFLE),
                QueueBehavior::LoopTrack => button(&icons::PLAY_LOOP_TRACK),
                QueueBehavior::LoopAll => button(&icons::PLAY_LOOP_ALL),
            }
        },
    )
}

fn small_button_widget<T: Data>(svg: &SvgIcon) -> impl Widget<T> {
    svg.scale((theme::grid(2.0), theme::grid(2.0)))
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
}

fn faded_button_widget<T: Data>(svg: &SvgIcon) -> impl Widget<T> {
    svg.scale((theme::grid(2.0), theme::grid(2.0)))
        .with_color(theme::PLACEHOLDER_COLOR)
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
}

fn durations_widget() -> impl Widget<NowPlaying> {
    Label::dynamic(|now_playing: &NowPlaying, _| {
        format!(
            "{} / {}",
            utils::as_minutes_and_seconds(&now_playing.progress),
            utils::as_minutes_and_seconds(&now_playing.item.duration)
        )
    })
    .with_text_size(theme::TEXT_SIZE_SMALL)
    .with_text_color(theme::PLACEHOLDER_COLOR)
}

struct SeekBar {
    loudness_path: BezPath,
}

impl SeekBar {
    fn new() -> Self {
        Self {
            loudness_path: BezPath::new(),
        }
    }
}

impl Widget<NowPlaying> for SeekBar {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut NowPlaying, _env: &Env) {
        match event {
            Event::MouseDown(mouse) => {
                if mouse.button == MouseButton::Left {
                    ctx.set_active(true);
                }
            }
            Event::MouseUp(mouse) => {
                if ctx.is_active() && mouse.button == MouseButton::Left {
                    if ctx.is_hot() {
                        let fraction = mouse.pos.x / ctx.size().width;
                        ctx.submit_command(cmd::PLAY_SEEK.with(fraction));
                    }
                    ctx.set_active(false);
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &NowPlaying,
        _env: &Env,
    ) {
        match &event {
            LifeCycle::Size(_bounds) => {
                // self.loudness_path = compute_loudness_path(bounds, &data);
            }
            LifeCycle::HotChanged(_) => {
                ctx.request_paint();
            }
            _ => {}
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &NowPlaying,
        data: &NowPlaying,
        _env: &Env,
    ) {
        if !old_data.analysis.same(&data.analysis) || !old_data.item.same(&data.item) {
            // self.loudness_path = compute_loudness_path(&ctx.size(), &data);
        }
        if !old_data.same(data) {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &NowPlaying,
        _env: &Env,
    ) -> Size {
        Size::new(bc.max().width, theme::grid(1.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &NowPlaying, env: &Env) {
        if self.loudness_path.is_empty() {
            paint_progress_bar(ctx, data, env)
        } else {
            paint_audio_analysis(ctx, data, &self.loudness_path, env)
        }
    }
}

fn _compute_loudness_path(bounds: &Size, data: &NowPlaying) -> BezPath {
    if let Promise::Resolved(analysis) = &data.analysis {
        _compute_loudness_path_from_analysis(bounds, &data.item.duration, analysis)
    } else {
        BezPath::new()
    }
}

fn _compute_loudness_path_from_analysis(
    bounds: &Size,
    total_duration: &Duration,
    analysis: &AudioAnalysis,
) -> BezPath {
    let (loudness_min, loudness_max) = analysis
        .segments
        .iter()
        .map(|s| s.loudness_max)
        .minmax()
        .into_option()
        .unwrap_or((0.0, 0.0));
    let total_loudness = loudness_max - loudness_min;

    let mut path = BezPath::new();

    // We start in the middle of the vertical space and first draw the upper half of
    // the curve, then take what we have drawn, flip the y-axis and append it
    // underneath.
    let origin_y = bounds.height / 2.0;

    // Start at the origin.
    path.move_to((0.0, origin_y));

    // Because the size of the seekbar is quite small, but the number of the
    // segments can be large, we down-sample the loudness spectrum in a very
    // primitive way and only add a vertex after crossing `WIDTH_PRECISION` of
    // pixels horizontally.
    const WIDTH_PRECISION: f64 = 2.0;
    let mut last_width = 0.0;

    for seg in &analysis.segments {
        let time = seg.interval.start.as_secs_f64() + seg.loudness_max_time;
        let tfrac = time / total_duration.as_secs_f64();
        let width = bounds.width * tfrac;

        let loud = seg.loudness_max - loudness_min;
        let lfrac = loud / total_loudness;
        let height = bounds.height * lfrac;

        if width - last_width >= WIDTH_PRECISION {
            // Down-scale the height, because we will be drawing also the inverted half.
            path.line_to((width, origin_y - height / 2.0));

            // Save the X-coordinate of this vertex.
            last_width = width;
        }
    }

    // Land back at the vertical origin.
    path.line_to((bounds.width, origin_y));

    // Flip the y-axis, translate just under the origin, and append.
    let mut inverted_path = path.clone();
    let inversion_tx = Affine::FLIP_Y * Affine::translate((0.0, -bounds.height));
    inverted_path.apply_affine(inversion_tx);
    path.extend(inverted_path);

    path
}

fn paint_audio_analysis(ctx: &mut PaintCtx, data: &NowPlaying, path: &BezPath, env: &Env) {
    let bounds = ctx.size();

    let elapsed_time = data.progress.as_secs_f64();
    let total_time = data.item.duration.as_secs_f64();
    let elapsed_frac = elapsed_time / total_time;
    let elapsed_width = bounds.width * elapsed_frac;
    let elapsed = Size::new(elapsed_width, bounds.height).to_rect();

    let (elapsed_color, remaining_color) = if ctx.is_hot() {
        (env.get(theme::GREY_200), env.get(theme::GREY_500))
    } else {
        (env.get(theme::GREY_300), env.get(theme::GREY_600))
    };

    ctx.with_save(|ctx| {
        ctx.fill(&path, &remaining_color);
        ctx.clip(&elapsed);
        ctx.fill(&path, &elapsed_color);
    });
}

fn paint_progress_bar(ctx: &mut PaintCtx, data: &NowPlaying, env: &Env) {
    let elapsed_time = data.progress.as_secs_f64();
    let total_time = data.item.duration.as_secs_f64();

    let (elapsed_color, remaining_color) = if ctx.is_hot() {
        (env.get(theme::GREY_200), env.get(theme::GREY_500))
    } else {
        (env.get(theme::GREY_300), env.get(theme::GREY_600))
    };
    let bounds = ctx.size();

    let elapsed_frac = elapsed_time / total_time;
    let elapsed_width = bounds.width * elapsed_frac;
    let remaining_width = bounds.width - elapsed_width;
    let elapsed = Size::new(elapsed_width, bounds.height).round();
    let remaining = Size::new(remaining_width, bounds.height).round();

    ctx.fill(
        &Rect::from_origin_size(Point::ORIGIN, elapsed),
        &elapsed_color,
    );
    ctx.fill(
        &Rect::from_origin_size(Point::new(elapsed.width, 0.0), remaining),
        &remaining_color,
    );
}
