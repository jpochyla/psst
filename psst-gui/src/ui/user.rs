use druid::{
    commands,
    kurbo::Circle,
    widget::{CrossAxisAlignment, Either, Flex, Label, LabelText, LineBreaking, List, Scroll},
    Data, Insets, LensExt, LocalizedString, Menu, MenuItem, Selector, Size, UnitPoint, Widget,
    WidgetExt,
};

use crate::{
    cmd,
    data::{AppState, Ctx, Library, Nav, PublicUser, UserDetail, UserLink, UserProfile, WithCtx},
    ui::utils::{stat_row, InfoLayout},
    webapi::WebApi,
    widget::{
        icons::{self, SvgIcon},
        Async, Empty, MyWidgetExt, RemoteImage,
    },
};

use super::{
    album, theme,
    utils::{self},
};

use crate::data::{UserAlbums, UserInfo};

pub const LOAD_PROFILE: Selector = Selector::new("app.user.load-profile");
pub const LOAD_DETAIL: Selector<UserLink> = Selector::new("app.user.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column().with_child(async_user_info().padding((theme::grid(1.0), 0.0)))
    // .with_child(async_albums_widget().padding((theme::grid(1.0), 0.0)))
}

// fn async_albums_widget() -> impl Widget<AppState> {
//     Async::new(utils::spinner_widget, albums_widget,
// utils::error_widget).lens(         Ctx::make(
//             AppState::common_ctx,
//             AppState::user_detail.then(UserDetail::albums),
//         )
//         .then(Ctx::in_promise()),
//     )
//      .on_command_async(
//          LOAD_DETAIL,
//           |d| WebApi::global().get_publicuser_albums(&d.id),
//           |_, data, d| data.user_detail.albums.defer(d),
//           |_, data, r| data.user_detail.albums.update(r),
//   )
// }

fn async_user_info() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, user_widget, || Empty)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::public_user_detail.then(UserDetail::user_info),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_DETAIL,
            |d| WebApi::global().get_publicuser_info(&d.id),
            |_, data, d| data.public_user_detail.user_info.defer(d),
            |_, data, r| data.public_user_detail.user_info.update(r),
        )
}

pub fn public_user_widget(horizontal: bool) -> impl Widget<PublicUser> {
    let (mut user, user_image) = if horizontal {
        (Flex::column(), cover_widget(theme::grid(16.0)))
    } else {
        (Flex::row(), cover_widget(theme::grid(6.0)))
    };

    user = if horizontal {
        user.with_child(user_image)
            .with_default_spacer()
            .with_child(
                Label::raw()
                    .with_font(theme::UI_FONT_MEDIUM)
                    .align_horizontal(UnitPoint::CENTER)
                    .align_vertical(UnitPoint::TOP)
                    .fix_size(theme::grid(16.0), theme::grid(8.0))
                    .lens(PublicUser::display_name),
            )
    } else {
        user.with_child(user_image)
            .with_default_spacer()
            .with_flex_child(
                Label::raw()
                    .with_font(theme::UI_FONT_MEDIUM)
                    .lens(PublicUser::display_name),
                1.0,
            )
    };

    user.padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, user, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::UserDetail(user.link())));
        })
        .context_menu(|user| user_menu(&user.link()))
}

pub fn link_widget() -> impl Widget<UserLink> {
    Label::raw()
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_font(theme::UI_FONT_MEDIUM)
        .link()
        .lens(UserLink::name)
        .on_left_click(|ctx, _, link, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::UserDetail(link.to_owned())));
        })
        .context_menu(user_menu)
}

pub fn cover_widget(size: f64) -> impl Widget<PublicUser> {
    let radius = size / 2.0;
    RemoteImage::new(utils::placeholder_widget(), move |user: &PublicUser, _| {
        user.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
    .clip(Circle::new((radius, radius), radius))
}

fn user_info_widget() -> impl Widget<WithCtx<UserInfo>> {
    let size = theme::grid(16.0);

    let artist_image =
        RemoteImage::new(utils::placeholder_widget(), move |artist: &UserInfo, _| {
            Some(artist.main_image.clone())
        })
        .fix_size(size, size)
        .clip(Size::new(size, size).to_rounded_rect(4.0))
        .lens(Ctx::data());

    let biography = Scroll::new(
        Label::new("")
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_size(theme::TEXT_SIZE_NORMAL)
            .lens(Ctx::data()),
    )
    .vertical();

    let artist_stats = Flex::column()
        .with_child(stat_row("Followers:", |info: &UserInfo| {
            utils::format_number_with_commas(info.stats.followers)
        }))
        .with_default_spacer()
        .with_child(stat_row("Following:", |info: &UserInfo| {
            utils::format_number_with_commas(info.stats.following)
        }));

    Flex::row()
        .with_child(artist_image)
        .with_spacer(theme::grid(1.0))
        .with_flex_child(
            Flex::row().with_flex_child(InfoLayout::new(biography, artist_stats), 1.0),
            1.0,
        )
        .padding((0.0, theme::grid(1.0))) // Keep overall vertical padding
}

fn albums_widget() -> impl Widget<WithCtx<UserAlbums>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(header_widget("Playlists"))
        .with_child(List::new(|| album::album_widget(false)).lens(Ctx::map(UserAlbums::albums)))
}

fn header_widget<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
    Label::new(text)
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding(Insets::new(0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
}

fn user_menu(user: &UserLink) -> Menu<AppState> {
    let mut menu = Menu::empty();

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to User"),
        )
        .command(cmd::COPY.with(user.url())),
    );

    menu
}

pub fn user_widget() -> impl Widget<AppState> {
    let is_connected = Either::new(
        // TODO: Avoid the locking here.
        |state: &AppState, _| state.session.is_connected(),
        Label::new("Connected")
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL),
        Label::new("Disconnected")
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL),
    );

    let user_profile = Async::new(
        || Empty,
        || {
            Label::raw()
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(UserProfile::display_name)
        },
        || Empty,
    )
    .lens(AppState::library.then(Library::user_profile.in_arc()))
    .on_command_async(
        LOAD_PROFILE,
        |_| WebApi::global().get_user_profile(),
        |_, data, d| data.with_library_mut(|l| l.user_profile.defer(d)),
        |_, data, r| data.with_library_mut(|l| l.user_profile.update(r)),
    );

    Flex::row()
        .with_child(
            Flex::column()
                .with_child(is_connected)
                .with_default_spacer()
                .with_child(user_profile)
                .padding(theme::grid(1.0)),
        )
        .with_child(preferences_widget(&icons::PREFERENCES).padding(theme::grid(1.0)))
}

fn preferences_widget<T: Data>(svg: &SvgIcon) -> impl Widget<T> {
    svg.scale((theme::grid(3.0), theme::grid(3.0)))
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, _, _| ctx.submit_command(commands::SHOW_PREFERENCES))
}
