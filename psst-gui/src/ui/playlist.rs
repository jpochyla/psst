use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking, List},
    Insets, LensExt, LocalizedString, Menu, MenuItem, Selector, Size, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{AppState, Ctx, Library, Nav, Playlist, PlaylistDetail, PlaylistLink, PlaylistTracks},
    webapi::WebApi,
    widget::{Async, MyWidgetExt, RemoteImage},
};

use super::{
    theme,
    track::{tracklist_widget, TrackDisplay},
    utils::{error_widget, placeholder_widget, spinner_widget},
};

pub const LOAD_DETAIL: Selector<PlaylistLink> = Selector::new("app.playlist.load-detail");

pub fn list_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || {
            List::new(|| {
                Label::raw()
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(theme::TEXT_SIZE_SMALL)
                    .lens(Playlist::name)
                    .expand_width()
                    .padding(Insets::uniform_xy(theme::grid(2.0), theme::grid(0.6)))
                    .link()
                    .on_click(|ctx, playlist, _| {
                        ctx.submit_command(
                            cmd::NAVIGATE.with(Nav::PlaylistDetail(playlist.link())),
                        );
                    })
                    .context_menu(playlist_menu)
            })
        },
        error_widget,
    )
    .on_deferred(|_| WebApi::global().get_playlists())
    .lens(AppState::library.then(Library::playlists.in_arc()))
}

pub fn playlist_widget() -> impl Widget<Playlist> {
    let playlist_image = rounded_cover_widget(theme::grid(6.0));

    let playlist_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::Clip)
        .lens(Playlist::name);

    let playlist_description = Label::raw()
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .lens(Playlist::description);

    let playlist_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(playlist_name)
        .with_spacer(2.0)
        .with_child(playlist_description);

    let playlist = Flex::row()
        .with_child(playlist_image)
        .with_default_spacer()
        .with_flex_child(playlist_info, 1.0)
        .padding(theme::grid(1.0));

    playlist
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, playlist, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::PlaylistDetail(playlist.link())));
        })
        .context_menu(playlist_menu)
}

fn cover_widget(size: f64) -> impl Widget<Playlist> {
    RemoteImage::new(placeholder_widget(), move |playlist: &Playlist, _| {
        playlist.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Playlist> {
    // TODO: Take the radius from theme.
    cover_widget(size).clip(Size::new(size, size).to_rounded_rect(4.0))
}

pub fn detail_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || {
            tracklist_widget(TrackDisplay {
                title: true,
                artist: true,
                album: true,
                ..TrackDisplay::empty()
            })
        },
        error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::playlist_detail.then(PlaylistDetail::tracks),
        )
        .then(Ctx::in_promise()),
    )
    .on_cmd_async(
        LOAD_DETAIL,
        |d| WebApi::global().get_playlist_tracks(&d.id),
        |_, data, d| data.playlist_detail.tracks.defer(d),
        |_, data, (d, r)| {
            let r = r.map(|tracks| PlaylistTracks {
                id: d.id.clone(),
                name: d.name.clone(),
                tracks,
            });
            data.playlist_detail.tracks.update((d, r))
        },
    )
}

fn playlist_menu(playlist: &Playlist) -> Menu<AppState> {
    let mut menu = Menu::empty();

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Playlist"),
        )
        .command(cmd::COPY.with(playlist.url())),
    );

    menu
}
