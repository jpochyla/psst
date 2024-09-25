use std::sync::Arc;

use druid::im::Vector;
use druid::widget::{Either, Flex, Label, Scroll};
use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::data::{Artist, Ctx, HomeDetail, MixedView, Show, Track, WithCtx};
use crate::widget::Empty;
use crate::{
    data::AppState,
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{album, artist, playable, show, theme, track};
use super::{
    playlist,
    utils::{error_widget, spinner_widget},
};

pub const LOAD_MADE_FOR_YOU: Selector = Selector::new("app.home.load-made-for-your");

pub fn home_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(made_for_you())
        .with_child(jump_back_in())
        .with_child(user_top_mixes())
        .with_child(recommended_stations())
        .with_child(best_of_artists())
        .with_child(uniquely_yours())
        .with_child(your_shows())
        .with_child(shows_that_you_might_like())
        .with_child(simple_title_label("Your top artists"))
        .with_child(user_top_artists_widget())
        .with_child(simple_title_label("Your top tracks"))
        .with_child(user_top_tracks_widget())
}

fn simple_title_label(title: &str) -> impl Widget<AppState> {
    Flex::column()
    .with_default_spacer()
    .with_child(Label::new(title)
        .with_text_size(theme::grid(2.5))
        .align_left()
        .padding((theme::grid(1.5), 0.0))
    )
}

pub fn made_for_you() -> impl Widget<AppState> {
    Async::new(spinner_widget, loaded_results_widget, || {Empty})
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::made_for_you),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().get_made_for_you(),
            |_, data, q| data.home_detail.made_for_you.defer(q),
            |_, data, r| data.home_detail.made_for_you.update(r),
        )
}

pub fn recommended_stations() -> impl Widget<AppState> {
    Async::new(spinner_widget, loaded_results_widget, || {Empty})
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::recommended_stations),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().recommended_stations(),
            |_, data, q| data.home_detail.recommended_stations.defer(q),
            |_, data, r| data.home_detail.recommended_stations.update(r),
        )
}

fn uniquely_yours_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |results: &WithCtx<MixedView>, _| {
                results.data.playlists.is_empty()
        },
        Empty,
        Flex::column().with_default_spacer()
        .with_child(Label::new("Uniquely yours")
            .with_text_size(theme::grid(2.5))
            .align_left()
            .padding((theme::grid(1.5), 0.0))
        ).with_child(
            Scroll::new(
                Flex::row()
                    .with_child(playlist_results_widget())
            )
            .align_left(),
        ),
    )
}

pub fn uniquely_yours() -> impl Widget<AppState> {
    Async::new(spinner_widget, uniquely_yours_results_widget, || {Empty})
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::uniquely_yours),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().uniquely_yours(),
            |_, data, q| data.home_detail.uniquely_yours.defer(q),
            |_, data, r| data.home_detail.uniquely_yours.update(r),
        )
}

pub fn user_top_mixes() -> impl Widget<AppState> {
    Async::new(spinner_widget, loaded_results_widget, || {Empty})
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::user_top_mixes),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().get_top_mixes(),
            |_, data, q| data.home_detail.user_top_mixes.defer(q),
            |_, data, r| data.home_detail.user_top_mixes.update(r),
        )
}

pub fn best_of_artists() -> impl Widget<AppState> {
    Async::new(spinner_widget, loaded_results_widget, || {Empty})
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::best_of_artists),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().best_of_artists(),
            |_, data, q| data.home_detail.best_of_artists.defer(q),
            |_, data, r| data.home_detail.best_of_artists.update(r),
        )
}

pub fn your_shows() -> impl Widget<AppState> {
    Async::new(spinner_widget, loaded_results_widget, || {Empty})
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::your_shows),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().your_shows(),
            |_, data, q| data.home_detail.your_shows.defer(q),
            |_, data, r| data.home_detail.your_shows.update(r),
        )
}

pub fn jump_back_in() -> impl Widget<AppState> {
    Async::new(spinner_widget, loaded_results_widget, || {Empty})
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::jump_back_in),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().jump_back_in(),
            |_, data, q| data.home_detail.jump_back_in.defer(q),
            |_, data, r| data.home_detail.jump_back_in.update(r),
        )
}

