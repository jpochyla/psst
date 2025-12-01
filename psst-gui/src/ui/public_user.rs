use std::sync::Arc;

use crate::{
    cmd,
    data::{
        public_user::{PublicUserDetail, PublicUserInformation},
        AppState, Cached, Ctx, MixedView, Nav, PublicUser, WithCtx,
    },
    ui::{
        artist, playlist, theme,
        utils::{self, spinner_widget},
    },
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, RemoteImage},
};
use druid::{
    im::Vector,
    kurbo::Circle,
    lens::Map,
    widget::{CrossAxisAlignment, Either, Flex, Label, List, Scroll},
    LensExt, Selector, UnitPoint, Widget, WidgetExt,
};

pub const LOAD_DETAIL: Selector<(PublicUser, AppState)> =
    Selector::new("app.publicUser.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Async::new(spinner_widget, loaded_detail_widget, || Empty)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::public_user_detail.then(PublicUserDetail::info),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_DETAIL,
            |d| WebApi::global().get_public_user_profile(d.0.id),
            |_, data, q| data.public_user_detail.info.defer(q.0),
            |_, data, r| {
                data.public_user_detail
                    .info
                    .update((r.0 .0, r.1.map(|info| Cached::fresh(Arc::new(info)))))
            },
        )
}

fn loaded_detail_widget() -> impl Widget<WithCtx<Cached<Arc<PublicUserInformation>>>> {
    let user_profile_top = user_info_widget().padding(theme::grid(1.0));

    // This puts it as read only, it is needed to trainsform the context into what
    // is needed
    let user_playlists = user_playlists_widget().lens(Map::new(
        |data: &WithCtx<Cached<Arc<PublicUserInformation>>>| WithCtx {
            ctx: data.ctx.clone(),
            data: data.data.data.public_playlists.clone(),
        },
        |_, _| {},
    ));
    let user_artists = user_artists_widget().lens(Map::new(
        |data: &WithCtx<Cached<Arc<PublicUserInformation>>>| WithCtx {
            ctx: data.ctx.clone(),
            data: data.data.data.recently_played_artists.clone(),
        },
        |_, _| {},
    ));
    let user_followers = user_followers_widget().lens(Map::new(
        |data: &WithCtx<Cached<Arc<PublicUserInformation>>>| WithCtx {
            ctx: data.ctx.clone(),
            data: data.data.data.followers.clone(),
        },
        |_, _| {},
    ));
    let user_following = user_following_widget().lens(Map::new(
        |data: &WithCtx<Cached<Arc<PublicUserInformation>>>| WithCtx {
            ctx: data.ctx.clone(),
            data: data.data.data.following.clone(),
        },
        |_, _| {},
    ));

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_spacer(theme::grid(1.0))
        .with_child(user_profile_top)
        .with_spacer(theme::grid(1.0))
        .with_child(user_playlists)
        .with_spacer(theme::grid(1.0))
        .with_child(user_artists)
        .with_spacer(theme::grid(1.0))
        .with_child(user_followers)
        .with_spacer(theme::grid(1.0))
        .with_child(user_following)
}

pub fn cover_widget(size: f64) -> impl Widget<WithCtx<Cached<Arc<PublicUserInformation>>>> {
    let radius = size / 2.0;
    RemoteImage::new(
        utils::placeholder_widget(),
        move |ctx: &WithCtx<Cached<Arc<PublicUserInformation>>>, _| {
            ctx.data.data.image_url.clone().map(|url| url.into())
        },
    )
    .fix_size(size, size)
    .clip(Circle::new((radius, radius), radius))
}

fn user_info_widget() -> impl Widget<WithCtx<Cached<Arc<PublicUserInformation>>>> {
    let size = theme::grid(10.0);
    let user_cover = cover_widget(size);

    let user_name = Label::dynamic(|info: &Arc<PublicUserInformation>, _| info.name.clone())
        .with_text_size(theme::TEXT_SIZE_LARGE)
        .with_font(theme::UI_FONT_MEDIUM);

    let follower_count = Label::dynamic(|info: &Arc<PublicUserInformation>, _| {
        format!("{} followers", info.followers_count)
    })
    .with_text_size(theme::TEXT_SIZE_SMALL)
    .with_text_color(theme::PLACEHOLDER_COLOR);

    let following_count = Label::dynamic(|info: &Arc<PublicUserInformation>, _| {
        format!("{} following", info.following_count)
    })
    .with_text_size(theme::TEXT_SIZE_SMALL)
    .with_text_color(theme::PLACEHOLDER_COLOR);

    let user_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(user_name)
        .with_default_spacer()
        .with_child(follower_count)
        .with_spacer(theme::grid(0.5))
        .with_child(following_count)
        .padding(theme::grid(1.0))
        .lens(Ctx::data().then(Cached::data));

    Flex::row()
        .with_spacer(theme::grid(4.2))
        .with_child(user_cover)
        .with_default_spacer()
        .with_child(user_info)
}

