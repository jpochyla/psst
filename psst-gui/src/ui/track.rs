use std::{mem, sync::Arc};

use druid::{
    im::Vector,
    kurbo::Line,
    lens::Map,
    piet::StrokeStyle,
    widget::{
        Controller, ControllerHost, CrossAxisAlignment, Either, Flex, Label, LineBreaking, List,
        ListIter, Painter, ViewSwitcher,
    },
    Data, Env, Event, EventCtx, Lens, LensExt, LocalizedString, Menu, MenuItem, RenderContext,
    Selector, Size, TextAlignment, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        Album, AppState, ArtistLink, ArtistTracks, CommonCtx, Episode, FindQuery, Library,
        MatchFindQuery, Nav, PlaybackItem, PlaybackOrigin, PlaybackPayload, PlaylistAddTrack,
        PlaylistTracks, Recommendations, RecommendationsRequest, SavedTracks, SearchResults,
        ShowEpisodes, Track, WithCtx,
    },
    ui::playlist,
    widget::{Empty, MyWidgetExt, RemoteImage},
};

use super::{
    find::{Find, Findable},
    library,
    show::episode_cover_widget,
    theme,
    utils::{self, placeholder_widget},
};

#[derive(Copy, Clone)]
pub struct TrackDisplay {
    pub number: bool,
    pub title: bool,
    pub artist: bool,
    pub album: bool,
    pub cover: bool,
    pub popularity: bool,
}

impl TrackDisplay {
    pub fn empty() -> Self {
        TrackDisplay {
            number: false,
            title: false,
            artist: false,
            album: false,
            cover: false,
            popularity: false,
        }
    }
}

pub fn tracklist_widget<T>(display: TrackDisplay) -> impl Widget<WithCtx<T>>
where
    T: PlaybackItemIter + Data,
{
    let list = List::new(move || playback_item_widget(display));
    ControllerHost::new(list, PlayController)
}

pub fn findable_tracklist_widget<T>(
    display: TrackDisplay,
    selector: Selector<Find>,
) -> impl Widget<WithCtx<T>>
where
    T: PlaybackItemIter + Data,
{
    let list = List::new(move || Findable::new(playback_item_widget(display), selector));
    ControllerHost::new(list, PlayController)
}

#[derive(Clone, Data, Lens)]
struct Row<T> {
    item: T,
    ctx: Arc<CommonCtx>,
    position: usize,
    is_playing: bool,
}

impl<T> Row<T> {
    fn with<U>(&self, item: U) -> Row<U> {
        Row {
            item,
            ctx: self.ctx.clone(),
            position: self.position,
            is_playing: self.is_playing,
        }
    }
}

pub trait PlaybackItemIter {
    fn origin(&self) -> PlaybackOrigin;
    fn len(&self) -> usize;
    fn for_each(&self, cb: impl FnMut(PlaybackItem, usize));
    fn collect(&self) -> Vector<PlaybackItem> {
        let mut items = Vector::new();
        self.for_each(|item, _| items.push_back(item));
        items
    }
}

impl PlaybackItemIter for Arc<Album> {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Album(self.link())
    }

    fn for_each(&self, mut cb: impl FnMut(PlaybackItem, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(PlaybackItem::Track(track.to_owned()), position);
        }
    }

    fn len(&self) -> usize {
        self.tracks.len()
    }
}

impl PlaybackItemIter for PlaylistTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Playlist(self.link())
    }

    fn for_each(&self, mut cb: impl FnMut(PlaybackItem, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(PlaybackItem::Track(track.to_owned()), position);
        }
    }

    fn len(&self) -> usize {
        self.tracks.len()
    }
}

impl PlaybackItemIter for ArtistTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Artist(self.link())
    }

    fn for_each(&self, mut cb: impl FnMut(PlaybackItem, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(PlaybackItem::Track(track.to_owned()), position);
        }
    }

    fn len(&self) -> usize {
        self.tracks.len()
    }
}

