use druid::widget::{prelude::*, Controller};

use crate::{
    cmd,
    data::{AppState, Nav},
};

pub struct NavController;

impl NavController {
    fn load_route_data(&self, ctx: &mut EventCtx, data: &mut AppState) {
        match &data.route {
            Nav::Home => {}
            Nav::SavedTracks => {
                if data.library.saved_tracks.is_empty() {
                    data.library_mut().saved_tracks.defer_default();
                }
            }
            Nav::SavedAlbums => {
                if data.library.saved_albums.is_empty() {
                    data.library_mut().saved_albums.defer_default();
                }
            }
            Nav::SearchResults(query) => {
                ctx.submit_command(cmd::LOAD_SEARCH_RESULTS.with(query.to_owned()));
            }
            Nav::AlbumDetail(link) => {
                if !data.album_detail.album.is_deferred(link) {
                    data.album_detail.album.defer(link.to_owned());
                }
            }
            Nav::ArtistDetail(link) => {
                if !data.artist_detail.top_tracks.is_deferred(link) {
                    data.artist_detail.top_tracks.defer(link.to_owned());
                    data.artist_detail.albums.defer(link.to_owned());
                    data.artist_detail.related_artists.defer(link.to_owned());
                }
            }
            Nav::PlaylistDetail(link) => {
                if !data.playlist_detail.tracks.is_deferred(link) {
                    data.playlist_detail.tracks.defer(link.to_owned());
                }
            }
            Nav::Recommendations => {}
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
                self.load_route_data(ctx, data);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::NAVIGATE_BACK) => {
                let count = cmd.get_unchecked(cmd::NAVIGATE_BACK);
                for _ in 0..*count {
                    data.navigate_back();
                }
                self.load_route_data(ctx, data);
                ctx.set_handled();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }
}
