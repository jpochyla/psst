use druid::widget::{prelude::*, Controller};

use crate::{
    cmd,
    data::{AppState, Nav, SpotifyUrl},
    ui::{album, artist, library, playlist, recommend, search, show},
};

pub struct NavController;

impl NavController {
    fn load_route_data(&self, ctx: &mut EventCtx, data: &mut AppState) {
        match &data.nav {
            Nav::Home => {}
            Nav::SavedTracks => {
                if !data.library.saved_tracks.is_resolved() {
                    ctx.submit_command(library::LOAD_TRACKS);
                }
            }
            Nav::SavedAlbums => {
                if !data.library.saved_albums.is_resolved() {
                    ctx.submit_command(library::LOAD_ALBUMS);
                }
            }
            Nav::SavedShows => {
                if !data.library.saved_shows.is_resolved() {
                    ctx.submit_command(library::LOAD_SHOWS);
                }
            }
            Nav::SearchResults(query) => {
                if let Some(link) = SpotifyUrl::parse(query) {
                    ctx.submit_command(search::OPEN_LINK.with(link));
                } else if !data.search.results.contains(query) {
                    ctx.submit_command(search::LOAD_RESULTS.with(query.to_owned()));
                }
            }
            Nav::AlbumDetail(link) => {
                if !data.album_detail.album.contains(link) {
                    ctx.submit_command(album::LOAD_DETAIL.with(link.to_owned()));
                }
            }
            Nav::ArtistDetail(link) => {
                if !data.artist_detail.top_tracks.contains(link) {
                    ctx.submit_command(artist::LOAD_DETAIL.with(link.to_owned()));
                }
            }
            Nav::PlaylistDetail(link) => {
                if !data.playlist_detail.playlist.contains(link) {
                    ctx.submit_command(
                        playlist::LOAD_DETAIL.with((link.to_owned(), data.to_owned())),
                    );
                }
            }
            Nav::ShowDetail(link) => {
                if !data.show_detail.show.contains(link) {
                    ctx.submit_command(show::LOAD_DETAIL.with(link.to_owned()));
                }
            }
            Nav::Recommendations(request) => {
                if !data.recommend.results.contains(request) {
                    ctx.submit_command(recommend::LOAD_RESULTS.with(request.clone()));
                }
            }
        }
    }
}

impl<W> Controller<AppState, W> for NavController
where
    W: Widget<AppState>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(cmd::NAVIGATE) => {
                let nav = cmd.get_unchecked(cmd::NAVIGATE);
                data.navigate(nav);
                ctx.set_handled();
                self.load_route_data(ctx, data);
            }
            Event::Command(cmd) if cmd.is(cmd::NAVIGATE_BACK) => {
                let count = cmd.get_unchecked(cmd::NAVIGATE_BACK);
                for _ in 0..*count {
                    data.navigate_back();
                }
                ctx.set_handled();
                self.load_route_data(ctx, data);
            }
            Event::Command(cmd) if cmd.is(cmd::NAVIGATE_REFRESH) => {
                data.refresh();
                ctx.set_handled();
                self.load_route_data(ctx, data);
            }
            Event::MouseDown(cmd) if cmd.button.is_x1() => {
                data.navigate_back();
                ctx.set_handled();
                self.load_route_data(ctx, data);
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &AppState,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if let Some(route) = &data.config.last_route {
                ctx.submit_command(cmd::NAVIGATE.with(route.to_owned()));
            }
        }
        child.lifecycle(ctx, event, data, env)
    }
}