impl PlaybackItemIter for SavedTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Library
    }

    fn for_each(&self, mut cb: impl FnMut(PlaybackItem, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(PlaybackItem::Track(track.to_owned()), position);
        }
    }

    fn len(&self) -> usize {
        self.tracks.len()
    }
}

impl PlaybackItemIter for SearchResults {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Search(self.query.clone())
    }

    fn for_each(&self, mut cb: impl FnMut(PlaybackItem, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(PlaybackItem::Track(track.to_owned()), position);
        }
    }

    fn len(&self) -> usize {
        self.tracks.len()
    }
}

impl PlaybackItemIter for Recommendations {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Recommendations(self.request.clone())
    }

    fn for_each(&self, mut cb: impl FnMut(PlaybackItem, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(PlaybackItem::Track(track.to_owned()), position);
        }
    }

    fn len(&self) -> usize {
        self.tracks.len()
    }
}

impl PlaybackItemIter for ShowEpisodes {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Show(self.show.clone())
    }

    fn for_each(&self, mut cb: impl FnMut(PlaybackItem, usize)) {
        for (position, episode) in self.episodes.iter().enumerate() {
            cb(PlaybackItem::Episode(episode.to_owned()), position);
        }
    }

    fn len(&self) -> usize {
        self.episodes.len()
    }
}

impl<T> ListIter<Row<PlaybackItem>> for WithCtx<T>
where
    T: PlaybackItemIter + Data,
{
    fn for_each(&self, mut cb: impl FnMut(&Row<PlaybackItem>, usize)) {
        self.data.for_each(|item, position| {
            cb(
                &Row {
                    is_playing: self.ctx.is_playing(&item),
                    ctx: self.ctx.to_owned(),
                    item,
                    position,
                },
                position,
            )
        });
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut Row<PlaybackItem>, usize)) {
        self.data.for_each(|item, position| {
            cb(
                &mut Row {
                    is_playing: self.ctx.is_playing(&item),
                    ctx: self.ctx.to_owned(),
                    item,
                    position,
                },
                position,
            )
        });
    }

    fn data_len(&self) -> usize {
        self.data.len()
    }
}

impl MatchFindQuery for Row<PlaybackItem> {
    fn matches_query(&self, q: &FindQuery) -> bool {
        match &self.item {
            PlaybackItem::Track(track) => {
                q.matches_str(&track.name)
                    || track.album.iter().any(|a| q.matches_str(&a.name))
                    || track.artists.iter().any(|a| q.matches_str(&a.name))
            }
            PlaybackItem::Episode(episode) => false,
        }
    }
}

struct PlayController;

impl<T, W> Controller<WithCtx<T>, W> for PlayController
where
    T: PlaybackItemIter + Data,
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
                        items: data.data.collect(),
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

fn playback_item_widget(display: TrackDisplay) -> impl Widget<Row<PlaybackItem>> {
    ViewSwitcher::new(
        |row: &Row<PlaybackItem>, _| mem::discriminant(&row.item),
        move |_, row: &Row<PlaybackItem>, _| match row.item.clone() {
            PlaybackItem::Track(track) => track_widget(display)
                .lens(Map::new(
                    move |pb: &Row<PlaybackItem>| pb.with(track.clone()),
                    |_, _| {
                        // Ignore mutation.
                    },
                ))
                .boxed(),
            PlaybackItem::Episode(episode) => {
                episode_widget()
                    .lens(Map::new(
                        move |pb: &Row<PlaybackItem>| pb.with(episode.clone()),
                        |_, _| {
                            // Ignore mutation.
                        },
                    ))
                    .boxed()
            }
        },
    )
}

