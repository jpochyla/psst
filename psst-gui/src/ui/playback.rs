use crate::{
    cmd,
    data::{
        AudioAnalysis, AudioDuration, CurrentPlayback, Playback, PlaybackOrigin, PlaybackState,
        Promise, State, Track,
    },
    ui::{theme, utils::Border},
    widget::{icons, Empty, HoverExt, Maybe},
};
use druid::{
    kurbo::{Affine, BezPath},
    widget::{CrossAxisAlignment, Either, Flex, Label, LineBreaking, Spinner, ViewSwitcher},
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LensExt, LifeCycle, LifeCycleCtx,
    MouseButton, PaintCtx, Point, Rect, RenderContext, Size, UpdateCtx, Widget, WidgetExt,
};
use itertools::Itertools;
use std::sync::Arc;

pub fn make_panel() -> impl Widget<State> {
    Flex::row()
        .must_fill_main_axis(true)
        .with_flex_child(make_playback_info(), 1.0)
        .with_flex_child(make_player(), 1.0)
        .background(Border::Top.widget(theme::BACKGROUND_DARK))
        .lens(State::playback)
}

fn make_playback_info() -> impl Widget<Playback> {
    Maybe::or_empty(make_current_playback_info).lens(Playback::current)
}

fn make_current_playback_info() -> impl Widget<CurrentPlayback> {
    let track_name = Label::raw()
        .with_line_break_mode(LineBreaking::Clip)
        .with_font(theme::UI_FONT_MEDIUM)
        .lens(CurrentPlayback::item.then(Track::name.in_arc()));

    let track_artist = Label::dynamic(|track: &Arc<Track>, _| track.artist_name())
        .with_line_break_mode(LineBreaking::Clip)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .lens(CurrentPlayback::item);

    let track_origin = ViewSwitcher::new(
        |current: &CurrentPlayback, _| current.origin.clone(),
        |origin, _, _| {
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with_flex_child(
                    Label::dynamic(|current: &CurrentPlayback, _| current.origin.as_string())
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
                    .scale(theme::ICON_SIZE),
                )
                .boxed()
        },
    );

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(track_name)
        .with_child(track_artist)
        .with_child(track_origin)
        .padding(theme::grid(1.8))
        .expand_width()
        .hover()
        .on_ex_click(|ctx, _event, current: &mut CurrentPlayback, _| {
            let nav = current.origin.as_nav();
            ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
        })
}

fn make_player() -> impl Widget<Playback> {
    Flex::column()
        .with_child(make_player_controls())
        .with_default_spacer()
        .with_child(Maybe::or_empty(make_player_progress).lens(Playback::current))
        .padding(theme::grid(1.0))
}

fn make_player_controls() -> impl Widget<Playback> {
    let play_previous = icons::SKIP_BACK
        .scale((theme::grid(2.0), theme::grid(2.0)))
        .with_color(theme::PLACEHOLDER_COLOR)
        .padding(theme::grid(1.0))
        .hover()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_PREVIOUS));
    let play_previous = Either::new(
        |playback: &Playback, _| playback.current.is_some(),
        play_previous,
        Empty,
    );

    let play_pause = ViewSwitcher::new(
        |playback: &Playback, _| playback.state,
        |&state, _, _| match state {
            PlaybackState::Loading => Spinner::new()
                .with_color(theme::GREY_400)
                .padding(theme::grid(1.0))
                .hover()
                .circle()
                .border(theme::GREY_600, 1.0)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_STOP))
                .boxed(),
            PlaybackState::Playing => icons::PAUSE
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .hover()
                .circle()
                .border(theme::GREY_500, 1.0)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_PAUSE))
                .boxed(),
            PlaybackState::Paused => icons::PLAY
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .hover()
                .circle()
                .border(theme::GREY_500, 1.0)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_RESUME))
                .boxed(),
            PlaybackState::Stopped => Empty.boxed(),
        },
    );

    let play_next = icons::SKIP_FORWARD
        .scale((theme::grid(2.0), theme::grid(2.0)))
        .with_color(theme::PLACEHOLDER_COLOR)
        .padding(theme::grid(1.0))
        .hover()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_NEXT));
    let play_next = Either::new(
        |playback: &Playback, _| playback.current.is_some(),
        play_next,
        Empty,
    );

    Flex::row()
        .with_child(play_previous)
        .with_default_spacer()
        .with_child(play_pause)
        .with_default_spacer()
        .with_child(play_next)
}

fn make_player_progress() -> impl Widget<CurrentPlayback> {
    let current_time =
        Label::dynamic(|progress: &AudioDuration, _| progress.as_minutes_and_seconds())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .align_right()
            .fix_width(theme::grid(4.0))
            .lens(CurrentPlayback::progress);
    let total_time =
        Label::dynamic(|track: &Arc<Track>, _| track.duration.as_minutes_and_seconds())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .align_left()
            .fix_width(theme::grid(4.0))
            .lens(CurrentPlayback::item);
    Flex::row()
        .with_child(current_time)
        .with_default_spacer()
        .with_flex_child(SeekBar::new(), 1.0)
        .with_default_spacer()
        .with_child(total_time)
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

impl Widget<CurrentPlayback> for SeekBar {
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        _data: &mut CurrentPlayback,
        _env: &Env,
    ) {
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
                        ctx.submit_command(cmd::SEEK_TO_FRACTION.with(fraction));
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
        data: &CurrentPlayback,
        _env: &Env,
    ) {
        match &event {
            LifeCycle::Size(bounds) => {
                self.loudness_path = compute_loudness_path(bounds, &data);
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
        old_data: &CurrentPlayback,
        data: &CurrentPlayback,
        _env: &Env,
    ) {
        if !old_data.analysis.same(&data.analysis) || !old_data.item.same(&data.item) {
            self.loudness_path = compute_loudness_path(&ctx.size(), &data);
        }
        if !old_data.same(data) {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &CurrentPlayback,
        _env: &Env,
    ) -> Size {
        Size::new(bc.max().width, theme::grid(1.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &CurrentPlayback, env: &Env) {
        if self.loudness_path.is_empty() {
            paint_progress_bar(ctx, data, env)
        } else {
            paint_audio_analysis(ctx, data, &self.loudness_path, env)
        }
    }
}

fn compute_loudness_path(bounds: &Size, data: &CurrentPlayback) -> BezPath {
    if let Promise::Resolved(analysis) = &data.analysis {
        compute_loudness_path_from_analysis(&bounds, &data.item.duration, &analysis)
    } else {
        BezPath::new()
    }
}

fn compute_loudness_path_from_analysis(
    bounds: &Size,
    total_duration: &AudioDuration,
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

fn paint_audio_analysis(ctx: &mut PaintCtx, data: &CurrentPlayback, path: &BezPath, env: &Env) {
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

fn paint_progress_bar(ctx: &mut PaintCtx, data: &CurrentPlayback, env: &Env) {
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
