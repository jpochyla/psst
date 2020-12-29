use crate::{
    cmd,
    data::{Artist, ArtistAlbums, ArtistDetail, CommonCtx, Ctx, Nav, State},
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
    widget::{CrossAxisAlignment, Flex, Label, List},
    LensExt, Widget, WidgetExt,
};

pub fn make_detail() -> impl Widget<State> {
    let top_tracks = Async::new(
        || make_loader(),
        || {
            make_tracklist(TrackDisplay {
                number: false,
                title: true,
                artist: false,
                album: true,
            })
        },
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
        .with_default_spacer()
        .with_child(albums)
        .with_default_spacer()
        .with_child(related_artists)
}

pub fn make_cover(width: f64, height: f64) -> impl Widget<Artist> {
    let image = RemoteImage::new(make_placeholder(), move |artist: &Artist, _| {
        artist.image(width, height).map(|image| image.url.clone())
    })
    .fix_size(width, height);
    Clip::new(Circle::new((width / 2.0, height / 2.0), width / 2.0), image)
}

pub fn make_artist() -> impl Widget<Artist> {
    make_artist_with_cover(theme::grid(7.0), theme::grid(7.0))
}

fn make_artist_with_cover(width: f64, height: f64) -> impl Widget<Artist> {
    let artist_image = make_cover(width, height);
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

fn make_albums() -> impl Widget<Ctx<CommonCtx, ArtistAlbums>> {
    let label = |text| {
        Label::new(text)
            .with_font(theme::UI_FONT_MEDIUM)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL)
    };
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(label("Albums"))
        .with_default_spacer()
        .with_child(List::new(make_album).lens(Ctx::map(ArtistAlbums::albums)))
        .with_default_spacer()
        .with_child(label("Singles"))
        .with_default_spacer()
        .with_child(List::new(make_album).lens(Ctx::map(ArtistAlbums::singles)))
        .with_default_spacer()
        .with_child(label("Compilations"))
        .with_default_spacer()
        .with_child(List::new(make_album).lens(Ctx::map(ArtistAlbums::compilations)))
}

fn make_related() -> impl Widget<Vector<Artist>> {
    let label = |text| {
        Label::new(text)
            .with_font(theme::UI_FONT_MEDIUM)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL)
    };
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(label("Related Artists"))
        .with_default_spacer()
        .with_child(List::new(make_artist))
}
