use std::sync::Arc;

use druid::{
    im::Vector,
    kurbo::Circle,
    widget::{CrossAxisAlignment, Flex, Label, LabelText, LineBreaking, List},
    Data, Insets, LensExt, LocalizedString, Menu, MenuItem, MouseButton, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        AppState, Artist, ArtistAlbums, ArtistDetail, ArtistLink, ArtistTracks, Cached, CommonCtx,
        Ctx, Nav,
    },
    ui::{
        album::album_widget,
        theme,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, placeholder_widget, spinner_widget},
    },
    webapi::WebApi,
    widget::{Async, Clip, MyWidgetExt, RemoteImage},
};

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(async_top_tracks_widget())
        .with_child(async_albums_widget().padding((theme::grid(1.0), 0.0)))
        .with_child(async_related_artists_widget().padding((theme::grid(1.0), 0.0)))
}

fn async_top_tracks_widget() -> impl Widget<AppState> {
    Async::new(spinner_widget, top_tracks_widget, || {
        error_widget().lens(Ctx::data())
    })
    .on_deferred(|c: &Ctx<Arc<CommonCtx>, ArtistLink>| {
        WebApi::global()
            .get_artist_top_tracks(&c.data.id)
            .map(|tracks| {
                c.replace(ArtistTracks {
                    id: c.data.id.to_owned(),
                    name: c.data.name.to_owned(),
                    tracks,
                })
            })
            .map_err(|err| c.replace(err))
    })
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::artist_detail.then(ArtistDetail::top_tracks),
        )
        .then(Ctx::in_promise()),
    )
}

fn async_albums_widget() -> impl Widget<AppState> {
    Async::new(spinner_widget, albums_widget, || {
        error_widget().lens(Ctx::data())
    })
    .on_deferred(|c: &Ctx<Arc<CommonCtx>, ArtistLink>| {
        WebApi::global()
            .get_artist_albums(&c.data.id)
            .map(|albums| c.replace(albums))
            .map_err(|err| c.replace(err))
    })
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::artist_detail.then(ArtistDetail::albums),
        )
        .then(Ctx::in_promise()),
    )
}

fn async_related_artists_widget() -> impl Widget<AppState> {
    Async::new(spinner_widget, related_widget, error_widget)
        .on_deferred(|link: &ArtistLink| WebApi::global().get_related_artists(&link.id))
        .lens(AppState::artist_detail.then(ArtistDetail::related_artists))
}

pub fn artist_widget() -> impl Widget<Artist> {
    let artist_image = cover_widget(theme::grid(7.0));
    let artist_label = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .lens(Artist::name);
    let artist = Flex::row()
        .with_child(artist_image)
        .with_default_spacer()
        .with_flex_child(artist_label, 1.);
    artist
        .padding(theme::grid(0.5))
        .link()
        .on_ex_click(|ctx, event, artist, _| match event.button {
            MouseButton::Left => {
                ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(artist.link())));
            }
            MouseButton::Right => {
                ctx.show_context_menu(artist_menu(&artist.link()), event.window_pos);
            }
            _ => {}
        })
}

pub fn artist_link_widget() -> impl Widget<ArtistLink> {
    Label::raw()
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_font(theme::UI_FONT_MEDIUM)
        .link()
        .lens(ArtistLink::name)
        .on_ex_click(|ctx, event, link, _| match event.button {
            MouseButton::Left => {
                ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(link.to_owned())));
            }
            MouseButton::Right => {
                ctx.show_context_menu(artist_menu(link), event.window_pos);
            }
            _ => {}
        })
}

pub fn cover_widget(size: f64) -> impl Widget<Artist> {
    let radius = size / 2.0;
    Clip::new(
        Circle::new((radius, radius), radius),
        RemoteImage::new(placeholder_widget(), move |artist: &Artist, _| {
            artist.image(size, size).map(|image| image.url.clone())
        })
        .fix_size(size, size),
    )
}

fn top_tracks_widget() -> impl Widget<Ctx<Arc<CommonCtx>, ArtistTracks>> {
    tracklist_widget(TrackDisplay {
        title: true,
        album: true,
        popularity: true,
        ..TrackDisplay::empty()
    })
}

fn albums_widget() -> impl Widget<Ctx<Arc<CommonCtx>, ArtistAlbums>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(header_widget("Albums"))
        .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::albums)))
        .with_child(header_widget("Singles"))
        .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::singles)))
        .with_child(header_widget("Compilations"))
        .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::compilations)))
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
