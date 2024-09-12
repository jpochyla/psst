use std::sync::Arc;

use druid::im::Vector;
use druid::widget::{Either, Flex, Label, Scroll};
use druid::{widget::List, LensExt, Selector, Widget, WidgetExt};

use crate::data::{Album, Artist, Ctx, HomeDetail, MixedView, Show, Track, WithCtx};
use crate::widget::Empty;
use crate::{
    data::AppState,
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{artist, playable, theme, track};
use super::{
    playlist,
    utils::{error_widget, spinner_widget},
};

pub const LOAD_MADE_FOR_YOU: Selector = Selector::new("app.home.load-made-for-your");

pub fn home_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Label::new("Made for you").with_text_size(theme::grid(2.5)).align_left().padding((theme::grid(1.5), 0.0)))
        .with_default_spacer()
        .with_child(results_widget())
        .with_default_spacer()
        .with_child(Label::new("Your top artists").with_text_size(theme::grid(2.5)).align_left().padding((theme::grid(1.5), 0.0)))
        .with_default_spacer()
        .with_child(user_top_artists_widget())
        .with_default_spacer()
        .with_child(Label::new("Your top tracks").with_text_size(theme::grid(2.5)).align_left().padding((theme::grid(1.5), 0.0)))
        .with_default_spacer()
        .with_child(user_top_tracks_widget())
}

pub fn results_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        loaded_results_widget,
        error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::home_detail.then(HomeDetail::made_for_x_hub),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_MADE_FOR_YOU,
        |q| WebApi::global().get_made_for_you(),
        |_, data, q| data.home_detail.made_for_x_hub.defer(q),
        |_, data, r| data.home_detail.made_for_x_hub.update(r),
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
        Label::new("No results")
            .with_text_size(theme::TEXT_SIZE_LARGE)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .padding(theme::grid(6.0))
            .center(),
        Flex::column()
            .with_child(artist_results_widget())
            .with_child(album_results_widget())
            .with_child(playlist_results_widget())
            .with_child(show_results_widget()),
    )
}
fn artist_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |artists: &Vector<Artist>, _| artists.is_empty(),
        Empty,
        Flex::column()
            .with_child(List::new(artist::recommended_artist_widget)),
    )
    .lens(Ctx::data().then(MixedView::artists))
}

fn album_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |albums: &Vector<Album>, _| albums.is_empty(),
        Empty,
        Flex::column()
            .with_child(Label::new("not implemented")),
    )
    .lens(Ctx::data().then(MixedView::albums))
}

fn playlist_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |playlists: &WithCtx<MixedView>, _| playlists.data.playlists.is_empty(),
        Empty,
        Flex::column()
            .with_child(
                // List::new(playlist::playlist_widget).lens(Ctx::map(SearchResults::playlists)),
                // May be nicer
                Scroll::new(
                    List::new(
                            || playlist::horizontal_playlist_widget(false, true)
                        ).horizontal()
                    ).horizontal()
                    .lens(Ctx::map(MixedView::playlists)),
            ),
    )
}

fn show_results_widget() -> impl Widget<WithCtx<MixedView>> {
    Either::new(
        |shows: &Vector<Show>, _| shows.is_empty(),
        Empty,
        Flex::column()
            .with_child(Label::new("not implemented")),
    )
    .lens(Ctx::data().then(MixedView::shows))
}

fn user_top_artists_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || Scroll::new(
            List::new(
                    artist::horizontal_recommended_artist_widget
                ).horizontal()
                // TODO Add a function which allows people to scroll with their scroll wheel!!!
            ).horizontal(),
        error_widget,
    )
    .lens(
        AppState::home_detail.then(HomeDetail::user_top_artists)
    )
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
    Async::new(
        spinner_widget,
        top_tracks_widget,
        error_widget,
    )
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