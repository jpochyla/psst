use crate::{
    cmd,
    data::{AudioDuration, CurrentPlayback, Playback, PlaybackOrigin, PlaybackState, State, Track},
    ui::theme,
    widget::{icons, Empty, HoverExt, Maybe},
};
use druid::{
    widget::{CrossAxisAlignment, Either, Flex, Label, LineBreaking, Spinner, ViewSwitcher},
    Data, Env, Event, EventCtx, LensExt, MouseButton, PaintCtx, Point, Rect, RenderContext, Size,
    Widget, WidgetExt,
};
use std::sync::Arc;

pub fn make_panel() -> impl Widget<State> {
    Flex::row()
        .must_fill_main_axis(true)
        .with_flex_child(make_playback_info(), 1.0)
        .with_flex_child(make_player(), 1.0)
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
                .with_child(
                    Label::dynamic(|current: &CurrentPlayback, _| current.origin.as_string())
                        .with_line_break_mode(LineBreaking::Clip)
                        .with_text_size(theme::TEXT_SIZE_SMALL),
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
                .with_color(theme::GREY_4)
                .padding(theme::grid(1.0))
                .boxed(),
            PlaybackState::Playing => icons::PAUSE
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .hover()
                .circle()
                .border(theme::GREY_5, 1.0)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_PAUSE))
                .boxed(),
            PlaybackState::Paused => icons::PLAY
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .hover()
                .circle()
                .border(theme::GREY_5, 1.0)
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
        .with_flex_child(SeekBar, 1.0)
        .with_default_spacer()
        .with_child(total_time)
}

struct SeekBar;

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
        _ctx: &mut druid::LifeCycleCtx,
        _event: &druid::LifeCycle,
        _data: &CurrentPlayback,
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &CurrentPlayback,
        data: &CurrentPlayback,
        _env: &Env,
    ) {
        if !old_data.same(data) {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &CurrentPlayback,
        _env: &Env,
    ) -> Size {
        Size::new(bc.max().width, 4.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &CurrentPlayback, env: &Env) {
        let elapsed_time = data.progress.as_secs_f32();
        let total_time = data.item.duration.as_secs_f32();

        let elapsed_color = env.get(theme::PRIMARY_DARK);
        let remaining_color = env.get(theme::PRIMARY_LIGHT).with_alpha(0.5);
        let bounds = ctx.size();

        let elapsed_frac = elapsed_time / total_time;
        let elapsed_width = bounds.width * elapsed_frac as f64;
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
}
