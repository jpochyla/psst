use crate::{
    commands,
    ctx::Ctx,
    data::{Navigation, PlaybackCtx, State, Track, TrackCtx},
    ui::theme,
    widgets::HoverExt,
};
use druid::{
    im::Vector,
    kurbo::Line,
    piet::StrokeStyle,
    widget::{Controller, CrossAxisAlignment, Flex, Label, List, ListIter, Painter},
    ContextMenu, Data, Env, Event, EventCtx, Lens, LensExt, LocalizedString, MenuDesc, MenuItem,
    MouseButton, RenderContext, Widget, WidgetExt, WidgetId,
};
use std::sync::Arc;

#[derive(Copy, Clone)]
pub struct TrackDisplay {
    pub number: bool,
    pub title: bool,
    pub artist: bool,
    pub album: bool,
}

#[derive(Clone, Data, Lens)]
struct TrackState {
    ctx: TrackCtx,
    track: Arc<Track>,
    index: usize,
}

impl ListIter<TrackState> for Ctx<TrackCtx, Vector<Arc<Track>>> {
    fn for_each(&self, mut cb: impl FnMut(&TrackState, usize)) {
        for (i, item) in self.data.iter().enumerate() {
            let d = TrackState {
                ctx: self.ctx.to_owned(),
                track: item.to_owned(),
                index: i,
            };
            cb(&d, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut TrackState, usize)) {
        for (i, item) in self.data.iter_mut().enumerate() {
            let mut d = TrackState {
                ctx: self.ctx.to_owned(),
                track: item.to_owned(),
                index: i,
            };
            cb(&mut d, i);

            if !self.ctx.same(&d.ctx) {
                self.ctx = d.ctx;
            }
            if !item.same(&d.track) {
                *item = d.track;
            }
            // `d.index` is considered immutable.
        }
    }

    fn data_len(&self) -> usize {
        self.data.len()
    }
}

pub fn make_tracklist(mode: TrackDisplay) -> impl Widget<Ctx<TrackCtx, Vector<Arc<Track>>>> {
    let id = WidgetId::next();
    List::new(move || make_track(mode, id))
        .controller(PlayController)
        .with_id(id)
}

struct PlayController;

impl<W> Controller<Ctx<TrackCtx, Vector<Arc<Track>>>, W> for PlayController
where
    W: Widget<Ctx<TrackCtx, Vector<Arc<Track>>>>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        tracks: &mut Ctx<TrackCtx, Vector<Arc<Track>>>,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(commands::PLAY_TRACK_AT) => {
                let position = cmd.get_unchecked(commands::PLAY_TRACK_AT);
                let pb = PlaybackCtx {
                    position: position.to_owned(),
                    tracks: tracks.data.to_owned(),
                };
                ctx.submit_command(commands::PLAY_TRACKS.with(pb));
            }
            _ => child.event(ctx, event, tracks, env),
        }
    }
}

fn make_track(display: TrackDisplay, play_ctrl: WidgetId) -> impl Widget<TrackState> {
    let track_duration =
        Label::dynamic(|ts: &TrackState, _| ts.track.duration.as_minutes_and_seconds())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR);

    let line_painter = Painter::new(move |ctx, ts: &TrackState, _| {
        let line = Line::new((0.0, 0.0), (ctx.size().width, 0.0));
        let color = if ts.ctx.playback.is_playing_track(&ts.track) {
            theme::BLACK
        } else {
            theme::GREY_5
        };
        ctx.stroke_styled(
            line,
            &color,
            1.0,
            &StrokeStyle {
                line_join: None,
                line_cap: None,
                dash: Some((vec![1.0, 2.0], 0.0)),
                miter_limit: None,
            },
        );
    })
    .fix_height(1.0);

    let mut major = Flex::row();
    let mut minor = Flex::row();

    if display.number {
        let track_number = Label::dynamic(|ts: &TrackState, _| ts.track.track_number.to_string())
            .with_font(theme::UI_FONT_MONO)
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR);
        major.add_child(track_number);
        major.add_default_spacer();
    }
    if display.title {
        let track_name = Label::raw()
            .with_font(theme::UI_FONT_MEDIUM)
            .lens(TrackState::track.then(Track::name.in_arc()));
        major.add_child(track_name);
    }
    if display.artist {
        let track_artist = Label::dynamic(|ts: &TrackState, _| ts.track.artist_name())
            .with_text_size(theme::TEXT_SIZE_SMALL);
        minor.add_child(track_artist);
    }
    if display.album {
        let track_album = Label::dynamic(|ts: &TrackState, _| ts.track.album_name())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR);
        if display.artist {
            minor.add_child(Label::new(": ").with_text_color(theme::GREY_5));
        }
        minor.add_child(track_album);
    }
    major.add_default_spacer();
    major.add_flex_child(line_painter, 1.0);
    major.add_default_spacer();
    major.add_child(track_duration);

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(major)
        .with_child(minor)
        .padding(theme::grid(0.8))
        .hover()
        .on_ex_click(
            move |ctx, event, ts: &mut TrackState, _| match event.button {
                MouseButton::Right => {
                    let menu = ContextMenu::new(make_track_menu(&ts.track), event.window_pos);
                    ctx.show_context_menu(menu);
                }
                MouseButton::Left => {
                    ctx.submit_command(commands::PLAY_TRACK_AT.with(ts.index).to(play_ctrl));
                }
                _ => {}
            },
        )
}

fn make_track_menu(track: &Track) -> MenuDesc<State> {
    let mut menu = MenuDesc::empty()
        .append(MenuItem::new(
            LocalizedString::new("menu-item-save-to-library").with_placeholder("Save to Library"),
            commands::SAVE_TRACK.with(track.id.clone().unwrap()),
        ))
        .append(MenuItem::new(
            LocalizedString::new("menu-item-remove-from-library")
                .with_placeholder("Remove from Library"),
            commands::UNSAVE_TRACK.with(track.id.clone().unwrap()),
        ))
        .append_separator();

    if let Some(artist) = track.artists.front() {
        menu = menu.append(MenuItem::new(
            LocalizedString::new("menu-item-show-artist").with_placeholder("Show Artist"),
            commands::NAVIGATE_TO.with(Navigation::ArtistDetail(artist.id.clone())),
        ));
    }
    if let Some(album) = track.album.as_ref() {
        menu = menu.append(MenuItem::new(
            LocalizedString::new("menu-item-show-album").with_placeholder("Show Album"),
            commands::NAVIGATE_TO.with(Navigation::AlbumDetail(album.id.clone())),
        ))
    }
    menu = menu.append(MenuItem::new(
        LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link"),
        commands::COPY_TO_CLIPBOARD.with(track.link()),
    ));

    menu
}
