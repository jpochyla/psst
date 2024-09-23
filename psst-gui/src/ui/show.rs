use std::sync::Arc;

use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking},
    LensExt, LocalizedString, Menu, MenuItem, Selector, Size, UnitPoint, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{AppState, Ctx, Library, Nav, Show, ShowDetail, ShowEpisodes, ShowLink, WithCtx},
    webapi::WebApi,
    widget::{Async, MyWidgetExt, RemoteImage},
};

use super::{library, playable, theme, track, utils};

pub const LOAD_DETAIL: Selector<ShowLink> = Selector::new("app.show.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        // .with_child(async_info_widget())
        // .with_default_spacer()
        .with_child(async_episodes_widget())
}

// fn async_info_widget() -> impl Widget<AppState> {
//     Async::new(utils::spinner_widget, info_widget, utils::error_widget)
//         .lens(
//             Ctx::make(
//                 AppState::common_ctx,
//                 AppState::show_detail.then(ShowDetail::show),
//             )
//             .then(Ctx::in_promise()),
//         )
//         .on_command_async(
//             LOAD_DETAIL,
//             |d| WebApi::global().get_show(&d.id),
//             |_, data, d| data.show_detail.show.defer(d),
//             |_, data, (d, r)| data.show_detail.show.update((d, r)),
//         )
// }

// fn info_widget() -> impl Widget<WithCtx<Arc<Show>>> {
//     Label::raw().lens(Ctx::data().then(Show::description.in_arc()))
// }

fn async_episodes_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        || {
            playable::list_widget(playable::Display {
                track: track::Display::empty(),
            })
        },
        utils::error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::show_detail.then(ShowDetail::episodes),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_DETAIL,
        |d| WebApi::global().get_show_episodes(&d.id),
        |_, data, d| data.show_detail.episodes.defer(d),
        |_, data, (d, r)| {
            let r = r.map(|episodes| ShowEpisodes {
                show: d.clone(),
                episodes,
            });
            data.show_detail.episodes.update((d, r))
        },
    )
}

pub fn show_widget(horizontal: bool) -> impl Widget<WithCtx<Arc<Show>>> {
    let image_size = theme::grid(if horizontal { 16.0 } else { 6.0 });
    let show_image = rounded_cover_widget(image_size);

    let show_name = Label::raw()
        .with_font(theme::UI_FONT_MEDIUM)
        .with_line_break_mode(LineBreaking::Clip)
        .lens(Show::name.in_arc())
        .align_left();

    let show_publisher = Label::raw()
        .with_line_break_mode(LineBreaking::Clip)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .lens(Show::publisher.in_arc())
        .align_left();

    let show = if horizontal {
        Flex::column()
            .with_child(show_image)
            .with_default_spacer()
            .with_child(
                Flex::column()
                    .with_child(show_name)
                    .with_child(show_publisher)
                    .align_horizontal(UnitPoint::CENTER)
                    .align_vertical(UnitPoint::TOP)
                    .fix_size(theme::grid(16.0), theme::grid(8.0)),
            )
            .padding(theme::grid(1.0))
            .lens(Ctx::data())
    } else {
        Flex::row()
            .with_child(show_image)
            .with_default_spacer()
            .with_flex_child(
                Flex::column()
                    .with_child(show_name)
                    .with_child(show_publisher),
                1.0,
            )
            .padding(theme::grid(1.0))
            .lens(Ctx::data())
    };

    show.align_left()
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, show, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::ShowDetail(show.data.link())));
        })
        .context_menu(show_ctx_menu)
}

fn cover_widget(size: f64) -> impl Widget<Arc<Show>> {
    RemoteImage::new(utils::placeholder_widget(), move |show: &Arc<Show>, _| {
        show.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Arc<Show>> {
    // TODO: Take the radius from theme.
    cover_widget(size).clip(Size::new(size, size).to_rounded_rect(4.0))
}

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
                LocalizedString::new("menu-item-remove-from-library").with_placeholder("Unfollow"),
            )
            .command(library::UNSAVE_SHOW.with(show.link())),
        );
    } else {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-save-to-library").with_placeholder("Follow"),
            )
            .command(library::SAVE_SHOW.with(show.clone())),
        );
    }

    menu
}
