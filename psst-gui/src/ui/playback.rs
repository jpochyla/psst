use crate::{
    cmd,
    data::{AudioDuration, Navigation, Playback, PlaybackState, State, Track},
    ui::{album, theme},
    widget::{icons, HoverExt, Maybe},
};
use druid::{
    lens::{Identity, InArc},
    widget::{
        Controller, CrossAxisAlignment, Flex, Label, Painter, SizedBox, Spinner, ViewSwitcher,
    },
    Env, Event, EventCtx, MouseButton, MouseEvent, PaintCtx, Point, Rect, RenderContext, Size,
    Widget, WidgetExt,
};
use std::sync::Arc;

pub fn make_panel() -> impl Widget<State> {
    Flex::row()
        .with_flex_child(make_info().align_left(), 1.0)
        .with_flex_child(make_player().align_right(), 1.0)
        .expand_width()
        .padding(theme::grid(1.0))
        .background(theme::WHITE)
        .lens(State::playback)
}

fn make_info() -> impl Widget<Playback> {
    Maybe::or_empty(make_info_track).lens(Playback::item)
}

fn make_info_track() -> impl Widget<Arc<Track>> {
    let album_cover = Maybe::or_empty(|| album::make_cover(theme::grid(7.0), theme::grid(7.0)))
        .lens(Track::album);

    let track_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .lens(Track::name);

    let track_artist = Label::dynamic(|track: &Track, _| track.artist_name())
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .hover()
        .on_click(|ctx: &mut EventCtx, track: &mut Track, _| {
            if let Some(artist) = track.artists.front() {
                let nav = Navigation::ArtistDetail(artist.id.clone());
                ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
            }
        });

    let track_album = Label::dynamic(|track: &Track, _| track.album_name())
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .hover()
        .on_click(|ctx, track: &mut Track, _| {
            if let Some(album) = track.album.as_ref() {
                let nav = Navigation::AlbumDetail(album.id.clone());
                ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
            }
        });

    let track_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(track_name)
        .with_child(track_artist)
        .with_child(track_album);

    Flex::row()
        .with_child(album_cover)
        .with_default_spacer()
        .with_child(track_info)
        .lens(InArc::new::<Arc<Track>, Arc<Track>>(Identity))
}

fn make_player() -> impl Widget<Playback> {
    ViewSwitcher::new(
        |playback: &Playback, _| playback.item.is_some(),
        |&has_item, _, _| {
            if has_item {
                Flex::column()
                    .with_child(make_player_controls())
                    .with_default_spacer()
                    .with_child(make_player_progress())
                    .boxed()
            } else {
                SizedBox::empty().boxed()
            }
        },
    )
}

fn make_player_controls() -> impl Widget<Playback> {
    let play_previous = icons::SKIP_BACK
        .scale((theme::grid(2.0), theme::grid(2.0)))
        .with_color(theme::PLACEHOLDER_COLOR)
        .padding(theme::grid(1.0))
        .hover()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_PREVIOUS));

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
                .border(theme::GREY_5)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_PAUSE))
                .boxed(),
            PlaybackState::Paused => icons::PLAY
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .hover()
                .circle()
                .border(theme::GREY_5)
                .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_RESUME))
                .boxed(),
            PlaybackState::Stopped => SizedBox::empty().boxed(),
        },
    );

    let play_next = icons::SKIP_FORWARD
        .scale((theme::grid(2.0), theme::grid(2.0)))
        .with_color(theme::PLACEHOLDER_COLOR)
        .padding(theme::grid(1.0))
        .hover()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _, _| ctx.submit_command(cmd::PLAY_NEXT));

    Flex::row()
        .with_child(play_previous)
        .with_default_spacer()
        .with_child(play_pause)
        .with_default_spacer()
        .with_child(play_next)
}

fn make_player_progress() -> impl Widget<Playback> {
    let current_time = Maybe::or_empty(|| {
        Label::dynamic(|progress: &AudioDuration, _| progress.as_minutes_and_seconds())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .align_right()
            .fix_width(theme::grid(4.0))
    })
    .lens(Playback::progress);
    let total_time = Maybe::or_empty(|| {
        Label::dynamic(|track: &Track, _| track.duration.as_minutes_and_seconds())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .align_left()
            .fix_width(theme::grid(4.0))
            .lens(InArc::new::<Arc<Track>, _>(Identity))
    })
    .lens(Playback::item);
    Flex::row()
        .with_child(current_time)
        .with_default_spacer()
        .with_flex_child(make_progress(), 1.0)
        .with_default_spacer()
        .with_child(total_time)
}

fn make_progress() -> impl Widget<Playback> {
    Painter::new(|ctx, playback: &Playback, env| {
        paint_progress(ctx, &playback, env);
    })
    .controller(SeekController)
    .fix_height(theme::grid(1.0))
}

fn paint_progress(ctx: &mut PaintCtx, playback: &Playback, env: &Env) {
    let elapsed_time = playback
        .progress
        .map(|progress| progress.as_secs_f32())
        .unwrap_or(0.0);
    let total_time = playback
        .item
        .as_ref()
        .map(|track| track.duration.as_secs_f32())
        .unwrap_or(0.0);

    let elapsed_color = env.get(theme::PRIMARY_DARK);
    let remaining_color = env.get(theme::PRIMARY_LIGHT).with_alpha(0.5);
    let bounds = ctx.size();

    const HEIGHT: f64 = 2.0;
    let elapsed_frac = elapsed_time / total_time;
    let elapsed_width = bounds.width * elapsed_frac as f64;
    let remaining_width = bounds.width - elapsed_width;
    let elapsed = Size::new(elapsed_width, HEIGHT).round();
    let remaining = Size::new(remaining_width, HEIGHT).round();

    let vertical_center = bounds.height / 2.0 - HEIGHT / 2.0;
    ctx.fill(
        &Rect::from_origin_size(Point::new(0.0, vertical_center), elapsed),
        &elapsed_color,
    );
    ctx.fill(
        &Rect::from_origin_size(Point::new(elapsed.width, vertical_center), remaining),
        &remaining_color,
    );
}

struct SeekController;

impl<T, W: Widget<T>> Controller<T, W> for SeekController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let seek_to_mouse_pos = |ctx: &mut EventCtx, mouse_event: &MouseEvent| {
            let frac = mouse_event.pos.x / ctx.size().width;
            ctx.submit_command(cmd::SEEK_TO_FRACTION.with(frac));
        };

        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button == MouseButton::Left {
                    ctx.set_active(true);
                }
            }
            Event::MouseUp(mouse_event) => {
                if ctx.is_active() && mouse_event.button == MouseButton::Left {
                    if ctx.is_hot() {
                        seek_to_mouse_pos(ctx, mouse_event);
                    }
                    ctx.set_active(false);
                }
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}
