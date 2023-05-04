use std::sync::Arc;

use druid::{
    im::Vector,
    widget::{CrossAxisAlignment, Either, Flex, Label, LabelText, List, TextBox},
    Data, LensExt, Selector, Widget, WidgetExt,
};

use crate::{
    cmd,
    controller::InputController,
    data::{
        Album, AppState, Artist, Ctx, Nav, Search, SearchResults, SearchTopic, Show, SpotifyUrl,
        WithCtx,
    },
    ui::show,
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt},
};

use super::{album, artist, playable, playlist, theme, track, utils};

const NUMBER_OF_RESULTS_PER_TOPIC: usize = 5;

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
        utils::spinner_widget,
        loaded_results_widget,
        utils::error_widget,
    )
    .lens(
        Ctx::make(AppState::common_ctx, AppState::search.then(Search::results))
            .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_RESULTS,
        |q| WebApi::global().search(&q, SearchTopic::all(), NUMBER_OF_RESULTS_PER_TOPIC),
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

fn loaded_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    Either::new(
        |results: &WithCtx<SearchResults>, _| {
            results.data.artists.is_empty()
                && results.data.albums.is_empty()
                && results.data.tracks.is_empty()
                && results.data.playlists.is_empty()
                && results.data.shows.is_empty()
        },
        Label::new("No results")
            .with_text_size(theme::TEXT_SIZE_LARGE)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .padding(theme::grid(6.0))
            .center(),
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .with_child(artist_results_widget())
            .with_child(album_results_widget())
            .with_child(track_results_widget())
            .with_child(playlist_results_widget())
            .with_child(show_results_widget()),
    )
}

fn artist_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    Either::new(
        |artists: &Vector<Artist>, _| artists.is_empty(),
        Empty,
        Flex::column()
            .with_child(header_widget("Artists"))
            .with_child(List::new(artist::artist_widget)),
    )
    .lens(Ctx::data().then(SearchResults::artists))
}

fn album_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    Either::new(
        |albums: &WithCtx<Vector<Arc<Album>>>, _| albums.data.is_empty(),
        Empty,
        Flex::column()
            .with_child(header_widget("Albums"))
            .with_child(List::new(album::album_widget)),
    )
    .lens(Ctx::map(SearchResults::albums))
}

fn track_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    Either::new(
        |results: &WithCtx<SearchResults>, _| results.data.tracks.is_empty(),
        Empty,
        Flex::column()
            .with_child(header_widget("Tracks"))
            .with_child(playable::list_widget(playable::Display {
                track: track::Display {
                    title: true,
                    artist: true,
                    album: true,
                    cover: true,
                    ..track::Display::empty()
                },
            })),
    )
}

fn playlist_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    Either::new(
        |playlists: &WithCtx<SearchResults>, _| playlists.data.playlists.is_empty(),
        Empty,
        Flex::column()
            .with_child(header_widget("Playlists"))
            .with_child(
                List::new(playlist::playlist_widget).lens(Ctx::map(SearchResults::playlists)),
            ),
    )
}

fn show_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    Either::new(
        |shows: &WithCtx<Vector<Arc<Show>>>, _| shows.data.is_empty(),
        Empty,
        Flex::column()
            .with_child(header_widget("Podcasts"))
            .with_child(List::new(show::show_widget)),
    )
    .lens(Ctx::map(SearchResults::shows))
}

fn header_widget<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
    Label::new(text)
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding((0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
}
