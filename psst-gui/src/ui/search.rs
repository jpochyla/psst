use crate::{
    commands, consts,
    ctx::Ctx,
    data::{Navigation, Search, SearchResults, State, TrackCtx},
    ui::{
        album::make_album,
        artist::make_artist,
        theme,
        track::{make_tracklist, TrackDisplay},
    },
    widgets::{icons, InputController, Promised, Stack},
};
use druid::{
    widget::{Flex, Label, List, TextBox},
    LensExt, Widget, WidgetExt,
};

pub fn make_input() -> impl Widget<State> {
    let textbox = TextBox::new()
        .with_placeholder("Search")
        .controller(InputController::new().on_submit(|ctx, query, _env| {
            let nav = Navigation::SearchResults(query.clone());
            ctx.submit_command(commands::NAVIGATE_TO.with(nav));
        }))
        .with_id(consts::WIDGET_SEARCH_INPUT)
        .expand_width();

    let icon = icons::SEARCH
        .scale((theme::grid(2.0), theme::grid(2.0)))
        .padding(theme::grid(1.0) + 1.0) // TODO: Take the padding from env or constant.
        .env_scope(|env, _data| {
            env.set(icons::ICON_COLOR, theme::GREY_4);
        });

    Stack::new()
        .with_child(textbox)
        .with_child(icon)
        .lens(Search::input)
        .lens(State::search)
}

pub fn make_results() -> impl Widget<State> {
    Promised::new(
        || Label::new("Loading"),
        || {
            Flex::column()
                .with_child(make_artist_results())
                .with_child(make_album_results())
                .with_child(make_track_results())
        },
        || Label::new("Error"),
    )
    .lens(
        Ctx::make(State::track_context(), State::search.then(Search::results))
            .then(Ctx::in_promise()),
    )
}

fn make_artist_results() -> impl Widget<Ctx<TrackCtx, SearchResults>> {
    Flex::column()
        .with_child(List::new(make_artist))
        .lens(Ctx::data().then(SearchResults::artists))
}

fn make_album_results() -> impl Widget<Ctx<TrackCtx, SearchResults>> {
    Flex::column()
        .with_child(List::new(make_album))
        .lens(Ctx::data().then(SearchResults::albums))
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
