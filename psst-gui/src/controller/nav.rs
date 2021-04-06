use druid::widget::{prelude::*, Controller};

use crate::{
    cmd,
    data::{Nav, State},
};

pub struct NavController;

impl NavController {
    fn load_route_data(&self, ctx: &mut EventCtx, data: &mut State) {
        match &data.route {
            Nav::Home => {}
            Nav::SavedTracks => {
                ctx.submit_command(cmd::LOAD_SAVED_TRACKS);
            }
            Nav::SavedAlbums => {
                ctx.submit_command(cmd::LOAD_SAVED_ALBUMS);
            }
            Nav::SearchResults(query) => {
                ctx.submit_command(cmd::LOAD_SEARCH_RESULTS.with(query.to_owned()));
            }
            Nav::AlbumDetail(link) => {
                ctx.submit_command(cmd::LOAD_ALBUM_DETAIL.with(link.to_owned()));
            }
            Nav::ArtistDetail(link) => {
                ctx.submit_command(cmd::LOAD_ARTIST_DETAIL.with(link.to_owned()));
            }
            Nav::PlaylistDetail(link) => {
                ctx.submit_command(cmd::LOAD_PLAYLIST_DETAIL.with(link.to_owned()));
            }
        }
    }
}

impl<W> Controller<State, W> for NavController
where
    W: Widget<State>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut State,
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
                data.navigate_back();
                self.load_route_data(ctx, data);
                ctx.set_handled();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }
}
