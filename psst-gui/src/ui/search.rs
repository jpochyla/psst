use crate::{
    cmd,
    controller::InputController,
    data::{CommonCtx, Ctx, Nav, Search, SearchResults, State},
    ui::{
        album::album_widget,
        artist::artist_widget,
        theme,
        track::{tracklist_widget, TrackDisplay},
        utils::{error_widget, spinner_widget},
    },
    widget::Async,
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, List, TextBox},
    LensExt, Widget, WidgetExt,
};

use super::playlist::playlist_widget;

pub fn input_widget() -> impl Widget<State> {
    TextBox::new()
        .with_placeholder("Search")
        .controller(InputController::new().on_submit(|ctx, query, _env| {
            let nav = Nav::SearchResults(query.clone());
            ctx.submit_command(cmd::NAVIGATE.with(nav));
        }))
        .with_id(cmd::WIDGET_SEARCH_INPUT)
        .expand_width()
        .lens(State::search.then(Search::input))
}

pub fn results_widget() -> impl Widget<State> {
    Async::new(
        || spinner_widget(),
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
                .with_child(artist_results_widget())
                .with_child(label("Albums"))
                .with_child(album_results_widget())
                .with_child(label("Tracks"))
                .with_child(track_results_widget())
                .with_child(label("Playlists"))
                .with_child(playlist_results_widget())
        },
        || error_widget().lens(Ctx::data()),
    )
    .lens(Ctx::make(State::common_ctx, State::search.then(Search::results)).then(Ctx::in_promise()))
}

fn artist_results_widget() -> impl Widget<Ctx<CommonCtx, SearchResults>> {
    List::new(artist_widget).lens(Ctx::data().then(SearchResults::artists))
}

fn album_results_widget() -> impl Widget<Ctx<CommonCtx, SearchResults>> {
    List::new(album_widget).lens(Ctx::map(SearchResults::albums))
}

fn track_results_widget() -> impl Widget<Ctx<CommonCtx, SearchResults>> {
    tracklist_widget(TrackDisplay {
        title: true,
        artist: true,
        album: true,
        ..TrackDisplay::empty()
    })
}

fn playlist_results_widget() -> impl Widget<Ctx<CommonCtx, SearchResults>> {
    List::new(playlist_widget).lens(Ctx::map(SearchResults::playlists))
}
