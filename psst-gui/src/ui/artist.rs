use std::sync::Arc;

use druid::{
    im::Vector,
    kurbo::Circle,
    widget::{
        CrossAxisAlignment, Either, Flex, Label, LineBreaking, List, MainAxisAlignment, Scroll,
    },
    Lens, LensExt, LocalizedString, Menu, MenuItem, Selector, Size, UnitPoint, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        Album, AppState, Artist, ArtistAlbums, ArtistDetail, ArtistInfo, ArtistLink, ArtistTracks,
        Cached, Ctx, Nav, WithCtx,
    },
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, RemoteImage},
};

use super::{album, playable, theme, track, utils};

pub const LOAD_DETAIL: Selector<ArtistLink> = Selector::new("app.artist.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(async_artist_info().padding((theme::grid(1.0), 0.0)))
        .with_child(async_top_tracks_widget())
        .with_child(async_albums_widget().padding((theme::grid(1.0), 0.0)))
        .with_child(async_related_widget().padding((theme::grid(1.0), 0.0)))
}

fn async_top_tracks_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        top_tracks_widget,
        utils::error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::artist_detail.then(ArtistDetail::top_tracks),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_DETAIL,
        |d| WebApi::global().get_artist_top_tracks(&d.id),
        |_, data, d| data.artist_detail.top_tracks.defer(d),
        |_, data, (d, r)| {
            let r = r.map(|tracks| ArtistTracks {
                id: d.id.clone(),
                name: d.name.clone(),
                tracks,
            });
            data.artist_detail.top_tracks.update((d, r))
        },
    )
}

fn async_albums_widget() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, albums_widget, utils::error_widget)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::artist_detail.then(ArtistDetail::albums),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_DETAIL,
            |d| WebApi::global().get_artist_albums(&d.id),
            |_, data, d| data.artist_detail.albums.defer(d),
            |_, data, r| data.artist_detail.albums.update(r),
        )
}

fn async_artist_info() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, artist_info_widget, || Empty)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::artist_detail.then(ArtistDetail::artist_info),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_DETAIL,
            |d| WebApi::global().get_artist_info(&d.id),
            |_, data, d| data.artist_detail.artist_info.defer(d),
            |_, data, r| data.artist_detail.artist_info.update(r),
        )
}

fn async_related_widget() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, related_widget, utils::error_widget)
        .lens(AppState::artist_detail.then(ArtistDetail::related_artists))
        .on_command_async(
            LOAD_DETAIL,
            |d| WebApi::global().get_related_artists(&d.id),
            |_, data, d| data.artist_detail.related_artists.defer(d),
            |_, data, r| data.artist_detail.related_artists.update(r),
        )
}

pub fn artist_widget(horizontal: bool) -> impl Widget<Artist> {
    let (mut artist, artist_image) = if horizontal {
        (Flex::column(), cover_widget(theme::grid(16.0)))
    } else {
        (Flex::row(), cover_widget(theme::grid(8.0)))
    };

    artist = if horizontal {
        let artist_name = Label::dynamic(|artist: &Artist, _| {
            let name = artist.name.as_ref();
            if name.chars().count() > 20 {
                format!("{}...", name.chars().take(20).collect::<String>())
            } else {
                name.to_string()
            }
        })
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::Clip)
        .align_horizontal(UnitPoint::CENTER)
        .fix_width(theme::grid(16.0));

        artist
            .with_child(artist_image)
            .with_spacer(theme::grid(1.0))
            .with_child(
                Flex::column()
                    .main_axis_alignment(MainAxisAlignment::Start)
                    .with_child(artist_name)
                    .align_horizontal(UnitPoint::CENTER)
                    .align_vertical(UnitPoint::TOP)
                    .fix_size(theme::grid(16.0), theme::grid(6.5)),
            )
    } else {
        artist
            .with_child(artist_image)
            .with_default_spacer()
            .with_flex_child(
                Label::raw()
                    .with_font(theme::UI_FONT_MEDIUM)
                    .align_left()
                    .lens(Artist::name),
                1.0,
            )
    };

    artist
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, artist, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(artist.link())));
        })
        .context_menu(|artist| artist_menu(&artist.link()))
}

pub fn link_widget() -> impl Widget<ArtistLink> {
    Label::raw()
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_font(theme::UI_FONT_MEDIUM)
        .link()
        .lens(ArtistLink::name)
        .on_left_click(|ctx, _, link, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(link.to_owned())));
        })
        .context_menu(artist_menu)
}

