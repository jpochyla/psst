use std::sync::Arc;

use druid::{
    im::Vector,
    kurbo::Circle,
    widget::{Either, Flex, Label, LineBreaking, List, Scroll},
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

use super::{
    album, playable, theme, track,
    utils::{self},
};

pub const LOAD_DETAIL: Selector<ArtistLink> = Selector::new("app.artist.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(async_artist_info())
        .with_child(async_top_tracks_widget().padding((theme::grid(1.0), 0.0)))
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
    Async::new(utils::spinner_widget, artist_info_view_switcher, utils::error_widget)
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

fn artist_info_view_switcher() -> impl Widget<WithCtx<ArtistInfo>> {
    Either::new(
        |data: &WithCtx<ArtistInfo>, _| data.data.bio.is_empty(),
        Empty,
        Flex::column()
            .with_child(artist_info_widget())
            .padding((theme::grid(1.0), 0.0)),
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
        (Flex::row(), cover_widget(theme::grid(6.0)))
    };

    artist = if horizontal {
        artist
            .with_child(artist_image)
            .with_spacer(theme::grid(1.0))
            .with_child(
                Label::dynamic(|artist: &Artist, _| {
                    let name = artist.name.as_ref();
                    if name.chars().count() > 20 {
                        format!("{:.<20}...", name)
                    } else {
                        name.to_string()
                    }
                })
                    .with_font(theme::UI_FONT_MEDIUM)
                    .with_line_break_mode(LineBreaking::Clip)
                    .align_horizontal(UnitPoint::CENTER)
                    .fix_width(theme::grid(16.0)), // Set a fixed width for the text, same as image
            )
    } else {
        artist
            .with_child(artist_image)
            .with_default_spacer()
            .with_flex_child(
                Label::raw()
                    .with_font(theme::UI_FONT_MEDIUM)
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
    .fix_height(size);

    Flex::row()
        .with_child(artist_image)
        .with_spacer(theme::grid(1.0))
        .with_flex_child(biography, 1.0)
        .context_menu(|artist| artist_info_menu(&artist.data))
        .padding((0.0, theme::grid(1.0))) // Keep overall vertical padding
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
        .with_child(album_section(
            "Compilations",
            ArtistAlbums::compilations,
        ))
        .with_child(album_section("Appears On", ArtistAlbums::appears_on))
}

fn album_section<L>(
    title: &'static str,
    lens: L,
) -> impl Widget<WithCtx<ArtistAlbums>>
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
