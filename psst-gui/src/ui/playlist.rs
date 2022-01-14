use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking, List},
    Insets, LensExt, LocalizedString, Menu, MenuItem, Selector, Size, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        AppState, Ctx, Library, Nav, Playlist, PlaylistAddTrack, PlaylistDetail, PlaylistLink,
        PlaylistTracks,
    },
    error::Error,
    webapi::WebApi,
    widget::{Async, MyWidgetExt, RemoteImage},
};

use super::{playable, theme, track, utils};

pub const LOAD_LIST: Selector = Selector::new("app.playlist.load-list");
pub const LOAD_DETAIL: Selector<PlaylistLink> = Selector::new("app.playlist.load-detail");
pub const ADD_TRACK: Selector<PlaylistAddTrack> = Selector::new("app.playlist.add-track");

pub fn list_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
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
        utils::error_widget,
    )
    .lens(AppState::library.then(Library::playlists.in_arc()))
    .on_command_async(
        LOAD_LIST,
        |_| WebApi::global().get_playlists(),
        |_, data, d| data.with_library_mut(|l| l.playlists.defer(d)),
        |_, data, r| data.with_library_mut(|l| l.playlists.update(r)),
    )
    .on_command_async(
        ADD_TRACK,
        |d| {
            WebApi::global().add_track_to_playlist(
                &d.link.id,
                &d.track_id
                    .0
                    .to_uri()
                    .ok_or_else(|| Error::WebApiError("Item doesn't have URI".to_string()))?,
            )
        },
        |_, data, d| {
            data.with_library_mut(|library| library.increment_playlist_track_count(&d.link))
        },
        |_, data, (_, r)| {
            if let Err(err) = r {
                data.error_alert(err);
            } else {
                data.info_alert("Added to playlist.");
            }
        },
    )
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
    RemoteImage::new(
        utils::placeholder_widget(),
        move |playlist: &Playlist, _| playlist.image(size, size).map(|image| image.url.clone()),
    )
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Playlist> {
    // TODO: Take the radius from theme.
    cover_widget(size).clip(Size::new(size, size).to_rounded_rect(4.0))
}

pub fn detail_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || {
            playable::list_widget_with_find(
                playable::Display {
                    track: track::Display {
                        title: true,
                        artist: true,
                        album: true,
                        cover: true,
                        ..track::Display::empty()
                    },
                },
                cmd::FIND_IN_PLAYLIST,
            )
        },
        utils::error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::playlist_detail.then(PlaylistDetail::tracks),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
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
