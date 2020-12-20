use crate::{
    cmd,
    data::{Ctx, Navigation, Search, SearchResults, State, TrackCtx},
    ui::{
        album::make_album,
        artist::make_artist,
        track::{make_tracklist, TrackDisplay},
        utils::{make_error, make_loader},
    },
    widget::{InputController, Promised},
};
use druid::{
    widget::{Flex, List, TextBox},
    LensExt, Widget, WidgetExt,
};

pub fn make_input() -> impl Widget<State> {
    TextBox::new()
        .with_placeholder("Search")
        .controller(InputController::new().on_submit(|ctx, query, _env| {
            let nav = Navigation::SearchResults(query.clone());
            ctx.submit_command(cmd::NAVIGATE_TO.with(nav));
        }))
        .with_id(cmd::WIDGET_SEARCH_INPUT)
        .expand_width()
        .lens(Search::input)
        .lens(State::search)
}

pub fn make_results() -> impl Widget<State> {
    Promised::new(
        || make_loader(),
        || {
            Flex::column()
                .with_child(make_artist_results())
                .with_child(make_album_results())
                .with_child(make_track_results())
        },
        || make_error().lens(Ctx::data()),
    )
    .lens(Ctx::make(State::track_ctx, State::search.then(Search::results)).then(Ctx::in_promise()))
}

fn make_artist_results() -> impl Widget<Ctx<TrackCtx, SearchResults>> {
    Flex::column()
        .with_child(List::new(make_artist))
        .lens(Ctx::data().then(SearchResults::artists))
}

fn make_album_results() -> impl Widget<Ctx<TrackCtx, SearchResults>> {
    Flex::column()
        .with_child(List::new(make_album))
        .lens(Ctx::map(SearchResults::albums))
}

fn make_track_results() -> impl Widget<Ctx<TrackCtx, SearchResults>> {
    Flex::column()
        .with_child(make_tracklist(TrackDisplay {
            number: false,
            title: true,
            artist: true,
            album: true,
        }))
        .lens(Ctx::map(SearchResults::tracks))
}
