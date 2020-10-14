use crate::{
    commands,
    data::{Navigation, State, Track},
    ui::theme,
    widgets::HoverExt,
};
use druid::{
    im::Vector,
    kurbo::Line,
    lens::Map,
    piet::StrokeStyle,
    widget::{Controller, CrossAxisAlignment, Flex, Label, List, Painter},
    ContextMenu, Data, Env, Event, EventCtx, Lens, LensExt, LocalizedString, MenuDesc, MenuItem,
    MouseButton, MouseEvent, RenderContext, Widget, WidgetExt, WidgetId,
};
use std::sync::Arc;

#[derive(Copy, Clone)]
pub struct TrackDisplay {
    pub title: bool,
    pub artist: bool,
    pub album: bool,
}

pub fn make_tracklist(mode: TrackDisplay) -> impl Widget<Vector<Arc<Track>>> {
    let id = WidgetId::next();

    List::new(move || make_track(mode, id))
        .lens(Map::new(
            |t: &Vector<Arc<Track>>| {
                t.into_iter()
                    .cloned()
                    .enumerate()
                    .map(|(position, track)| EnumTrack { position, track })
                    .collect()
            },
            |_t: &mut Vector<Arc<Track>>, _enum_t: Vector<EnumTrack>| {
                // Ignore mutation.
            },
        ))
        .controller(PlayController)
        .with_id(id)
}

struct PlayController;

impl<W> Controller<Vector<Arc<Track>>, W> for PlayController
where
    W: Widget<Vector<Arc<Track>>>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        tracks: &mut Vector<Arc<Track>>,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) => {
                if let Some(&position) = cmd.get(commands::PLAY_TRACK_AT) {
                    ctx.submit_command(commands::PLAY_TRACKS.with((tracks.clone(), position)));
                }
            }
            _ => child.event(ctx, event, tracks, env),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct EnumTrack {
    position: usize,
    track: Arc<Track>,
}

pub fn make_track(display: TrackDisplay, play_ctrl: WidgetId) -> impl Widget<EnumTrack> {
    let track_duration = Label::dynamic(|enum_track: &EnumTrack, _| {
        enum_track.track.duration.as_minutes_and_seconds()
    })
    .with_text_size(theme::TEXT_SIZE_SMALL)
    .with_text_color(theme::PLACEHOLDER_COLOR);

    let line_painter = Painter::new(move |ctx, _, _| {
        let size = ctx.size();
        let line = Line::new((0.0, size.height), (size.width, size.height));
        ctx.stroke_styled(
            line,
            &theme::GREY_5,
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

    if display.title {
        let track_name = Label::raw()
            .with_font(theme::UI_FONT_MEDIUM)
            .lens(EnumTrack::track.then(Track::name.in_arc()));
        major.add_child(track_name);
    }
    if display.artist {
        let track_artist =
            Label::dynamic(|enum_track: &EnumTrack, _| enum_track.track.artist_name())
                .with_text_size(theme::TEXT_SIZE_SMALL);
        minor.add_child(track_artist);
    }
    if display.album {
        let track_album = Label::dynamic(|enum_track: &EnumTrack, _| enum_track.track.album_name())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR);
        minor.add_child(Label::new(": ").with_text_color(theme::GREY_5));
        minor.add_child(track_album);
    }
    major.add_default_spacer();
    major.add_flex_child(line_painter, 1.0);
    major.add_default_spacer();
    major.add_child(track_duration.align_right());

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(major)
        .with_child(minor)
        .padding(theme::grid(0.8))
        .hover()
        .on_ex_click(
            MouseButton::Right,
            |ctx, event, enum_track: &mut EnumTrack, _| {
                show_track_menu(ctx, event, &enum_track.track);
            },
        )
        .on_click(move |ctx, enum_track: &mut EnumTrack, _| {
            ctx.submit_command(
                commands::PLAY_TRACK_AT
                    .with(enum_track.position)
                    .to(play_ctrl),
            );
        })
}

fn show_track_menu(ctx: &mut EventCtx, event: &MouseEvent, track: &Track) {
    let desc = make_track_menu(track);
    let menu = ContextMenu::new(desc, event.window_pos);
    ctx.show_context_menu(menu);
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