fn user_playlists_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |playlists: &WithCtx<MixedView>, &_| playlists.data.playlists.is_empty(),
        Empty,
        Flex::column()
            .with_child(
                Label::new("Public Playlists")
                    .with_text_size(theme::grid(2.5))
                    .align_left()
                    .padding((theme::grid(1.5), theme::grid(0.5))),
            )
            .with_child(
                Scroll::new(List::new(|| playlist::playlist_widget(true)).horizontal())
                    .horizontal()
                    .align_left()
                    .lens(Ctx::map(MixedView::playlists)),
            ),
    )
}

fn user_artists_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |artists: &WithCtx<MixedView>, &_| artists.data.artists.is_empty(),
        Empty,
        Flex::column()
            .with_child(
                Label::new("Recently Played Artists")
                    .with_text_size(theme::grid(2.5))
                    .align_left()
                    .padding((theme::grid(1.5), theme::grid(0.5))),
            )
            .with_child(
                Scroll::new(List::new(|| artist::artist_widget(true)).horizontal())
                    .horizontal()
                    .align_left()
                    .lens(Ctx::data().then(MixedView::artists)),
            ),
    )
}

fn user_followers_widget() -> impl Widget<WithCtx<Vector<PublicUser>>> {
    Either::new(
        |followers: &WithCtx<Vector<PublicUser>>, &_| followers.data.is_empty(),
        Empty,
        Flex::column()
            .with_child(
                Label::new("Followers")
                    .with_text_size(theme::grid(2.5))
                    .align_left()
                    .padding((theme::grid(1.5), theme::grid(0.5))),
            )
            .with_child(
                Scroll::new(List::new(|| public_user_widget(true)).horizontal())
                    .horizontal()
                    .align_left()
                    .lens(Ctx::data()),
            ),
    )
}

fn user_following_widget() -> impl Widget<WithCtx<Vector<PublicUser>>> {
    Either::new(
        |following: &WithCtx<Vector<PublicUser>>, &_| following.data.is_empty(),
        Empty,
        Flex::column()
            .with_child(
                Label::new("Following")
                    .with_text_size(theme::grid(2.5))
                    .align_left()
                    .padding((theme::grid(1.5), theme::grid(0.5))),
            )
            .with_child(
                Scroll::new(List::new(|| public_user_widget(true)).horizontal())
                    .horizontal()
                    .align_left()
                    .lens(Ctx::data()),
            ),
    )
}

pub fn public_user_widget(horizontal: bool) -> impl Widget<PublicUser> {
    let size = if horizontal {
        theme::grid(16.0)
    } else {
        theme::grid(6.0)
    };

    let user_image = user_cover_widget(size);

    let user = if horizontal {
        Flex::column()
            .with_child(user_image)
            .with_default_spacer()
            .with_child(
                Label::dynamic(|user: &PublicUser, _| user.get_display_name().to_string())
                    .with_font(theme::UI_FONT_MEDIUM)
                    .align_horizontal(UnitPoint::CENTER)
                    .align_vertical(UnitPoint::TOP)
                    .fix_size(theme::grid(16.0), theme::grid(8.0)),
            )
    } else {
        Flex::row()
            .with_child(user_image)
            .with_default_spacer()
            .with_flex_child(
                Label::dynamic(|user: &PublicUser, _| user.get_display_name().to_string())
                    .with_font(theme::UI_FONT_MEDIUM),
                1.0,
            )
    };

    user.padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, user, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::PublicUserDetail(user.clone())));
        })
}

fn user_cover_widget(size: f64) -> impl Widget<PublicUser> {
    let radius = size / 2.0;
    RemoteImage::new(utils::placeholder_widget(), move |user: &PublicUser, _| {
        user.image_url.clone().map(|url| url.into())
    })
    .fix_size(size, size)
    .clip(Circle::new((radius, radius), radius))
}
