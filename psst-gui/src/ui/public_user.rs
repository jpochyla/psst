use std::sync::Arc;

use crate::{
    data::{
        public_user::{PublicUserDetail, PublicUserInformation},
        AppState, Cached, Ctx, MixedView, PublicUser, WithCtx,
    },
    ui::{
        artist, playlist, theme,
        utils::{self, spinner_widget},
    },
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, RemoteImage},
};
use druid::{
    kurbo::Circle,
    lens::Map,
    widget::{CrossAxisAlignment, Either, Flex, Label, List, Scroll},
    LensExt, Selector, Widget, WidgetExt,
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

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_spacer(theme::grid(1.0))
        .with_child(user_profile_top)
        .with_spacer(theme::grid(1.0))
        .with_child(user_playlists)
        .with_spacer(theme::grid(1.0))
        .with_child(user_artists)
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
