use crate::{
    cmd,
    data::{AppState, Nav, SpotifyUrl},
    ui::{album, artist, library, lyrics, playlist, recommend, search, show},
};
use druid::widget::{prelude::*, Controller};
use druid::Code;

pub struct NavController;

impl NavController {
    fn load_route_data(&self, ctx: &mut EventCtx, data: &mut AppState) {
        match &data.nav {
            Nav::Devices => ctx.submit_command(crate::ui::connect::LOAD),
            Nav::Notifications => {
                if !data.news.feed.is_resolved() && !data.news.feed.is_deferred(&()) {
                    ctx.submit_command(crate::ui::news::LOAD);
                }
            }
            Nav::Home | Nav::Queue => {}
            Nav::Lyrics => {
                if let Some(np) = &data.playback.now_playing {
                    let key = np.item.id().to_base62();
                    if !data.lyrics.contains(&key) && !data.lyrics.is_deferred(&key) {
                        ctx.submit_command(lyrics::SHOW_LYRICS.with(np.clone()));
                    }
                }
            }
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
            Nav::Shows => {
                if !data.library.saved_shows.is_resolved() {
                    ctx.submit_command(library::LOAD_SHOWS);
                }
            }
            Nav::SearchResults(query) => {
                if let Some(link) = SpotifyUrl::parse(query) {
                    ctx.submit_command(search::OPEN_LINK.with(link));
                } else if !data
                    .search
                    .results
                    .contains(&(query.clone(), data.search.topic))
                {
                    ctx.submit_command(
                        search::LOAD_RESULTS.with((query.to_owned(), data.search.topic)),
                    );
                }
            }
            Nav::AlbumDetail(link, _) => {
                if !data.album_detail.album.contains(link) {
                    ctx.submit_command(album::LOAD_DETAIL.with(link.to_owned()));
                }
            }
            Nav::ArtistDetail(link) => {
                if !data.artist_detail.artist.contains(link) {
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
                if matches!(nav, Nav::Queue) {
                    data.queue_panel_open =
                        !(data.queue_panel_open && data.config.show_now_playing);
                    data.config.show_now_playing = data.queue_panel_open;
                    ctx.set_handled();
                    return;
                }
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
                if let Err(error) = crate::webapi::WebApi::global().invalidate_metadata() {
                    data.error_alert(error);
                    ctx.set_handled();
                    return;
                }
                data.refresh_all();
                data.search.results.clear();
                data.news.feed.clear();
                ctx.set_handled();
                self.load_route_data(ctx, data);
            }
            Event::Command(cmd) if cmd.is(cmd::TOGGLE_LYRICS) => {
                match data.nav {
                    Nav::Lyrics => data.navigate_back(),
                    _ => {
                        data.navigate(&Nav::Lyrics);
                    }
                }
                ctx.set_handled();
                self.load_route_data(ctx, data);
            }
            Event::MouseDown(cmd) if cmd.button.is_x1() => {
                data.navigate_back();
                ctx.set_handled();
                self.load_route_data(ctx, data);
            }
            Event::KeyDown(key) if key.mods.ctrl() && key.code == Code::KeyR => {
                if let Err(error) = crate::webapi::WebApi::global().invalidate_metadata() {
                    data.error_alert(error);
                    ctx.set_handled();
                    return;
                }
                data.refresh_all();
                data.search.results.clear();
                data.news.feed.clear();
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
            // Loads the library's saved tracks without the user needing to click on the tab.
            ctx.submit_command(cmd::NAVIGATE.with(Nav::SavedTracks));
            // Load the last route, or the default.
            ctx.submit_command(cmd::NAVIGATE.with(if data.playback.now_playing.is_some() {
                data.nav.clone()
            } else {
                data.config.last_route.to_owned().unwrap_or_default()
            }));
        }
        child.lifecycle(ctx, event, data, env)
    }
}
