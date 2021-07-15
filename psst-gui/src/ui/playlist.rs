use crate::{
    cmd,
    data::{AppState, Ctx, Library, Nav, Playlist, PlaylistDetail},
    ui::{
        theme,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, spinner_widget},
    },
    webapi::WebApi,
    widget::{Async, AsyncAction, Clip, LinkExt, RemoteImage},
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking, List},
    Insets, LensExt, LocalizedString, Menu, MenuItem, MouseButton, Size, Widget, WidgetExt,
};

use super::utils::placeholder_widget;

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
                    .on_ex_click(|ctx, event, playlist, _| match event.button {
                        MouseButton::Left => {
                            let nav = Nav::PlaylistDetail(playlist.link());
                            ctx.submit_command(cmd::NAVIGATE.with(nav));
                        }
                        MouseButton::Right => {
                            ctx.show_context_menu(playlist_menu(playlist), event.window_pos);
                        }
                        _ => {}
                    })
            })
        },
        error_widget,
    )
    .controller(AsyncAction::new(|_| WebApi::global().get_playlists()))
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
        .on_ex_click(
            move |ctx, event, playlist: &mut Playlist, _| match event.button {
                MouseButton::Left => {
                    let nav = Nav::PlaylistDetail(playlist.link());
                    ctx.submit_command(cmd::NAVIGATE.with(nav));
                }
                MouseButton::Right => {
                    ctx.show_context_menu(playlist_menu(playlist), event.window_pos);
                }
                _ => {}
            },
        )
}

fn cover_widget(size: f64) -> impl Widget<Playlist> {
    RemoteImage::new(placeholder_widget(), move |playlist: &Playlist, _| {
        playlist.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Playlist> {
    // TODO: Take the radius from theme.
    Clip::new(
        Size::new(size, size).to_rounded_rect(4.0),
        cover_widget(size),
    )
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
        || error_widget().lens(Ctx::data()),
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::playlist_detail.then(PlaylistDetail::tracks),
        )
        .then(Ctx::in_promise()),
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
