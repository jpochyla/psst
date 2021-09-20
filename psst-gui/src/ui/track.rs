use std::sync::Arc;

use druid::{
    im::Vector,
    kurbo::Line,
    piet::StrokeStyle,
    widget::{
        Controller, ControllerHost, CrossAxisAlignment, Flex, Label, List, ListIter, Painter,
    },
    Data, Env, Event, EventCtx, Lens, LensExt, LocalizedString, Menu, MenuItem, RenderContext,
    TextAlignment, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        Album, AppState, ArtistLink, ArtistTracks, CommonCtx, Library, Nav, PlaybackOrigin,
        PlaybackPayload, PlaylistTracks, Recommendations, RecommendationsRequest, SavedTracks,
        SearchResults, Track, WithCtx,
    },
    widget::MyWidgetExt,
};

use super::{library, theme, utils};

#[derive(Copy, Clone)]
pub struct TrackDisplay {
    pub number: bool,
    pub title: bool,
    pub artist: bool,
    pub album: bool,
    pub popularity: bool,
}

impl TrackDisplay {
    pub fn empty() -> Self {
        TrackDisplay {
            number: false,
            title: false,
            artist: false,
            album: false,
            popularity: false,
        }
    }
}

pub fn tracklist_widget<T>(display: TrackDisplay) -> impl Widget<WithCtx<T>>
where
    T: TrackIter + Data,
{
    ControllerHost::new(List::new(move || track_widget(display)), PlayController)
}

pub trait TrackIter {
    fn origin(&self) -> PlaybackOrigin;
    fn tracks(&self) -> &Vector<Arc<Track>>;
}

impl TrackIter for Arc<Album> {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Album(self.link())
    }

    fn tracks(&self) -> &Vector<Arc<Track>> {
        &self.tracks
    }
}

impl TrackIter for ArtistTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Artist(self.link())
    }

    fn tracks(&self) -> &Vector<Arc<Track>> {
        &self.tracks
    }
}

impl TrackIter for SearchResults {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Search(self.query.clone())
    }

    fn tracks(&self) -> &Vector<Arc<Track>> {
        &self.tracks
    }
}

impl TrackIter for Recommendations {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Recommendations(self.request.clone())
    }

    fn tracks(&self) -> &Vector<Arc<Track>> {
        &self.tracks
    }
}

impl TrackIter for PlaylistTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Playlist(self.link())
    }

    fn tracks(&self) -> &Vector<Arc<Track>> {
        &self.tracks
    }
}

impl TrackIter for SavedTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Library
    }

    fn tracks(&self) -> &Vector<Arc<Track>> {
        &self.tracks
    }
}

impl<T> ListIter<TrackRow> for WithCtx<T>
where
    T: TrackIter + Data,
{
    fn for_each(&self, mut cb: impl FnMut(&TrackRow, usize)) {
        let origin = self.data.origin();
        let tracks = self.data.tracks();
        ListIter::for_each(tracks, |track, index| {
            let d = TrackRow {
                ctx: self.ctx.to_owned(),
                origin: origin.to_owned(),
                track: track.to_owned(),
                position: index,
                is_playing: self.ctx.is_track_playing(track),
            };
            cb(&d, index);
        });
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut TrackRow, usize)) {
        let origin = self.data.origin();
        let tracks = self.data.tracks();
        ListIter::for_each(tracks, |track, index| {
            let mut d = TrackRow {
                ctx: self.ctx.to_owned(),
                origin: origin.to_owned(),
                track: track.to_owned(),
                position: index,
                is_playing: self.ctx.is_track_playing(track),
            };
            cb(&mut d, index);

            // Mutation intentionally ignored.
        });
    }

    fn data_len(&self) -> usize {
        self.data.tracks().len()
    }
}

#[derive(Clone, Data, Lens)]
struct TrackRow {
    ctx: Arc<CommonCtx>,
    track: Arc<Track>,
    origin: PlaybackOrigin,
    position: usize,
    is_playing: bool,
}

struct PlayController;

