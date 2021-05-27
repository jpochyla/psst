use crate::{
    cmd,
    data::{Album, AlbumDetail, ArtistLink, Cached, CommonCtx, Ctx, Nav, State},
    ui::{
        theme,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, placeholder_widget, spinner_widget},
    },
    widget::{Async, Clip, LinkExt, RemoteImage},
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking, List},
    LensExt, LocalizedString, Menu, MenuItem, MouseButton, Size, Widget, WidgetExt,
};

pub fn detail_widget() -> impl Widget<State> {
    Async::new(
        || spinner_widget(),
        || loaded_detail_widget(),
        || error_widget().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(State::common_ctx, State::album.then(AlbumDetail::album)).then(Ctx::in_promise()),
    )
}

fn loaded_detail_widget() -> impl Widget<Ctx<CommonCtx, Cached<Album>>> {
    let album_cover = rounded_cover_widget(theme::grid(10.0));

    let album_artists = List::new(|| {
        Label::raw()
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_font(theme::UI_FONT_MEDIUM)
            .link()
            .lens(ArtistLink::name)
            .on_click(|ctx, artist_link: &mut ArtistLink, _| {
                let nav = Nav::ArtistDetail(artist_link.to_owned());
                ctx.submit_command(cmd::NAVIGATE.with(nav));
            })
    })
    .lens(Album::artists);

    let album_date = Label::dynamic(|album: &Album, _| album.release())
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR);

    let album_label = Label::raw()
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .lens(Album::label);

    let album_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(album_artists)
        .with_default_spacer()
        .with_child(album_date)
        .with_default_spacer()
        .with_child(album_label)
        .padding(theme::grid(1.0));

    let album_tracks = tracklist_widget(TrackDisplay {
        number: true,
        title: true,
        ..TrackDisplay::empty()
    });

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::row()
                .with_spacer(theme::grid(4.0))
                .with_child(album_cover)
                .with_default_spacer()
                .with_child(album_info)
                .lens(Ctx::data()),
        )
        .with_spacer(theme::grid(1.0))
        .with_child(album_tracks)
        .lens(Ctx::map(Cached::data))
}

fn cover_widget(size: f64) -> impl Widget<Album> {
    RemoteImage::new(placeholder_widget(), move |album: &Album, _| {
        album.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Album> {
    // TODO: Take the radius from theme.
    Clip::new(
        Size::new(size, size).to_rounded_rect(4.0),
        cover_widget(size),
    )
}

pub fn album_widget() -> impl Widget<Ctx<CommonCtx, Album>> {
    let album_cover = cover_widget(theme::grid(7.0));

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

    let album_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(album_name)
        .with_spacer(1.0)
        .with_child(album_artists)
        .with_spacer(1.0)
        .with_child(album_date);

    let album = Flex::row()
        .with_child(album_cover)
        .with_default_spacer()
        .with_flex_child(album_info, 1.0)
        .lens(Ctx::data());

    album
        .link()
        .on_ex_click(
            move |ctx, event, album: &mut Ctx<CommonCtx, Album>, _| match event.button {
                MouseButton::Left => {
                    let nav = Nav::AlbumDetail(album.data.link());
                    ctx.submit_command(cmd::NAVIGATE.with(nav));
                }
                MouseButton::Right => {
                    ctx.show_context_menu(album_menu(&album), event.window_pos);
                }
                _ => {}
            },
        )
}

fn album_menu(album: &Ctx<CommonCtx, Album>) -> Menu<State> {
    let mut menu = Menu::empty();

    for artist_link in &album.data.artists {
        let more_than_one_artist = album.data.artists.len() > 1;
        let title = if more_than_one_artist {
            LocalizedString::new("menu-item-show-artist-name")
                .with_placeholder(format!("Go To Artist “{}”", artist_link.name))
        } else {
            LocalizedString::new("menu-item-show-artist").with_placeholder("Go To Artist")
        };
        menu = menu.entry(
            MenuItem::new(title)
                .command(cmd::NAVIGATE.with(Nav::ArtistDetail(artist_link.to_owned()))),
        );
    }

    menu = menu.entry(
        MenuItem::new(LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link"))
            .command(cmd::COPY.with(album.data.url())),
    );

    menu = menu.separator();

    if album.ctx.is_album_saved(&album.data) {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-remove-from-library")
                    .with_placeholder("Remove from Library"),
            )
            .command(cmd::UNSAVE_ALBUM.with(album.data.link())),
        );
    } else {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-save-to-library")
                    .with_placeholder("Save to Library"),
            )
            .command(cmd::SAVE_ALBUM.with(album.data.clone())),
        );
    }

    menu
}