pub fn shows_that_you_might_like() -> impl Widget<AppState> {
    Async::new(spinner_widget, loaded_results_widget, || {Empty})
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::shows_that_you_might_like),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().shows_that_you_might_like(),
            |_, data, q| data.home_detail.shows_that_you_might_like.defer(q),
            |_, data, r| data.home_detail.shows_that_you_might_like.update(r),
        )
}

fn loaded_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |results: &WithCtx<MixedView>, _| {
            results.data.artists.is_empty()
                && results.data.albums.is_empty()
                && results.data.playlists.is_empty()
                && results.data.shows.is_empty()
        },
        Empty,
        Flex::column().with_child(title_label()).with_child(
            Scroll::new(
                Flex::row()
                    .with_child(playlist_results_widget())
                    .with_child(album_results_widget())
                    .with_child(artist_results_widget())
                    .with_child(show_results_widget()),
            )
            .align_left(),
        ),
    )
}

fn title_label() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |title_check: &Arc<str>, _| title_check.is_empty(),
        Empty,
        Flex::column()
            .with_default_spacer()
            .with_child(
                Label::raw()
                    .with_text_size(theme::grid(2.5))
                    .align_left()
                    .padding((theme::grid(1.5), theme::grid(0.5))),
            )
            .with_default_spacer()
            .align_left(),
    )
    .lens(Ctx::data().then(MixedView::title))
}

fn artist_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |artists: &Vector<Artist>, _| artists.is_empty(),
        Empty,
        Scroll::new(List::new(|| artist::artist_widget(true)).horizontal())
            .horizontal()
            .align_left(),
    )
    .lens(Ctx::data().then(MixedView::artists))
}

fn album_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |playlists: &WithCtx<MixedView>, _| playlists.data.albums.is_empty(),
        Empty,
        Flex::column().with_child(
            Scroll::new(List::new(|| album::album_widget(true)).horizontal())
                .horizontal()
                .align_left()
                .lens(Ctx::map(MixedView::albums)),
        ),
    )
}

fn playlist_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |playlists: &WithCtx<MixedView>, _| playlists.data.playlists.is_empty(),
        Empty,
        Flex::column().with_child(
            Scroll::new(List::new(|| playlist::playlist_widget(true)).horizontal())
                .horizontal()
                .align_left()
                .lens(Ctx::map(MixedView::playlists)),
        ),
    )
}

fn show_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |shows: &WithCtx<Vector<Arc<Show>>>, _| shows.data.is_empty(),
        Empty,
        Flex::column().with_child(
            Scroll::new(List::new(|| show::show_widget(true)).horizontal()).align_left(),
        ),
    )
    .lens(Ctx::map(MixedView::shows))
}

fn user_top_artists_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || Scroll::new(List::new(|| artist::artist_widget(true)).horizontal()).horizontal(),
        error_widget,
    )
    .lens(AppState::home_detail.then(HomeDetail::user_top_artists))
    .on_command_async(
        LOAD_MADE_FOR_YOU,
        |_| WebApi::global().get_user_top_artist(),
        |_, data, d| data.home_detail.user_top_artists.defer(d),
        |_, data, r| data.home_detail.user_top_artists.update(r),
    )
}

fn top_tracks_widget() -> impl Widget<WithCtx<Vector<Arc<Track>>>> {
    playable::list_widget(playable::Display {
        track: track::Display {
            title: true,
            album: true,
            popularity: true,
            cover: true,
            ..track::Display::empty()
        },
    })
}

fn user_top_tracks_widget() -> impl Widget<AppState> {
    Async::new(spinner_widget, top_tracks_widget, error_widget)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::home_detail.then(HomeDetail::user_top_tracks),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_MADE_FOR_YOU,
            |_| WebApi::global().get_user_top_tracks(),
            |_, data, d| data.home_detail.user_top_tracks.defer(d),
            |_, data, r| data.home_detail.user_top_tracks.update(r),
        )
}