impl<T, W> Controller<WithCtx<T>, W> for PlayController
where
    T: TrackIter + Data,
    W: Widget<WithCtx<T>>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut WithCtx<T>,
        env: &Env,
    ) {
        match event {
            Event::Notification(note) => {
                if let Some(position) = note.get(cmd::PLAY_TRACK_AT) {
                    let payload = PlaybackPayload {
                        origin: data.data.origin(),
                        tracks: data.data.tracks().to_owned(),
                        position: position.to_owned(),
                    };
                    ctx.submit_command(cmd::PLAY_TRACKS.with(payload));
                    ctx.set_handled();
                }
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

fn track_widget(display: TrackDisplay) -> impl Widget<TrackRow> {
    let mut major = Flex::row();
    let mut minor = Flex::row();

    if display.number {
        let track_number = Label::<Arc<Track>>::dynamic(|track, _| track.track_number.to_string())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_alignment(TextAlignment::Center)
            .center()
            .fix_width(theme::grid(2.0))
            .lens(TrackRow::track);
        major.add_child(track_number);
        major.add_default_spacer();

        // Align the bottom line content.
        minor.add_spacer(theme::grid(2.0));
        minor.add_default_spacer();
    }

    if display.title {
        let track_name = Label::raw()
            .with_font(theme::UI_FONT_MEDIUM)
            .lens(TrackRow::track.then(Track::name.in_arc()));
        major.add_child(track_name);
    }

    if display.artist {
        let track_artists = List::new(|| {
            Label::raw()
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(ArtistLink::name)
        })
        .horizontal()
        .with_spacing(theme::grid(0.5))
        .lens(TrackRow::track.then(Track::artists.in_arc()));
        minor.add_child(track_artists);
    }

    if display.album {
        let track_album = Label::raw()
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(TrackRow::track.then(Track::lens_album_name().in_arc()));
        if display.artist {
            minor.add_default_spacer();
        }
        minor.add_child(track_album);
    }

    let line_painter = Painter::new(|ctx, is_playing, env| {
        const STYLE: StrokeStyle = StrokeStyle::new().dash_pattern(&[1.0, 2.0]);

        let line = Line::new((0.0, 0.0), (ctx.size().width, 0.0));
        let color = if *is_playing {
            env.get(theme::GREY_200)
        } else {
            env.get(theme::GREY_500)
        };
        ctx.stroke_styled(line, &color, 1.0, &STYLE);
    })
    .lens(TrackRow::is_playing)
    .fix_height(1.0);
    major.add_default_spacer();
    major.add_flex_child(line_painter, 1.0);

    if display.popularity {
        let track_popularity = Label::<Arc<Track>>::dynamic(|track, _| {
            track.popularity.map(popularity_stars).unwrap_or_default()
        })
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .lens(TrackRow::track);
        major.add_default_spacer();
        major.add_child(track_popularity);
    }

    let track_duration =
        Label::<Arc<Track>>::dynamic(|track, _| utils::as_minutes_and_seconds(&track.duration))
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(TrackRow::track);
    major.add_default_spacer();
    major.add_child(track_duration);

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(major)
        .with_spacer(2.0)
        .with_child(minor)
        .padding(theme::grid(1.0))
        .link()
        .active(|row, _| row.is_playing)
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, row, _| {
            ctx.submit_notification(cmd::PLAY_TRACK_AT.with(row.position));
        })
        .context_menu(track_row_menu)
}

fn popularity_stars(popularity: u32) -> String {
    const COUNT: usize = 5;

    let popularity_coef = popularity as f32 / 100.0;
    let popular = (COUNT as f32 * popularity_coef).round() as usize;
    let unpopular = COUNT - popular;

    let mut stars = String::with_capacity(COUNT);
    for _ in 0..popular {
        stars.push('★');
    }
    for _ in 0..unpopular {
        stars.push('☆');
    }
    stars
}

fn track_row_menu(row: &TrackRow) -> Menu<AppState> {
    track_menu(&row.track, &row.ctx.library)
}

pub fn track_menu(track: &Arc<Track>, library: &Arc<Library>) -> Menu<AppState> {
    let mut menu = Menu::empty();

    for artist_link in &track.artists {
        let more_than_one_artist = track.artists.len() > 1;
        let title = if more_than_one_artist {
            LocalizedString::new("menu-item-show-artist-name")
                .with_placeholder(format!("Go To Artist “{}”", artist_link.name))
        } else {
            LocalizedString::new("menu-item-show-artist").with_placeholder("Go To Artist")
        };
        menu = menu.entry(
            MenuItem::new(title)
                .command(cmd::NAVIGATE.with(Nav::ArtistDetail(artist_link.to_owned()))),
        );
    }

    if let Some(album_link) = track.album.as_ref() {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-show-album").with_placeholder("Go To Album"),
            )
            .command(cmd::NAVIGATE.with(Nav::AlbumDetail(album_link.to_owned()))),
        );
    }

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-show-recommended")
                .with_placeholder("Show Similar Tracks"),
        )
        .command(cmd::NAVIGATE.with(Nav::Recommendations(Arc::new(
            RecommendationsRequest::for_track(track.id),
        )))),
    );

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Track"),
        )
        .command(cmd::COPY.with(track.url())),
    );

    menu = menu.separator();

    if library.contains_track(track) {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-remove-from-library")
                    .with_placeholder("Remove Track from Library"),
            )
            .command(library::UNSAVE_TRACK.with(track.id)),
        );
    } else {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-save-to-library")
                    .with_placeholder("Save Track to Library"),
            )
            .command(library::SAVE_TRACK.with(track.clone())),
        );
    }

    menu
}
