use super::{theme, utils};
use crate::{
    cmd,
    data::{
        news::{NewsFeed, Release},
        AppState, Nav,
    },
    webapi::WebApi,
    widget::{Async, MyWidgetExt, RemoteImage},
};
use druid::{
    widget::{Button, CrossAxisAlignment, Flex, Label, LineBreaking, List, Scroll},
    LensExt, Selector, Size, Widget, WidgetExt,
};

pub const LOAD: Selector = Selector::new("news.load");
pub const MARK_READ: Selector = Selector::new("news.mark-read");

pub fn widget() -> impl Widget<AppState> {
    let feed = Async::new(utils::spinner_widget, || {
        Flex::column().cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(Label::dynamic(|feed: &NewsFeed, _| format!("{} lanzamientos · {} artistas seguidos{}", feed.releases.len(), feed.followed_count,
                if feed.failed_count > 0 { format!(" · No se pudieron consultar {} artistas", feed.failed_count) } else { String::new() }))
                .with_line_break_mode(LineBreaking::WordWrap).expand_width())
            .with_spacer(12.0)
            .with_child(Label::dynamic(|feed: &NewsFeed, _| feed.notice.clone()).with_text_color(theme::PLACEHOLDER_COLOR).with_line_break_mode(LineBreaking::WordWrap).expand_width())
            .with_spacer(16.0)
            .with_child(druid::widget::Either::new(|feed: &NewsFeed, _| feed.releases.is_empty(),
                Label::new("No hay lanzamientos recientes disponibles. Si alguna consulta fall\u{00f3}, vuelve a intentarlo cuando Spotify lo permita.")
                    .with_line_break_mode(LineBreaking::WordWrap).expand_width().boxed(),
                List::new(release_row).lens(NewsFeed::releases).boxed()))
    }, utils::error_widget).lens(AppState::news.then(crate::data::news::NewsState::feed));
    Scroll::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Novedades de tus artistas").with_font(theme::UI_FONT_MEDIUM).with_text_size(24.0))
        .with_spacer(12.0)
        .with_child(Label::new("Álbumes y sencillos publicados en los últimos 90 días por los artistas que sigues en Spotify.")
            .with_line_break_mode(LineBreaking::WordWrap).expand_width())
        .with_spacer(8.0)
        .with_child(Label::new("Esta vista muestra lanzamientos; los anuncios y mensajes de Spotify se consultan en la app oficial.")
            .with_text_color(theme::PLACEHOLDER_COLOR).with_text_size(theme::TEXT_SIZE_SMALL)
            .with_line_break_mode(LineBreaking::WordWrap).expand_width())
        .with_spacer(16.0)
        .with_child(Flex::row()
            .with_child(Button::new("Actualizar").on_click(|ctx, _: &mut AppState, _| ctx.submit_command(LOAD)).disabled_if(|state: &AppState, _| state.news.feed.is_deferred(&())))
            .with_spacer(12.0).with_child(Button::new("Marcar como leídas").on_click(|ctx, _: &mut AppState, _| ctx.submit_command(MARK_READ))))
        .with_spacer(24.0).with_child(feed).padding(24.0).expand_width()).vertical()
}

fn release_row() -> impl Widget<Release> {
    let image = RemoteImage::new(utils::placeholder_widget(), |release: &Release, _| {
        release
            .album
            .image(56.0, 56.0)
            .map(|image| image.url.clone())
    })
    .fix_size(56.0, 56.0)
    .clip(Size::new(56.0, 56.0).to_rounded_rect(6.0));
    Flex::row()
        .with_child(image)
        .with_spacer(16.0)
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(
                    Label::dynamic(|release: &Release, _| {
                        format!(
                            "{}{}",
                            if release.unread { "● " } else { "" },
                            release.album.name
                        )
                    })
                    .with_font(theme::UI_FONT_MEDIUM)
                    .with_line_break_mode(LineBreaking::Clip)
                    .expand_width(),
                )
                .with_spacer(6.0)
                .with_child(
                    Label::dynamic(|release: &Release, _| {
                        format!("{} · {}", release.artist, release.date)
                    })
                    .with_text_color(theme::PLACEHOLDER_COLOR)
                    .with_line_break_mode(LineBreaking::Clip)
                    .expand_width(),
                ),
            1.0,
        )
        .padding(12.0)
        .expand_width()
        .link()
        .rounded(8.0)
        .tooltip("Abrir este lanzamiento")
        .on_left_click(|ctx, _, release, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::AlbumDetail(release.album.link(), None)))
        })
}

pub fn controller(widget: impl Widget<AppState> + 'static) -> impl Widget<AppState> {
    widget
        .on_command_async(
            LOAD,
            |_| WebApi::global().followed_artist_news(),
            |_, state, _| state.news.feed.defer(()),
            |_, state, (_, result)| {
                let result = result.map(|mut feed| {
                    for release in feed.releases.iter_mut() {
                        release.unread = !state
                            .config
                            .seen_releases
                            .iter()
                            .any(|id| id == release.album.id.as_ref());
                    }
                    feed
                });
                state.news.feed.update(((), result));
            },
        )
        .on_command(MARK_READ, |_, _, state| {
            if let Some(feed) = state.news.feed.resolved_mut() {
                for release in feed.releases.iter_mut() {
                    release.unread = false;
                    if !state
                        .config
                        .seen_releases
                        .iter()
                        .any(|id| id == release.album.id.as_ref())
                    {
                        state
                            .config
                            .seen_releases
                            .push_back(release.album.id.to_string());
                    }
                }
                while state.config.seen_releases.len() > 2000 {
                    state.config.seen_releases.pop_front();
                }
                state.config.save();
            }
        })
}
