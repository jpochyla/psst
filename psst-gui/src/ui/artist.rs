use std::sync::Arc;

use crate::{
    cmd,
    data::{
        AppState, Artist, ArtistAlbums, ArtistDetail, ArtistTracks, Cached, CommonCtx, Ctx, Nav,
    },
    ui::{
        album::album_widget,
        theme,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, placeholder_widget, spinner_widget},
    },
    widget::{Async, Clip, LinkExt, RemoteImage},
};
use druid::{
    im::Vector,
    kurbo::Circle,
    widget::{CrossAxisAlignment, Flex, Label, LabelText, List},
    Data, Insets, LensExt, Widget, WidgetExt,
};

pub fn detail_widget() -> impl Widget<AppState> {
    let top_tracks = Async::new(spinner_widget, top_tracks_widget, || {
        error_widget().lens(Ctx::data())
    })
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::artist.then(ArtistDetail::top_tracks),
        )
        .then(Ctx::in_promise()),
    );

    let albums = Async::new(spinner_widget, albums_widget, || {
        error_widget().lens(Ctx::data())
    })
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::artist.then(ArtistDetail::albums),
        )
        .then(Ctx::in_promise()),
    )
    .padding((theme::grid(1.0), 0.0));

    let related_artists = Async::new(spinner_widget, related_widget, error_widget)
        .lens(AppState::artist.then(ArtistDetail::related_artists))
        .padding((theme::grid(1.0), 0.0));

    Flex::column()
        .with_child(top_tracks)
        .with_child(albums)
        .with_child(related_artists)
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
        .on_click(|ctx, artist, _| {
            let nav = Nav::ArtistDetail(artist.link());
            ctx.submit_command(cmd::NAVIGATE.with(nav));
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
        .with_child(label_widget("Albums"))
        .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::albums)))
        .with_child(label_widget("Singles"))
        .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::singles)))
        .with_child(label_widget("Compilations"))
        .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::compilations)))
}

fn related_widget() -> impl Widget<Cached<Vector<Artist>>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(label_widget("Related Artists"))
        .with_child(List::new(artist_widget))
        .lens(Cached::data)
}

fn label_widget<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
    Label::new(text)
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding(Insets::new(0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
}
