use druid::{
    kurbo::Circle,
    widget::{CrossAxisAlignment, Flex, Label, LabelText, LineBreaking, List},
    Data, Insets, LensExt, LocalizedString, Menu, MenuItem, Selector, Size, UnitPoint, Widget,
    WidgetExt,
};

use crate::{
    cmd,
    data::{AppState, Artist, ArtistAlbums, ArtistDetail, ArtistLink, Ctx, Nav, WithCtx},
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, RemoteImage},
};

use super::{
    album, theme,
    utils::{self},
};

pub const LOAD_DETAIL: Selector<ArtistLink> = Selector::new("app.artist.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(async_artist_header().padding((theme::grid(1.0), 0.0)))
        .with_child(async_albums_widget().padding((theme::grid(1.0), 0.0)))
}

fn async_artist_header() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, artist_header_widget, || Empty)
        .lens(AppState::artist_detail.then(ArtistDetail::artist))
        .on_command_async(
            LOAD_DETAIL,
            |d| WebApi::global().get_artist(&d.id),
            |_, data, d| data.artist_detail.artist.defer(d),
            |_, data, r| data.artist_detail.artist.update(r),
        )
}

fn artist_header_widget() -> impl Widget<Artist> {
    let size = theme::grid(16.0);

    let artist_image = RemoteImage::new(
        utils::placeholder_widget(),
        move |artist: &Artist, _| artist.image(size, size).map(|image| image.url.clone()),
    )
    .fix_size(size, size)
    .clip(Size::new(size, size).to_rounded_rect(4.0));

    let name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::WordWrap)
        .lens(Artist::name);

    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(artist_image)
        .with_spacer(theme::grid(1.0))
        .with_flex_child(name, 1.0)
        .padding((0.0, theme::grid(1.0)))
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

pub fn artist_widget(horizontal: bool) -> impl Widget<Artist> {
    let (mut artist, artist_image) = if horizontal {
        (Flex::column(), cover_widget(theme::grid(16.0)))
    } else {
        (Flex::row(), cover_widget(theme::grid(6.0)))
    };

    artist = if horizontal {
        artist
            .with_child(artist_image)
            .with_default_spacer()
            .with_child(
                Label::raw()
                    .with_font(theme::UI_FONT_MEDIUM)
                    .align_horizontal(UnitPoint::CENTER)
                    .align_vertical(UnitPoint::TOP)
                    .fix_size(theme::grid(16.0), theme::grid(8.0))
                    .lens(Artist::name),
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

fn albums_widget() -> impl Widget<WithCtx<ArtistAlbums>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(header_widget("Albums"))
        .with_child(List::new(|| album::album_widget(false)).lens(Ctx::map(ArtistAlbums::albums)))
        .with_child(header_widget("Singles"))
        .with_child(List::new(|| album::album_widget(false)).lens(Ctx::map(ArtistAlbums::singles)))
        .with_child(header_widget("Compilations"))
        .with_child(
            List::new(|| album::album_widget(false)).lens(Ctx::map(ArtistAlbums::compilations)),
        )
        .with_child(header_widget("Appears On"))
        .with_child(
            List::new(|| album::album_widget(false)).lens(Ctx::map(ArtistAlbums::appears_on)),
        )
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
