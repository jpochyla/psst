use crate::data::PlaybackContext;
use crate::{
    commands,
    data::{Navigation, State, Track},
    ui::theme,
    widgets::HoverExt,
};
use druid::{
    im::Vector,
    kurbo::Line,
    piet::StrokeStyle,
    widget::{Controller, CrossAxisAlignment, Flex, Label, List, ListIter, Painter, Scope},
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

#[derive(Clone, Data)]
struct TrackShared {
    playing: Option<String>,
}

impl TrackShared {
    fn new() -> Self {
        Self { playing: None }
    }
}

#[derive(Clone, Data, Lens)]
pub struct TrackState {
    shared: TrackShared,
    track: Arc<Track>,
    index: usize,
}

#[derive(Clone, Data, Lens)]
struct TrackScope {
    shared: TrackShared,
    tracks: Vector<Arc<Track>>,
}

impl ListIter<TrackState> for TrackScope {
    fn for_each(&self, mut cb: impl FnMut(&TrackState, usize)) {
        for (i, item) in self.tracks.iter().enumerate() {
            let d = TrackState {
                shared: self.shared.to_owned(),
                track: item.to_owned(),
                index: i,
            };
            cb(&d, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut TrackState, usize)) {
        for (i, item) in self.tracks.iter_mut().enumerate() {
            let mut d = TrackState {
                shared: self.shared.to_owned(),
                track: item.to_owned(),
                index: i,
            };
            cb(&mut d, i);

            if !self.shared.same(&d.shared) {
                self.shared = d.shared;
            }
            if !item.same(&d.track) {
                *item = d.track;
            }
        }
    }

    fn data_len(&self) -> usize {
        self.tracks.len()
    }
}

pub fn make_tracklist(mode: TrackDisplay) -> impl Widget<Vector<Arc<Track>>> {
    let id = WidgetId::next();
    let list = List::new(move || make_track(mode, id))
        .controller(PlayController)
        .with_id(id);
    let scope = Scope::from_lens(
        |tracks: Vector<Arc<Track>>| TrackScope {
            tracks,
            shared: TrackShared::new(),
        },
        TrackScope::tracks,
        list,
    );
    scope
}

struct PlayController;

impl<W> Controller<TrackScope, W> for PlayController
where
    W: Widget<TrackScope>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        scope: &mut TrackScope,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(commands::PLAY_TRACK_AT) => {
                let position = cmd.get_unchecked(commands::PLAY_TRACK_AT);
                let pb_ctx = PlaybackContext {
                    position: position.to_owned(),
                    tracks: scope.tracks.to_owned(),
                };
                ctx.submit_command(commands::PLAY_TRACKS.with(pb_ctx));
            }
            Event::Command(cmd) if cmd.is(commands::PLAYBACK_PLAYING) => {
                let report = cmd.get_unchecked(commands::PLAYBACK_PLAYING);
                scope.shared.playing = Some(report.item.to_owned());
            }
            _ => child.event(ctx, event, scope, env),
        }
    }
}

pub fn make_track(display: TrackDisplay, play_ctrl: WidgetId) -> impl Widget<TrackState> {
    let track_duration =
        Label::dynamic(|ts: &TrackState, _| ts.track.duration.as_minutes_and_seconds())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR);

    let line_painter = Painter::new(move |ctx, ts: &TrackState, _| {
        let size = ctx.size();
        let line = Line::new((0.0, 0.0), (size.width, 0.0));
        let color = if ts.shared.playing.same(&ts.track.id) {
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
        minor.add_child(Label::new(": ").with_text_color(theme::GREY_5));
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
