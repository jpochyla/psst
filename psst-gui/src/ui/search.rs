use std::sync::Arc;

use druid::{
    widget::{
        CrossAxisAlignment, Either, Flex, Label, LabelText, List, MainAxisAlignment, Scroll,
        TextBox,
    },
    Data, Env, Insets, Lens, LensExt, RenderContext, Selector, Widget, WidgetExt,
};

use crate::{
    cmd,
    controller::InputController,
    data::{AppState, Ctx, Nav, Search, SearchResults, SearchTopic, SpotifyUrl, WithCtx},
    ui::show,
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt},
};

use super::{album, artist, playable, playlist, theme, track, utils};

const NUMBER_OF_RESULTS_PER_TOPIC: usize = 5;
const INDIVIDUAL_TOPIC_RESULTS_LIMIT: usize = 50;

pub const LOAD_RESULTS: Selector<(Arc<str>, Option<SearchTopic>)> =
    Selector::new("app.search.load-results");
pub const OPEN_LINK: Selector<SpotifyUrl> = Selector::new("app.search.open-link");
pub const SET_TOPIC: Selector<Option<SearchTopic>> = Selector::new("app.search.set-topic");

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
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacer(theme::grid(1.0))
        .with_child(topic_widget())
        .with_flex_child(Scroll::new(async_results_widget()).vertical(), 1.0)
}

fn topic_widget() -> impl Widget<AppState> {
    let mut topics = Flex::row();

    topics.add_child(topic_button("All", None));

    for &topic in SearchTopic::all() {
        topics.add_default_spacer();
        topics.add_child(topic_button(topic.display_name(), Some(topic)));
    }

    Scroll::new(
        topics
            .main_axis_alignment(MainAxisAlignment::Center)
            .padding(Insets::new(0.0, 0.0, 0.0, theme::grid(2.0))),
    )
    .horizontal()
}

fn topic_button(label: &str, topic: Option<SearchTopic>) -> impl Widget<AppState> {
    Label::new(label)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding(Insets::uniform_xy(theme::grid(1.5), theme::grid(0.5)))
        .background(druid::widget::Painter::new(
            move |ctx, data: &AppState, env: &Env| {
                let is_selected = data.search.topic == topic;
                let color = if is_selected {
                    env.get(theme::GREY_500)
                } else if ctx.is_hot() {
                    env.get(theme::GREY_600)
                } else {
                    env.get(theme::GREY_700)
                };
                let bounds = ctx
                    .size()
                    .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));
                ctx.fill(bounds, &color);
            },
        ))
        .link()
        .on_click(move |ctx, _, _| {
            ctx.submit_command(SET_TOPIC.with(topic));
        })
}

fn async_results_widget() -> impl Widget<AppState> {
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
        |(q, t)| {
            let topics = t
                .map(|t| vec![t])
                .unwrap_or_else(|| SearchTopic::all().to_vec());
            let limit = if topics.len() == 1 {
                INDIVIDUAL_TOPIC_RESULTS_LIMIT
            } else {
                NUMBER_OF_RESULTS_PER_TOPIC
            };
            WebApi::global().search(&q, &topics, limit)
        },
        |_, data, (q, t)| data.search.results.defer((q, t)),
        |_, data, r| data.search.results.update(r),
    )
    .on_command(SET_TOPIC, |ctx, topic, data: &mut AppState| {
        data.search.topic = *topic;
        if !data.search.input.is_empty() {
            ctx.submit_command(
                LOAD_RESULTS.with((data.search.input.clone().into(), data.search.topic)),
            );
        }
    })
    .on_command_async(
        OPEN_LINK,
        |l| WebApi::global().load_spotify_link(&l),
        |_, data, l| data.search.results.defer((l.id(), None)),
        |ctx, data, (l, r)| match r {
            Ok(nav) => {
                data.search.results.clear();
                ctx.submit_command(cmd::NAVIGATE.with(nav));
            }
            Err(err) => {
                data.search.results.reject((l.id(), None), err);
            }
        },
    )
}

fn loaded_results_widget() -> impl Widget<WithCtx<SearchResults>> {
    Either::new(
        |results: &WithCtx<SearchResults>, _| results.data.is_empty(),
        Label::new("No results")
            .with_text_size(theme::TEXT_SIZE_LARGE)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .padding(theme::grid(6.0))
            .center(),
        Either::new(
            |results: &WithCtx<SearchResults>, _| results.data.topic.is_some(),
            results_list(false),
            results_list(true),
        ),
    )
}

fn results_list(include_headers: bool) -> Flex<WithCtx<SearchResults>> {
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Fill);
    column = column.with_child(artist_results_widget(include_headers));
    column = column.with_child(album_results_widget(include_headers));
    column = column.with_child(track_results_widget(include_headers));
    column = column.with_child(playlist_results_widget(include_headers));
    column.with_child(show_results_widget(include_headers))
}

fn section_widget<T: Data, W: Widget<T> + 'static>(
    header: &str,
    include_header: bool,
    lens: impl Lens<WithCtx<SearchResults>, T> + 'static,
    is_empty: impl Fn(&T) -> bool + 'static,
    content: impl Fn() -> W + 'static,
) -> impl Widget<WithCtx<SearchResults>> {
    let header_text = header.to_string();
    Either::new(move |data: &T, _| is_empty(data), Empty, {
        let mut column = Flex::column();
        if include_header {
            column = column.with_child(header_widget(header_text.clone()));
        }
        column.with_child(content())
    })
    .lens(lens)
}

fn artist_results_widget(include_header: bool) -> impl Widget<WithCtx<SearchResults>> {
    section_widget(
        SearchTopic::Artist.display_name(),
        include_header,
        Ctx::data().then(SearchResults::artists),
        |artists| artists.is_empty(),
        || List::new(|| artist::artist_widget(false)),
    )
}

fn album_results_widget(include_header: bool) -> impl Widget<WithCtx<SearchResults>> {
    section_widget(
        SearchTopic::Album.display_name(),
        include_header,
        Ctx::map(SearchResults::albums),
        |albums| albums.data.is_empty(),
        || List::new(|| album::album_widget(false)),
    )
}

fn track_results_widget(include_header: bool) -> impl Widget<WithCtx<SearchResults>> {
    section_widget(
        SearchTopic::Track.display_name(),
        include_header,
        druid::lens::Identity,
        |results| results.data.tracks.is_empty(),
        || {
            playable::list_widget(playable::Display {
                track: track::Display {
                    title: true,
                    artist: true,
                    album: true,
                    cover: true,
                    ..track::Display::empty()
                },
            })
        },
    )
}

fn playlist_results_widget(include_header: bool) -> impl Widget<WithCtx<SearchResults>> {
    section_widget(
        SearchTopic::Playlist.display_name(),
        include_header,
        Ctx::map(SearchResults::playlists),
        |playlists| playlists.data.is_empty(),
        || List::new(|| playlist::playlist_widget(false)),
    )
}

fn show_results_widget(include_header: bool) -> impl Widget<WithCtx<SearchResults>> {
    section_widget(
        SearchTopic::Show.display_name(),
        include_header,
        Ctx::map(SearchResults::shows),
        |shows| shows.data.is_empty(),
        || List::new(|| show::show_widget(false)),
    )
}

fn header_widget<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
    Label::new(text)
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding((0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
}
