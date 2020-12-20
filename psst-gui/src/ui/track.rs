use crate::{
    cmd,
    data::{Ctx, Navigation, PlaybackCtx, State, Track, TrackCtx, TrackList},
    ui::theme,
    widget::HoverExt,
};
use druid::{
    kurbo::Line,
    lens::Map,
    piet::StrokeStyle,
    widget::{Controller, CrossAxisAlignment, Flex, Label, List, ListIter, Painter},
    ContextMenu, Data, Env, Event, EventCtx, Lens, LensExt, LocalizedString, MenuDesc, MenuItem,
    MouseButton, RenderContext, Widget, WidgetExt,
};
use std::sync::Arc;

#[derive(Copy, Clone)]
pub struct TrackDisplay {
    pub number: bool,
    pub title: bool,
    pub artist: bool,
    pub album: bool,
}

pub fn make_tracklist(mode: TrackDisplay) -> impl Widget<Ctx<TrackCtx, TrackList>> {
    List::new(move || make_track(mode)).controller(PlayController)
}

#[derive(Clone, Data, Lens)]
struct TrackRow {
    ctx: TrackCtx,
    track: Arc<Track>,
    position: usize,
}

impl TrackRow {
    fn is_playing() -> impl Lens<TrackRow, bool> {
        Map::new(
            |tr: &TrackRow| tr.ctx.is_track_playing(&tr.track),
            |tr: &mut TrackRow, is_playing| {
                // Ignore mutation.
            },
        )
    }
}

impl ListIter<TrackRow> for Ctx<TrackCtx, TrackList> {
    fn for_each(&self, mut cb: impl FnMut(&TrackRow, usize)) {
        for (i, item) in self.data.tracks.iter().enumerate() {
            let d = TrackRow {
                ctx: self.ctx.to_owned(),
                track: item.to_owned(),
                position: i,
            };
            cb(&d, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut TrackRow, usize)) {
        for (i, item) in self.data.tracks.iter_mut().enumerate() {
            let mut d = TrackRow {
                ctx: self.ctx.to_owned(),
                track: item.to_owned(),
                position: i,
            };
            cb(&mut d, i);

            if !self.ctx.same(&d.ctx) {
                self.ctx = d.ctx;
            }
            if !item.same(&d.track) {
                *item = d.track;
            }
            // `d.position` is considered immutable.
        }
    }

    fn data_len(&self) -> usize {
        self.data.tracks.len()
    }
}

struct PlayController;

impl<W> Controller<Ctx<TrackCtx, TrackList>, W> for PlayController
where
    W: Widget<Ctx<TrackCtx, TrackList>>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        tracks: &mut Ctx<TrackCtx, TrackList>,
        env: &Env,
    ) {
        match event {
            Event::Notification(note) => {
                if let Some(position) = note.get(cmd::PLAY_TRACK_AT) {
                    let playback_ctx = PlaybackCtx {
                        tracks: tracks.data.to_owned(),
                        position: position.to_owned(),
                    };
                    ctx.submit_command(cmd::PLAY_TRACKS.with(playback_ctx));
                    ctx.set_handled();
                }
            }
            _ => child.event(ctx, event, tracks, env),
        }
    }
}

fn make_track(display: TrackDisplay) -> impl Widget<TrackRow> {
    let track_duration =
        Label::dynamic(|tr: &TrackRow, _| tr.track.duration.as_minutes_and_seconds())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR);

    let line_painter = Painter::new(move |ctx, is_playing: &bool, _| {
        let line = Line::new((0.0, 0.0), (ctx.size().width, 0.0));
        let color = if *is_playing {
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
    .lens(TrackRow::is_playing())
    .fix_height(1.0);

    let mut major = Flex::row();
    let mut minor = Flex::row();

    if display.number {
        let track_number = Label::dynamic(|tr: &TrackRow, _| tr.track.track_number.to_string())
            .with_font(theme::UI_FONT_MONO)
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR);
        major.add_child(track_number);
        major.add_default_spacer();
    }
    if display.title {
        let track_name = Label::raw()
            .with_font(theme::UI_FONT_MEDIUM)
            .lens(TrackRow::track.then(Track::name.in_arc()));
        major.add_child(track_name);
    }
    if display.artist {
        let track_artist = Label::dynamic(|tr: &TrackRow, _| tr.track.artist_name())
            .with_text_size(theme::TEXT_SIZE_SMALL);
        minor.add_child(track_artist);
    }
    if display.album {
        let track_album = Label::dynamic(|tr: &TrackRow, _| tr.track.album_name())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR);
        if display.artist {
            minor.add_default_spacer();
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
        .on_ex_click(move |ctx, event, tr: &mut TrackRow, _| match event.button {
            MouseButton::Left => {
                ctx.submit_notification(cmd::PLAY_TRACK_AT.with(tr.position));
            }
            MouseButton::Right => {
                let menu = make_track_menu(tr);
                ctx.show_context_menu(ContextMenu::new(menu, event.window_pos));
            }
            _ => {}
        })
}

fn make_track_menu(tr: &TrackRow) -> MenuDesc<State> {
    let mut menu = MenuDesc::empty();

    for artist in &tr.track.artists {
        let more_than_one_artist = tr.track.artists.len() > 1;
        let title = if more_than_one_artist {
            LocalizedString::new("menu-item-show-artist-name")
                .with_placeholder(format!("Go To Artist “{}”", artist.name))
        } else {
            LocalizedString::new("menu-item-show-artist").with_placeholder("Go To Artist")
        };
        menu = menu.append(MenuItem::new(
            title,
            cmd::NAVIGATE_TO.with(Navigation::ArtistDetail(artist.id.clone())),
        ));
    }

    if let Some(album) = tr.track.album.as_ref() {
        menu = menu.append(MenuItem::new(
            LocalizedString::new("menu-item-show-album").with_placeholder("Go To Album"),
            cmd::NAVIGATE_TO.with(Navigation::AlbumDetail(album.id.clone())),
        ))
    }

    menu = menu.append(MenuItem::new(
        LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link"),
        cmd::COPY.with(tr.track.link()),
    ));

    menu = menu.append_separator();

    if tr.ctx.is_track_saved(&tr.track) {
        menu = menu.append(MenuItem::new(
            LocalizedString::new("menu-item-remove-from-library")
                .with_placeholder("Remove from Library"),
            cmd::UNSAVE_TRACK.with(tr.track.id.clone()),
        ));
    } else {
        menu = menu.append(MenuItem::new(
            LocalizedString::new("menu-item-save-to-library").with_placeholder("Save to Library"),
            cmd::SAVE_TRACK.with(tr.track.clone()),
        ));
    }

    menu
}
