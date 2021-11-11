use std::sync::Arc;

use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LabelText, List, TextBox},
    Data, LensExt, Selector, Widget, WidgetExt,
};

use crate::{
    cmd,
    controller::InputController,
    data::{AppState, Ctx, Nav, Search, SearchResults, SpotifyUrl, WithCtx},
    webapi::WebApi,
    widget::{Async, MyWidgetExt},
};

use super::{
    album::album_widget,
    artist::artist_widget,
    playlist::playlist_widget,
    theme,
    track::{tracklist_widget, TrackDisplay},
    utils::{error_widget, spinner_widget},
};

pub const LOAD_RESULTS: Selector<Arc<str>> = Selector::new("app.search.load-results");
pub const OPEN_LINK: Selector<SpotifyUrl> = Selector::new("app.search.open-link");

pub fn input_widget() -> impl Widget<AppState> {
    TextBox::new()
        .with_placeholder("Search")
        .controller(InputController::new().on_submit(|ctx, query, _| {
            if query.trim().is_empty() {
                return;
            }
            ctx.submit_command(cmd::NAVIGATE.with(Nav::SearchResults(query.clone().into())));
        }))
        .with_id(cmd::WIDGET_SEARCH_INPUT)
        .expand_width()
        .lens(AppState::search.then(Search::input))
}

pub fn results_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || {
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Fill)
                .with_child(header_widget("Artists"))
                .with_child(artist_results_widget())
                .with_child(header_widget("Albums"))
                .with_child(album_results_widget())
                .with_child(header_widget("Tracks"))
                .with_child(track_results_widget())
                .with_child(header_widget("Playlists"))
                .with_child(playlist_results_widget())
        },
        error_widget,
    )
    .lens(
        Ctx::make(AppState::common_ctx, AppState::search.then(Search::results))
            .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_RESULTS,
        |q| WebApi::global().search(&q),
        |_, data, q| data.search.results.defer(q),
        |_, data, r| data.search.results.update(r),
    )
    .on_command_async(
        OPEN_LINK,
        |l| WebApi::global().load_spotify_link(&l),
        |_, data, l| data.search.results.defer(l.id()),
        |ctx, data, (l, r)| match r {
            Ok(nav) => {
                data.search.results.clear();
                ctx.submit_command(cmd::NAVIGATE.with(nav));
            }
            Err(err) => {
                data.search.results.reject(l.id(), err);
            }
        },
    )
}

fn artist_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    List::new(artist_widget).lens(Ctx::data().then(SearchResults::artists))
}

fn album_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    List::new(album_widget).lens(Ctx::map(SearchResults::albums))
}

fn track_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    tracklist_widget(TrackDisplay {
        title: true,
        artist: true,
        album: true,
        ..TrackDisplay::empty()
    })
}

fn playlist_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    List::new(playlist_widget).lens(Ctx::data().then(SearchResults::playlists))
}

fn header_widget<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
    Label::new(text)
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding((0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
}
