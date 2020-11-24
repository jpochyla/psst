use crate::{
    cmd,
    data::{Artist, ArtistAlbums, ArtistDetail, Ctx, Navigation, State},
    ui::{
        album::make_album,
        theme,
        track::{make_tracklist, TrackDisplay},
        utils::{make_error, make_loader, make_placeholder},
    },
    widget::{HoverExt, Promised, RemoteImage},
};
use druid::{
    im::Vector,
    widget::{Flex, Label, List},
    LensExt, Widget, WidgetExt,
};

pub fn make_detail() -> impl Widget<State> {
    let top_tracks = Promised::new(
        || make_loader(),
        || {
            make_tracklist(TrackDisplay {
                number: false,
                title: true,
                artist: false,
                album: true,
            })
        },
        || make_error(),
    )
    .lens(
        Ctx::make(
            State::track_ctx,
            State::artist.then(ArtistDetail::top_tracks),
        )
        .then(Ctx::in_promise()),
    );

    let albums = Promised::new(|| make_loader(), || make_albums(), || make_error())
        .lens(State::artist.then(ArtistDetail::albums));

    let related = Promised::new(|| make_loader(), || make_related(), || make_error())
        .lens(State::artist.then(ArtistDetail::related));

    Flex::column()
        .with_child(top_tracks)
        .with_default_spacer()
        .with_child(albums)
        .with_default_spacer()
        .with_child(related)
}

pub fn make_cover(width: f64, height: f64) -> impl Widget<Artist> {
    RemoteImage::new(make_placeholder(), move |artist: &Artist, _| {
        artist.image(width, height).map(|image| image.url.clone())
    })
    .fix_size(width, height)
}

pub fn make_artist() -> impl Widget<Artist> {
    make_artist_with_cover(theme::grid(12.0), theme::grid(12.0))
}

fn make_artist_with_cover(width: f64, height: f64) -> impl Widget<Artist> {
    let artist_image = make_cover(width, height);
    let artist_label = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .lens(Artist::name);
    Flex::row()
        .with_child(artist_image)
        .with_default_spacer()
        .with_flex_child(artist_label, 1.)
        .hover()
        .on_click(|ctx, artist, _| {
            let nav = Navigation::ArtistDetail(artist.id.clone());
            ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
        })
}

fn make_albums() -> impl Widget<ArtistAlbums> {
    Flex::column()
        .with_child(Label::new("Albums").with_text_color(theme::PLACEHOLDER_COLOR))
        .with_default_spacer()
        .with_child(List::new(make_album).lens(ArtistAlbums::albums))
        .with_default_spacer()
        .with_child(Label::new("Singles").with_text_color(theme::PLACEHOLDER_COLOR))
        .with_default_spacer()
        .with_child(List::new(make_album).lens(ArtistAlbums::singles))
        .with_default_spacer()
        .with_child(Label::new("Compilations").with_text_color(theme::PLACEHOLDER_COLOR))
        .with_default_spacer()
        .with_child(List::new(make_album).lens(ArtistAlbums::compilations))
}

fn make_related() -> impl Widget<Vector<Artist>> {
    Flex::column()
        .with_child(Label::new("Related Artists").with_text_color(theme::PLACEHOLDER_COLOR))
        .with_default_spacer()
        .with_child(List::new(make_artist))
}
