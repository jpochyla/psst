use crate::{
    commands,
    data::{Album, AlbumDetail, AlbumType, Artist, Navigation, State},
    ui::{
        theme,
        track::{make_tracklist, TrackDisplay},
        utils::make_placeholder,
    },
    widgets::{HoverExt, Maybe, RemoteImage},
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking, List},
    Widget, WidgetExt,
};

pub fn make_detail() -> impl Widget<State> {
    Maybe::new(make_detail_loaded, make_detail_loading)
        .lens(AlbumDetail::album)
        .lens(State::album)
}

fn make_detail_loaded() -> impl Widget<Album> {
    let album_cover = make_cover(theme::grid(30.0), theme::grid(30.0)).padding(theme::grid(1.0));

    let album_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::WordWrap)
        .lens(Album::name);

    let album_artists = List::new(|| {
        Label::raw()
            .with_line_break_mode(LineBreaking::WordWrap)
            .hover()
            .lens(Artist::name)
            .on_click(|ctx, artist: &mut Artist, _| {
                let nav = Navigation::ArtistDetail(artist.id.clone());
                ctx.submit_command(commands::NAVIGATE_TO.with(nav));
            })
    })
    .lens(Album::artists);

    let album_date = Label::dynamic(|album: &Album, _| album.release());

    let album_genres = List::new(|| Label::raw()).lens(Album::genres);

    let album_type = Label::dynamic(|album: &Album, _| match album.album_type {
        AlbumType::Album => "".to_string(),
        AlbumType::Single => "Single".to_string(),
        AlbumType::Compilation => "Compilation".to_string(),
    });

    let album_tracks = make_tracklist(TrackDisplay {
        title: true,
        artist: false,
        album: false,
    })
    .lens(Album::tracks);

    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::column()
                .with_child(album_cover)
                .with_child(album_name)
                .with_child(album_artists)
                .with_child(album_date)
                .with_child(album_type)
                .with_child(album_genres),
        )
        .with_default_spacer()
        .with_flex_child(album_tracks, 1.0)
}

fn make_detail_loading() -> impl Widget<()> {
    let album_cover = make_placeholder()
        .fix_size(theme::grid(30.0), theme::grid(30.0))
        .padding(theme::grid(1.0));
    let album_tracks = Flex::column()
        .with_child(make_placeholder().fix_height(theme::grid(3.0)))
        .with_spacer(1.0)
        .with_child(make_placeholder().fix_height(theme::grid(3.0)))
        .with_spacer(1.0)
        .with_child(make_placeholder().fix_height(theme::grid(3.0)));
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(album_cover)
        .with_default_spacer()
        .with_flex_child(album_tracks, 1.0)
}

pub fn make_cover(width: f64, height: f64) -> impl Widget<Album> {
    RemoteImage::new(make_placeholder(), move |album: &Album, _| {
        album.image(width, height).map(|image| image.url.clone())
    })
    .fix_size(width, height)
}

pub fn make_album() -> impl Widget<Album> {
    let album_cover = make_cover(theme::grid(7.0), theme::grid(7.0));

    let album_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::Clip)
        .lens(Album::name);

    let album_artists = Label::dynamic(|album: &Album, _| album.artist_list())
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_line_break_mode(LineBreaking::Clip);

    let album_date = Label::dynamic(|album: &Album, _| album.release_year())
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR);

    let album_label = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(album_name)
        .with_spacer(1.0)
        .with_child(album_artists)
        .with_spacer(1.0)
        .with_child(album_date);

    Flex::row()
        .with_child(album_cover)
        .with_default_spacer()
        .with_flex_child(album_label, 1.0)
        .hover()
        .on_click(|ctx, album, _| {
            let nav = Navigation::AlbumDetail(album.id.clone());
            ctx.submit_command(commands::NAVIGATE_TO.with(nav));
        })
}
