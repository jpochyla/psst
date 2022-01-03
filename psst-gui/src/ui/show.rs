use std::sync::Arc;

use druid::{
    im::Vector,
    kurbo::Circle,
    widget::{CrossAxisAlignment, Flex, Label, LabelText, LineBreaking, List},
    Data, Insets, LensExt, LocalizedString, Menu, MenuItem, Selector, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        AppState, Show, Cached, Ctx, Library, Nav,
        WithCtx, ShowLink
    },
    webapi::WebApi,
    widget::{Async, MyWidgetExt, RemoteImage},
};

use super::{
    album::album_widget,
    library, theme,
    track::{tracklist_widget, TrackDisplay},
    utils::{error_widget, placeholder_widget, spinner_widget},
};

// pub const LOAD_DETAIL: Selector<ArtistLink> = Selector::new("app.artist.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column()
        // .with_child(async_top_tracks_widget())
        // .with_child(async_albums_widget().padding((theme::grid(1.0), 0.0)))
        // .with_child(async_related_widget().padding((theme::grid(1.0), 0.0)))
}

// fn async_top_tracks_widget() -> impl Widget<AppState> {
//     Async::new(spinner_widget, top_tracks_widget, error_widget)
//         .lens(
//             Ctx::make(
//                 AppState::common_ctx,
//                 AppState::artist_detail.then(ArtistDetail::top_tracks),
//             )
//             .then(Ctx::in_promise()),
//         )
//         .on_command_async(
//             LOAD_DETAIL,
//             |d| WebApi::global().get_artist_top_tracks(&d.id),
//             |_, data, d| data.artist_detail.top_tracks.defer(d),
//             |_, data, (d, r)| {
//                 let r = r.map(|tracks| ArtistTracks {
//                     id: d.id.clone(),
//                     name: d.name.clone(),
//                     tracks,
//                 });
//                 data.artist_detail.top_tracks.update((d, r))
//             },
//         )
// }

// fn async_albums_widget() -> impl Widget<AppState> {
//     Async::new(spinner_widget, albums_widget, error_widget)
//         .lens(
//             Ctx::make(
//                 AppState::common_ctx,
//                 AppState::artist_detail.then(ArtistDetail::albums),
//             )
//             .then(Ctx::in_promise()),
//         )
//         .on_command_async(
//             LOAD_DETAIL,
//             |d| WebApi::global().get_artist_albums(&d.id),
//             |_, data, d| data.artist_detail.albums.defer(d),
//             |_, data, r| data.artist_detail.albums.update(r),
//         )
// }

// fn async_related_widget() -> impl Widget<AppState> {
//     Async::new(spinner_widget, related_widget, error_widget)
//         .lens(AppState::artist_detail.then(ArtistDetail::related_artists))
//         .on_command_async(
//             LOAD_DETAIL,
//             |d| WebApi::global().get_related_artists(&d.id),
//             |_, data, d| data.artist_detail.related_artists.defer(d),
//             |_, data, r| data.artist_detail.related_artists.update(r),
//         )
// }

pub fn show_widget() -> impl Widget<WithCtx<Arc<Show>>> {
    let show_image = cover_widget(theme::grid(7.0));

    let show_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::Clip)
        .lens(Show::name.in_arc());

    let show_publisher = Label::raw()
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .lens(Show::publisher.in_arc());

    let show_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(show_name)
        .with_spacer(1.0)
        .with_child(show_publisher);

    let show = Flex::row()
        .with_child(show_image)
        .with_default_spacer()
        .with_flex_child(show_info, 1.0)
        .lens(Ctx::data());
    show
        .padding(theme::grid(0.5))
        .on_click(|ctx, show, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::ShowDetail(show.data.link())));
        })
        .link()
        .on_click(|ctx, show, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::ShowDetail(show.data.link())));
        })
        .context_menu(show_ctx_menu)
}

// pub fn artist_link_widget() -> impl Widget<ArtistLink> {
//     Label::raw()
//         .with_line_break_mode(LineBreaking::WordWrap)
//         .with_font(theme::UI_FONT_MEDIUM)
//         .link()
//         .lens(ArtistLink::name)
//         .on_click(|ctx, link, _| {
//             ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(link.to_owned())));
//         })
//         .context_menu(artist_menu)
// }

pub fn cover_widget(size: f64) -> impl Widget<Arc<Show>> {
    let radius = size / 2.0;
    RemoteImage::new(placeholder_widget(), move |show: &Arc<Show>, _| {
        show.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
}

// fn top_tracks_widget() -> impl Widget<WithCtx<ArtistTracks>> {
//     tracklist_widget(TrackDisplay {
//         title: true,
//         album: true,
//         popularity: true,
//         ..TrackDisplay::empty()
//     })
// }

// fn albums_widget() -> impl Widget<WithCtx<ArtistAlbums>> {
//     Flex::column()
//         .cross_axis_alignment(CrossAxisAlignment::Start)
//         .with_child(header_widget("Albums"))
//         .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::albums)))
//         .with_child(header_widget("Singles"))
//         .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::singles)))
//         .with_child(header_widget("Compilations"))
//         .with_child(List::new(album_widget).lens(Ctx::map(ArtistAlbums::compilations)))
// }

// fn related_widget() -> impl Widget<Cached<Vector<Artist>>> {
//     Flex::column()
//         .cross_axis_alignment(CrossAxisAlignment::Start)
//         .with_child(header_widget("Related Artists"))
//         .with_child(List::new(artist_widget))
//         .lens(Cached::data)
// }

// fn header_widget<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
//     Label::new(text)
//         .with_font(theme::UI_FONT_MEDIUM)
//         .with_text_color(theme::PLACEHOLDER_COLOR)
//         .with_text_size(theme::TEXT_SIZE_SMALL)
//         .padding(Insets::new(0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
// }

fn show_ctx_menu(show: &WithCtx<Arc<Show>>) -> Menu<AppState> {
    show_menu(&show.data, &show.ctx.library)
}

fn show_menu(show: &Arc<Show>, library: &Arc<Library>) -> Menu<AppState> {
    let mut menu = Menu::empty();

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Show"),
        )
        .command(cmd::COPY.with(show.link().url())),
    );

    menu = menu.separator();

    if library.contains_show(show) {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-remove-from-library")
                    .with_placeholder("Unfollow"),
            )
            .command(library::UNSAVE_SHOW.with(show.link())),
        );
    } else {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-save-to-library")
                    .with_placeholder("Follow"),
            )
            .command(library::SAVE_SHOW.with(show.clone())),
        );
    }

    menu
}
