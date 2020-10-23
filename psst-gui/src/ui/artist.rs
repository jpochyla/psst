use crate::{
    commands,
    ctx::Ctx,
    data::{Album, Artist, ArtistDetail, Navigation, State, Track, TrackCtx},
    ui::{
        album::make_album,
        theme,
        track::{make_tracklist, TrackDisplay},
        utils::make_placeholder,
    },
    widgets::{HoverExt, Promised, RemoteImage},
};
use druid::{
    im::Vector,
    widget::{Flex, Label, List},
    Data, LensExt, Widget, WidgetExt,
};
use std::sync::Arc;

pub fn make_detail() -> impl Widget<State> {
    let artist = Promised::new(
        || make_detail_loading(),
        || make_detail_loaded(),
        || Label::new("Error"),
    )
    .lens(State::artist.then(ArtistDetail::artist));
    let albums = Promised::new(
        || make_albums_loading(),
        || make_albums_loaded(),
        || Label::new("Error"),
    )
    .lens(State::artist.then(ArtistDetail::albums));
    let top_tracks = Promised::new(
        || make_top_tracks_loading(),
        || make_top_tracks_loaded(),
        || Label::new("Error"),
    )
    .lens(
        Ctx::make(
            State::track_context(),
            State::artist.then(ArtistDetail::top_tracks),
        )
        .then(Ctx::in_promise()),
    );
    Flex::column()
        .with_child(artist)
        .with_default_spacer()
        .with_child(top_tracks)
        .with_default_spacer()
        .with_child(albums)
}

fn make_detail_loaded() -> impl Widget<Artist> {
    Flex::row()
        .with_child(make_cover(theme::grid(12.0), theme::grid(12.0)))
        .with_spacer(theme::grid(2.0))
        .with_child(make_title())
        .center()
}

fn make_detail_loading<T: Data>() -> impl Widget<T> {
    Flex::row()
        .with_child(make_placeholder().fix_size(theme::grid(12.0), theme::grid(12.0)))
        .with_spacer(theme::grid(2.0))
        .with_child(make_placeholder().fix_height(theme::grid(4.0)))
        .center()
}

pub fn make_cover(width: f64, height: f64) -> impl Widget<Artist> {
    RemoteImage::new(make_placeholder(), move |artist: &Artist, _| {
        artist.image(width, height).map(|image| image.url.clone())
    })
    .fix_size(width, height)
}

fn make_title() -> impl Widget<Artist> {
    Label::raw()
        .with_text_size(theme::TEXT_SIZE_LARGE)
        .padding(theme::grid(1.0))
        .lens(Artist::name)
}

fn make_albums_loaded() -> impl Widget<Vector<Album>> {
    List::new(make_album)
}

fn make_albums_loading<T: Data>() -> impl Widget<T> {
    Flex::row()
        .with_child(make_placeholder().fix_height(theme::grid(3.0)))
        .with_spacer(1.0)
        .with_child(make_placeholder().fix_height(theme::grid(3.0)))
        .with_spacer(1.0)
        .with_child(make_placeholder().fix_height(theme::grid(3.0)))
}

fn make_top_tracks_loaded() -> impl Widget<Ctx<TrackCtx, Vector<Arc<Track>>>> {
    make_tracklist(TrackDisplay {
        number: false,
        title: true,
        artist: false,
        album: true,
    })
}

fn make_top_tracks_loading<T: Data>() -> impl Widget<T> {
    Flex::row()
        .with_child(make_placeholder().fix_height(theme::grid(3.0)))
        .with_spacer(1.0)
        .with_child(make_placeholder().fix_height(theme::grid(3.0)))
        .with_spacer(1.0)
        .with_child(make_placeholder().fix_height(theme::grid(3.0)))
}

pub fn make_artist() -> impl Widget<Artist> {
    let artist_image = make_cover(theme::grid(12.0), theme::grid(12.0));
    let artist_label = Label::raw().lens(Artist::name);
    Flex::row()
        .with_child(artist_image)
        .with_default_spacer()
        .with_flex_child(artist_label, 1.)
        .hover()
        .on_click(|ctx, artist, _| {
            let nav = Navigation::ArtistDetail(artist.id.clone());
            ctx.submit_command(commands::NAVIGATE_TO.with(nav));
        })
}
