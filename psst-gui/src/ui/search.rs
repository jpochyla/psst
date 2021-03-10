use crate::{
    cmd,
    data::{CommonCtx, Ctx, Nav, Search, SearchResults, State},
    ui::{
        album::make_album,
        artist::make_artist,
        theme,
        track::{make_tracklist, TrackDisplay},
        utils::{make_error, make_loader},
    },
    widget::{Async, InputController},
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, List, TextBox},
    LensExt, Widget, WidgetExt,
};

use super::playlist::make_playlist;

pub fn make_input() -> impl Widget<State> {
    TextBox::new()
        .with_placeholder("Search")
        .controller(InputController::new().on_submit(|ctx, query, _env| {
            let nav = Nav::SearchResults(query.clone());
            ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
        }))
        .with_id(cmd::WIDGET_SEARCH_INPUT)
        .expand_width()
        .lens(State::search.then(Search::input))
}

pub fn make_results() -> impl Widget<State> {
    Async::new(
        || make_loader(),
        || {
            let label = |text| {
                Label::new(text)
                    .with_font(theme::UI_FONT_MEDIUM)
                    .with_text_color(theme::PLACEHOLDER_COLOR)
                    .with_text_size(theme::TEXT_SIZE_SMALL)
                    .padding((0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
            };
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Fill)
                .with_child(label("Artists"))
                .with_child(make_artist_results())
                .with_child(label("Albums"))
                .with_child(make_album_results())
                .with_child(label("Tracks"))
                .with_child(make_track_results())
                .with_child(label("Playlists"))
                .with_child(make_playlist_results())
        },
        || make_error().lens(Ctx::data()),
    )
    .lens(Ctx::make(State::common_ctx, State::search.then(Search::results)).then(Ctx::in_promise()))
}

fn make_artist_results() -> impl Widget<Ctx<CommonCtx, SearchResults>> {
    List::new(make_artist).lens(Ctx::data().then(SearchResults::artists))
}

fn make_album_results() -> impl Widget<Ctx<CommonCtx, SearchResults>> {
    List::new(make_album).lens(Ctx::map(SearchResults::albums))
}

fn make_track_results() -> impl Widget<Ctx<CommonCtx, SearchResults>> {
    make_tracklist(TrackDisplay {
        title: true,
        artist: true,
        album: true,
        ..TrackDisplay::empty()
    })
}

fn make_playlist_results() -> impl Widget<Ctx<CommonCtx, SearchResults>> {
    List::new(make_playlist).lens(Ctx::map(SearchResults::playlists))
}
