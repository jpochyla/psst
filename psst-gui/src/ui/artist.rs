use druid::{
    im::Vector,
    kurbo::Circle,
    widget::{CrossAxisAlignment, Flex, Label, LabelText, LineBreaking, List},
    Data, Insets, LensExt, LocalizedString, Menu, MenuItem, Selector, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        AppState, Artist, ArtistAlbums, ArtistDetail, ArtistLink, ArtistTracks, Cached, Ctx, Nav,
        WithCtx,
    },
    webapi::WebApi,
    widget::{Async, MyWidgetExt, RemoteImage},
};

use super::{album, playable, theme, track, utils};

pub const LOAD_DETAIL: Selector<ArtistLink> = Selector::new("app.artist.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column()
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

pub fn artist_widget() -> impl Widget<Artist> {
    let artist_image = cover_widget(theme::grid(7.0));
    let artist_label = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .lens(Artist::name);
    let artist = Flex::row()
        .with_child(artist_image)
        .with_default_spacer()
        .with_flex_child(artist_label, 1.0);
    artist
        .padding(theme::grid(0.5))
        .link()
        .on_click(|ctx, artist, _| {
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
        .on_click(|ctx, link, _| {
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

fn top_tracks_widget() -> impl Widget<WithCtx<ArtistTracks>> {
    playable::list_widget(playable::Display {
        track: track::Display {
            title: true,
            album: true,
            popularity: true,
            ..track::Display::empty()
        },
    })
}

fn albums_widget() -> impl Widget<WithCtx<ArtistAlbums>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(header_widget("Albums"))
        .with_child(List::new(album::album_widget).lens(Ctx::map(ArtistAlbums::albums)))
        .with_child(header_widget("Singles"))
        .with_child(List::new(album::album_widget).lens(Ctx::map(ArtistAlbums::singles)))
        .with_child(header_widget("Compilations"))
        .with_child(List::new(album::album_widget).lens(Ctx::map(ArtistAlbums::compilations)))
        .with_child(header_widget("Appears On"))
        .with_child(List::new(album::album_widget).lens(Ctx::map(ArtistAlbums::appears_on)))
}

fn related_widget() -> impl Widget<Cached<Vector<Artist>>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(header_widget("Related Artists"))
        .with_child(List::new(artist_widget))
        .lens(Cached::data)
}

fn header_widget<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
    Label::new(text)
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding(Insets::new(0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
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
