use druid::{
    widget::{Flex, Label, List, Scroll, ViewSwitcher},
    LensExt, Selector, Widget, WidgetExt,
};

use super::{
    album::album_widget,
    artist::artist_widget,
    playlist::playlist_widget,
    show::show_widget,
    theme,
    utils::{error_widget, spinner_widget},
};
use crate::cmd;
use crate::data::Album;
use crate::data::Artist;
use crate::data::Playlist;
use crate::data::Show;
use crate::data::{AppState, HomeFeed, HomeFeedItem, HomeFeedSection};
use crate::ui::Nav;
use crate::webapi::WebApi;
use crate::widget::{Async, Empty, MyWidgetExt};
use std::sync::Arc;

// pub const LOAD_MADE_FOR_YOU: Selector = Selector::new("app.home.load-made-for-your");
pub const LOAD_HOME_FEED: Selector = Selector::new("app.home.load-home-feed");

pub fn home_widget() -> impl Widget<AppState> {
    Async::new(
        spinner_widget,
        || {
            Scroll::new(
                List::new(|| {
                    Flex::column()
                        .with_child(
                            Label::new(|data: &HomeFeedSection, _env: &_| data.title.to_string())
                                .with_font(theme::UI_FONT_MEDIUM),
                        )
                        .with_child(
                            Label::new(|data: &HomeFeedSection, _env: &_| {
                                data.subtitle
                                    .as_ref()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default()
                            })
                            .with_text_size(theme::TEXT_SIZE_SMALL),
                        )
                        .with_child(
                            List::new(|| home_feed_item_widget())
                                .horizontal()
                                .lens(HomeFeedSection::items),
                        )
                })
                .lens(HomeFeed::sections),
            )
        },
        error_widget,
    )
    .lens(AppState::home_feed)
    .on_command_async(
        LOAD_HOME_FEED,
        |_| WebApi::global().get_home_feed(),
        |_, data, d| data.home_feed.defer(d),
        |_, data, r| data.home_feed.update(r),
    )
}

fn home_feed_item_widget() -> impl Widget<HomeFeedItem> {
    ViewSwitcher::new(
        |item: &HomeFeedItem, _| item.clone(),
        |item, _, _| match item {
            HomeFeedItem::Playlist(playlist) => playlist_item_widget(playlist.clone()).boxed(),
            HomeFeedItem::Album(album) => album_item_widget(album.clone()).boxed(),
            HomeFeedItem::Artist(artist) => artist_item_widget(artist.clone()).boxed(),
            HomeFeedItem::Show(show) => show_item_widget(show.clone()).boxed(),
            HomeFeedItem::Unknown => Empty.boxed(), // Don't display unknown types
        },
    )
}

fn playlist_item_widget(playlist: Arc<Playlist>) -> impl Widget<HomeFeedItem> {
    Label::new(playlist.name.clone()).on_click(move |ctx, _, _| {
        ctx.submit_command(cmd::NAVIGATE.with(Nav::PlaylistDetail(playlist.link())));
    })
}

fn album_item_widget(album: Arc<Album>) -> impl Widget<HomeFeedItem> {
    Label::new(album.name.clone()).on_click(move |ctx, _, _| {
        ctx.submit_command(cmd::NAVIGATE.with(Nav::AlbumDetail(album.link())));
    })
}

fn artist_item_widget(artist: Arc<Artist>) -> impl Widget<HomeFeedItem> {
    Label::new(artist.name.clone()).on_click(move |ctx, _, _| {
        ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(artist.link())));
    })
}

fn show_item_widget(show: Arc<Show>) -> impl Widget<HomeFeedItem> {
    Label::new(show.name.clone()).on_click(move |ctx, _, _| {
        ctx.submit_command(cmd::NAVIGATE.with(Nav::ShowDetail(show.link())));
    })
}