fn track_widget(display: TrackDisplay) -> impl Widget<Row<Arc<Track>>> {
    let mut main_row = Flex::row();
    let mut major = Flex::row();
    let mut minor = Flex::row();

    if display.number {
        let track_number = Label::<Arc<Track>>::dynamic(|track, _| track.track_number.to_string())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_alignment(TextAlignment::Center)
            .center()
            .fix_width(theme::grid(2.0))
            .lens(Row::item);
        major.add_child(track_number);
        major.add_default_spacer();

        // Align the bottom line content.
        minor.add_spacer(theme::grid(2.0));
        minor.add_default_spacer();
    }

    if display.cover {
        let album_cover = rounded_cover_widget(theme::grid(4.0))
            .padding_right(theme::grid(1.0)) // Instead of `add_default_spacer`.
            .lens(Row::item);
        main_row.add_child(Either::new(
            |row, _| row.ctx.show_track_cover,
            album_cover,
            Empty,
        ));
    }

    if display.title {
        let track_name = Label::raw()
            .with_font(theme::UI_FONT_MEDIUM)
            .lens(Row::item.then(Track::name.in_arc()));
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
        .lens(Row::item.then(Track::artists.in_arc()));
        minor.add_child(track_artists);
    }

    if display.album {
        let track_album = Label::raw()
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(Row::item.then(Track::lens_album_name().in_arc()));
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
    .lens(Row::is_playing)
    .fix_height(1.0);
    major.add_default_spacer();
    major.add_flex_child(line_painter, 1.0);

    if display.popularity {
        let track_popularity = Label::<Arc<Track>>::dynamic(|track, _| {
            track.popularity.map(popularity_stars).unwrap_or_default()
        })
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .lens(Row::item);
        major.add_default_spacer();
        major.add_child(track_popularity);
    }

    let track_duration =
        Label::<Arc<Track>>::dynamic(|track, _| utils::as_minutes_and_seconds(&track.duration))
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(Row::item);
    major.add_default_spacer();
    major.add_child(track_duration);

    main_row
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(major)
                .with_spacer(2.0)
                .with_child(minor),
            1.0,
        )
        .padding(theme::grid(1.0))
        .link()
        .active(|row, _| row.is_playing)
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, row, _| {
            ctx.submit_notification(cmd::PLAY_TRACK_AT.with(row.position));
        })
        .context_menu(track_row_menu)
}

fn episode_widget() -> impl Widget<Row<Arc<Episode>>> {
    let cover = episode_cover_widget(theme::grid(4.0));

    let name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::WordWrap)
        .lens(Episode::name.in_arc());

    let description = Label::raw()
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_line_break_mode(LineBreaking::WordWrap)
        .lens(Episode::description.in_arc());

    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(cover)
        .with_default_spacer()
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(name)
                .with_default_spacer()
                .with_child(description),
            1.0,
        )
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .lens(Row::item)
        .on_click(|ctx, row, _| {
            ctx.submit_notification(cmd::PLAY_TRACK_AT.with(row.position));
        })
}

fn cover_widget(size: f64) -> impl Widget<Arc<Track>> {
    RemoteImage::new(placeholder_widget(), move |track: &Arc<Track>, _| {
        track
            .album
            .as_ref()
            .and_then(|al| al.image(size, size).map(|image| image.url.clone()))
    })
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Arc<Track>> {
    cover_widget(size).clip(Size::new(size, size).to_rounded_rect(4.0))
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

fn track_row_menu(row: &Row<Arc<Track>>) -> Menu<AppState> {
    track_menu(&row.item, &row.ctx.library)
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

    let mut playlist_menu = Menu::new(
        LocalizedString::new("menu-item-add-to-playlist").with_placeholder("Add to Playlist"),
    );
    for playlist in library.writable_playlists() {
        playlist_menu = playlist_menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-save-to-playlist")
                    .with_placeholder(format!("{}", playlist.name)),
            )
            .command(playlist::ADD_TRACK.with(PlaylistAddTrack {
                link: playlist.link(),
                track_id: track.id,
            })),
        );
    }
    menu = menu.entry(playlist_menu);

    menu
}
