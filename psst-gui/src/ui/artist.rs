use crate::{
    cmd,
    data::{Artist, ArtistAlbums, ArtistDetail, ArtistTracks, CommonCtx, Ctx, Nav, State},
    ui::{
        album::make_album,
        theme,
        track::{make_tracklist, TrackDisplay},
        utils::{make_error, make_loader, make_placeholder},
    },
    widget::{Async, Clip, HoverExt, RemoteImage},
};
use druid::{
    im::Vector,
    kurbo::Circle,
    widget::{CrossAxisAlignment, Flex, Label, LabelText, List},
    Data, Insets, LensExt, Widget, WidgetExt,
};

pub fn make_detail() -> impl Widget<State> {
    let top_tracks = Async::new(
        || make_loader(),
        || make_top_tracks(),
        || make_error().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            State::common_ctx,
            State::artist.then(ArtistDetail::top_tracks),
        )
        .then(Ctx::in_promise()),
    );

    let albums = Async::new(
        || make_loader(),
        || make_albums(),
        || make_error().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(State::common_ctx, State::artist.then(ArtistDetail::albums))
            .then(Ctx::in_promise()),
    )
    .padding((theme::grid(0.8), 0.0));

    let related_artists = Async::new(|| make_loader(), || make_related(), || make_error())
        .lens(State::artist.then(ArtistDetail::related_artists))
        .padding((theme::grid(0.8), 0.0));

    Flex::column()
        .with_child(top_tracks)
        .with_child(albums)
        .with_child(related_artists)
}

pub fn make_artist() -> impl Widget<Artist> {
    make_artist_with_cover(theme::grid(7.0))
}

pub fn make_cover(size: f64) -> impl Widget<Artist> {
    let radius = size / 2.0;
    Clip::new(
        Circle::new((radius, radius), radius),
        RemoteImage::new(make_placeholder(), move |artist: &Artist, _| {
            artist.image(size, size).map(|image| image.url.clone())
        })
        .fix_size(size, size),
    )
}

fn make_artist_with_cover(width: f64) -> impl Widget<Artist> {
    let artist_image = make_cover(width);
    let artist_label = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .lens(Artist::name);
    let artist = Flex::row()
        .with_child(artist_image)
        .with_default_spacer()
        .with_flex_child(artist_label, 1.);
    artist.hover().on_click(|ctx, artist, _| {
        let nav = Nav::ArtistDetail(artist.link());
        ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
    })
}

fn make_top_tracks() -> impl Widget<Ctx<CommonCtx, ArtistTracks>> {
    make_tracklist(TrackDisplay {
        title: true,
        album: true,
        popularity: true,
        ..TrackDisplay::empty()
    })
}

fn make_albums() -> impl Widget<Ctx<CommonCtx, ArtistAlbums>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(make_label("Albums"))
        .with_child(List::new(make_album).lens(Ctx::map(ArtistAlbums::albums)))
        .with_child(make_label("Singles"))
        .with_child(List::new(make_album).lens(Ctx::map(ArtistAlbums::singles)))
        .with_child(make_label("Compilations"))
        .with_child(List::new(make_album).lens(Ctx::map(ArtistAlbums::compilations)))
}

fn make_related() -> impl Widget<Vector<Artist>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(make_label("Related Artists"))
        .with_child(List::new(make_artist))
}

fn make_label<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
    Label::new(text)
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding(Insets::new(0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
}