pub fn cover_widget(size: f64) -> impl Widget<Artist> {
    let radius = size / 2.0;
    RemoteImage::new(utils::placeholder_widget(), move |artist: &Artist, _| {
        artist.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
    .clip(Circle::new((radius, radius), radius))
}

fn artist_info_widget() -> impl Widget<WithCtx<ArtistInfo>> {
    let size = theme::grid(16.0);

    let artist_image = RemoteImage::new(
        utils::placeholder_widget(),
        move |artist: &ArtistInfo, _| Some(artist.main_image.clone()),
    )
    .fix_size(size, size)
    .clip(Size::new(size, size).to_rounded_rect(4.0))
    .lens(Ctx::data());

    let biography = Scroll::new(
        Label::new(|data: &ArtistInfo, _env: &_| data.bio.clone())
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_size(theme::TEXT_SIZE_NORMAL)
            .lens(Ctx::data()),
    )
    .vertical()
    .expand_height()
    .fix_height(size);

    let stats = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::dynamic(|info: &ArtistInfo, _| {
                format!(
                    "Followers: {}",
                    format_number_with_commas(info.stats.followers)
                )
            })
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(Ctx::data()),
        )
        .with_default_spacer()
        .with_child(
            Label::dynamic(|info: &ArtistInfo, _| {
                format!(
                    "Monthly listeners: {}",
                    format_number_with_commas(info.stats.monthly_listeners)
                )
            })
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(Ctx::data()),
        )
        .with_default_spacer()
        .with_child(Either::new(
            |data: &WithCtx<ArtistInfo>, _| data.data.stats.world_rank <= 0,
            Empty,
            Label::dynamic(|info: &ArtistInfo, _| {
                format!(
                    "World rank: #{}",
                    format_number_with_commas(info.stats.world_rank)
                )
            })
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(Ctx::data()),
        ));

    Flex::row()
        .with_child(artist_image)
        .with_spacer(theme::grid(1.0))
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(biography)
                .with_spacer(theme::grid(1.0))
                .with_child(stats),
            1.0,
        )
        .context_menu(|artist| artist_info_menu(&artist.data))
}

fn top_tracks_widget() -> impl Widget<WithCtx<ArtistTracks>> {
    Either::new(
        |data: &WithCtx<ArtistTracks>, _| data.data.tracks.is_empty(),
        Empty,
        Flex::column()
            .with_child(utils::header_widget("Popular"))
            .with_child(playable::list_widget(playable::Display {
                track: track::Display {
                    title: true,
                    album: true,
                    popularity: true,
                    cover: true,
                    ..track::Display::empty()
                },
            })),
    )
}

fn albums_widget() -> impl Widget<WithCtx<ArtistAlbums>> {
    Flex::column()
        .with_child(album_section("Albums", ArtistAlbums::albums))
        .with_child(album_section("Singles", ArtistAlbums::singles))
        .with_child(album_section("Compilations", ArtistAlbums::compilations))
        .with_child(album_section("Appears On", ArtistAlbums::appears_on))
}

fn album_section<L>(title: &'static str, lens: L) -> impl Widget<WithCtx<ArtistAlbums>>
where
    L: Lens<ArtistAlbums, Vector<Arc<Album>>> + Clone + 'static,
{
    let lens_clone = lens.clone();
    Either::new(
        move |data: &WithCtx<ArtistAlbums>, _| lens_clone.get(&data.data).is_empty(),
        Empty,
        Flex::column()
            .with_child(utils::header_widget(title))
            .with_child(List::new(|| album::album_widget(false)).lens(Ctx::map(lens))),
    )
}

fn related_widget() -> impl Widget<Cached<Vector<Artist>>> {
    Either::new(
        |data: &Cached<Vector<Artist>>, _| data.data.is_empty(),
        Empty,
        Flex::column()
            .with_child(utils::header_widget("Related Artists"))
            .with_child(List::new(|| artist_widget(false)).lens(Cached::data)),
    )
}

fn format_number_with_commas(n: i64) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }
    s.chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .rev()
        .map(|chunk| chunk.iter().rev().collect::<String>())
        .collect::<Vec<_>>()
        .join(",")
}

fn artist_info_menu(artist: &ArtistInfo) -> Menu<AppState> {
    let mut menu = Menu::empty();

    for artist_link in &artist.artist_links {
        let platform = if artist_link.contains("wikipedia.org") {
            "Wikipedia"
        } else {
            artist_link
                .strip_prefix("https://")
                .unwrap_or(artist_link)
                .split('.')
                .next()
                .unwrap_or("Unknown")
        };

        let title = LocalizedString::new("menu-item-go-to-social").with_placeholder(format!(
            "Go to their {}",
            platform
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .collect::<String>()
                + &platform[1..]
        ));

        menu =
            menu.entry(MenuItem::new(title).command(cmd::GO_TO_URL.with(artist_link.to_owned())));
    }

    menu
}

fn artist_menu(artist: &ArtistLink) -> Menu<AppState> {
    let mut menu = Menu::empty();

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Artist"),
        )
        .command(cmd::COPY.with(artist.url())),
    );

    menu
}
