use crate::{
    cmd,
    data::{ArtistTracks, Config, Nav, PlaylistTracks, SavedTracks, State},
    ui,
    webapi::WebApi,
    widget::remote_image,
};
use druid::{
    commands, im::Vector, image, AppDelegate, Application, Command, DelegateCtx, Env, ExtEventSink,
    Handled, ImageBuf, Target, WindowId,
};
use lru_cache::LruCache;
use psst_core::session::SessionHandle;
use std::{collections::HashSet, sync::Arc, thread};

pub struct Delegate {
    webapi: Arc<WebApi>,
    image_cache: LruCache<Arc<str>, ImageBuf>,
    pub main_window: Option<WindowId>,
    pub preferences_window: Option<WindowId>,
    opened_windows: HashSet<WindowId>,
}

impl Delegate {
    pub fn new(handle: SessionHandle) -> Self {
        let webapi = Arc::new(WebApi::new(handle, Config::proxy().as_deref()));

        const IMAGE_CACHE_SIZE: usize = 256;
        let image_cache = LruCache::new(IMAGE_CACHE_SIZE);

        Self {
            webapi,
            image_cache,
            main_window: None,
            preferences_window: None,
            opened_windows: HashSet::new(),
        }
    }

    fn navigate_to(data: &mut State, nav: Nav, event_sink: ExtEventSink) {
        if data.route != nav {
            data.history.push_back(nav.clone());
            Self::navigate(data, nav, event_sink);
        }
    }

    fn navigate_back(data: &mut State, event_sink: ExtEventSink) {
        data.history.pop_back();
        Self::navigate(
            data,
            data.history.last().cloned().unwrap_or(Nav::Home),
            event_sink,
        );
    }

    fn navigate(data: &mut State, nav: Nav, event_sink: ExtEventSink) {
        match nav {
            Nav::Home => {
                data.route = Nav::Home;
            }
            Nav::SearchResults(query) => {
                event_sink
                    .submit_command(cmd::GOTO_SEARCH_RESULTS, query, Target::Auto)
                    .unwrap();
            }
            Nav::AlbumDetail(link) => {
                event_sink
                    .submit_command(cmd::GOTO_ALBUM_DETAIL, link, Target::Auto)
                    .unwrap();
            }
            Nav::ArtistDetail(link) => {
                event_sink
                    .submit_command(cmd::GOTO_ARTIST_DETAIL, link, Target::Auto)
                    .unwrap();
            }
            Nav::PlaylistDetail(link) => {
                event_sink
                    .submit_command(cmd::GOTO_PLAYLIST_DETAIL, link, Target::Auto)
                    .unwrap();
            }
            Nav::Library => {
                event_sink
                    .submit_command(cmd::GOTO_LIBRARY, (), Target::Auto)
                    .unwrap();
            }
        }
    }

    fn spawn<F, T>(&self, f: F)
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        // TODO: Use a thread pool.
        thread::spawn(f);
    }
}

impl AppDelegate<State> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        data: &mut State,
        _env: &Env,
    ) -> Handled {
        if cmd.is(cmd::SHOW_MAIN) {
            if self
                .main_window
                .as_ref()
                .map(|id| self.opened_windows.contains(id))
                .unwrap_or(false)
            {
                let win_id = self.main_window.unwrap();
                ctx.submit_command(commands::SHOW_WINDOW.to(win_id));
            } else {
                let win = ui::main_window();
                self.main_window.replace(win.id);
                ctx.new_window(win);
            }
            Handled::Yes
        } else if cmd.is(commands::SHOW_PREFERENCES) {
            if self
                .preferences_window
                .as_ref()
                .map(|id| self.opened_windows.contains(id))
                .unwrap_or(false)
            {
                let win_id = self.preferences_window.unwrap();
                ctx.submit_command(commands::SHOW_WINDOW.to(win_id));
            } else {
                let win = ui::preferences_window();
                self.preferences_window.replace(win.id);
                ctx.new_window(win);
            }
            Handled::Yes
        } else if let Some(text) = cmd.get(cmd::COPY) {
            Application::global().clipboard().put_string(&text);
            Handled::Yes
        } else if let Handled::Yes = self.command_image(ctx, target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_playback(ctx, target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_nav(ctx, target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_playlist(ctx, target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_library(ctx, target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_album(ctx, target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_artist(ctx, target, cmd, data) {
            Handled::Yes
        } else if let Handled::Yes = self.command_search(ctx, target, cmd, data) {
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn window_added(
        &mut self,
        id: WindowId,
        _data: &mut State,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        self.opened_windows.insert(id);
    }

    fn window_removed(
        &mut self,
        id: WindowId,
        _data: &mut State,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        self.opened_windows.remove(&id);
    }
}

impl Delegate {
    fn command_image(
        &mut self,
        ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        _data: &mut State,
    ) -> Handled {
        if let Some(location) = cmd.get(remote_image::REQUEST_DATA).cloned() {
            let sink = ctx.get_external_handle();
            if let Some(image_buf) = self.image_cache.get_mut(&location).cloned() {
                let payload = remote_image::ImagePayload {
                    location,
                    image_buf,
                };
                sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                    .unwrap();
            } else {
                let web = self.webapi.clone();
                self.spawn(move || {
                    let dyn_image = web.get_image(&location, image::ImageFormat::Jpeg).unwrap();
                    let image_buf = ImageBuf::from_dynamic_image(dyn_image);
                    let payload = remote_image::ImagePayload {
                        location,
                        image_buf,
                    };
                    sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                        .unwrap();
                });
            }
            Handled::Yes
        } else if let Some(payload) = cmd.get(remote_image::PROVIDE_DATA).cloned() {
            self.image_cache.insert(payload.location, payload.image_buf);
            Handled::No
        } else {
            Handled::No
        }
    }

    fn command_nav(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
    ) -> Handled {
        if let Some(nav) = cmd.get(cmd::NAVIGATE_TO).cloned() {
            Self::navigate_to(data, nav, ctx.get_external_handle());
            Handled::Yes
        } else if cmd.is(cmd::NAVIGATE_BACK) {
            Self::navigate_back(data, ctx.get_external_handle());
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_playlist(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
    ) -> Handled {
        if cmd.is(cmd::SESSION_CONNECTED) || cmd.is(cmd::LOAD_PLAYLISTS) {
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            data.library_mut().playlists.defer_default();
            self.spawn(move || {
                sink.submit_command(cmd::UPDATE_PLAYLISTS, web.get_playlists(), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_PLAYLISTS).cloned() {
            data.library_mut().playlists.resolve_or_reject(result);
            Handled::Yes
        } else if let Some(link) = cmd.get(cmd::GOTO_PLAYLIST_DETAIL).cloned() {
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            data.route = Nav::PlaylistDetail(link.clone());
            data.playlist.playlist.defer(link.clone());
            data.playlist.tracks.defer(link.clone());
            self.spawn(move || {
                let result = web.get_playlist_tracks(&link.id);
                sink.submit_command(cmd::UPDATE_PLAYLIST_TRACKS, (link, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_PLAYLIST_TRACKS).cloned() {
            if data.playlist.tracks.is_deferred(&link) {
                data.playlist
                    .tracks
                    .resolve_or_reject(result.map(|tracks| PlaylistTracks {
                        id: link.id,
                        name: link.name,
                        tracks,
                    }));
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_library(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
    ) -> Handled {
        if cmd.is(cmd::GOTO_LIBRARY) {
            data.route = Nav::Library;
            if data.library.saved_albums.is_empty() || data.library.saved_albums.is_rejected() {
                data.library_mut().saved_albums.defer_default();
                let web = self.webapi.clone();
                let sink = ctx.get_external_handle();
                self.spawn(move || {
                    let result = web.get_saved_albums();
                    sink.submit_command(cmd::UPDATE_SAVED_ALBUMS, result, Target::Auto)
                        .unwrap();
                });
            }
            if data.library.saved_tracks.is_empty() || data.library.saved_tracks.is_rejected() {
                data.library_mut().saved_tracks.defer_default();
                let web = self.webapi.clone();
                let sink = ctx.get_external_handle();
                self.spawn(move || {
                    let result = web.get_saved_tracks();
                    sink.submit_command(cmd::UPDATE_SAVED_TRACKS, result, Target::Auto)
                        .unwrap();
                });
            }
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_SAVED_ALBUMS).cloned() {
            match result {
                Ok(albums) => {
                    data.common_ctx.set_saved_albums(&albums);
                    data.library_mut().saved_albums.resolve(albums);
                }
                Err(err) => {
                    data.common_ctx.set_saved_albums(&Vector::new());
                    data.library_mut().saved_albums.reject(err);
                }
            };
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_SAVED_TRACKS).cloned() {
            match result {
                Ok(tracks) => {
                    data.common_ctx.set_saved_tracks(&tracks);
                    data.library_mut()
                        .saved_tracks
                        .resolve(SavedTracks { tracks });
                }
                Err(err) => {
                    data.common_ctx.set_saved_tracks(&Vector::new());
                    data.library_mut().saved_tracks.reject(err);
                }
            };
            Handled::Yes
        } else if let Some(track) = cmd.get(cmd::SAVE_TRACK).cloned() {
            let web = self.webapi.clone();
            let track_id = track.id.to_base62();
            data.save_track(track);
            self.spawn(move || {
                let result = web.save_track(&track_id);
                if result.is_err() {
                    // TODO: Refresh saved tracks.
                }
            });
            Handled::Yes
        } else if let Some(track_id) = cmd.get(cmd::UNSAVE_TRACK).cloned() {
            let web = self.webapi.clone();
            data.unsave_track(&track_id);
            self.spawn(move || {
                let result = web.unsave_track(&track_id.to_base62());
                if result.is_err() {
                    // TODO: Refresh saved tracks.
                }
            });
            Handled::Yes
        } else if let Some(album) = cmd.get(cmd::SAVE_ALBUM).cloned() {
            let web = self.webapi.clone();
            let album_id = album.id.clone();
            data.save_album(album);
            self.spawn(move || {
                let result = web.save_album(&album_id);
                if result.is_err() {
                    // TODO: Refresh saved albums.
                }
            });
            Handled::Yes
        } else if let Some(link) = cmd.get(cmd::UNSAVE_ALBUM).cloned() {
            let web = self.webapi.clone();
            data.unsave_album(&link.id);
            self.spawn(move || {
                let result = web.unsave_album(&link.id);
                if result.is_err() {
                    // TODO: Refresh saved albums.
                }
            });
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_album(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
    ) -> Handled {
        if let Some(link) = cmd.get(cmd::GOTO_ALBUM_DETAIL).cloned() {
            data.route = Nav::AlbumDetail(link.clone());
            data.album.album.defer(link.clone());
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            self.spawn(move || {
                let result = web.get_album(&link.id);
                sink.submit_command(cmd::UPDATE_ALBUM_DETAIL, (link, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ALBUM_DETAIL).cloned() {
            if data.album.album.is_deferred(&link) {
                data.album.album.resolve_or_reject(result);
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_artist(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
    ) -> Handled {
        if let Some(album_link) = cmd.get(cmd::GOTO_ARTIST_DETAIL) {
            data.route = Nav::ArtistDetail(album_link.clone());
            // Load artist detail
            data.artist.artist.defer(album_link.clone());
            let link = album_link.clone();
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            self.spawn(move || {
                let result = web.get_artist(&link.id);
                sink.submit_command(cmd::UPDATE_ARTIST_DETAIL, (link, result), Target::Auto)
                    .unwrap();
            });
            // Load artist top tracks
            data.artist.top_tracks.defer(album_link.clone());
            let link = album_link.clone();
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            self.spawn(move || {
                let result = web.get_artist_top_tracks(&link.id);
                sink.submit_command(cmd::UPDATE_ARTIST_TOP_TRACKS, (link, result), Target::Auto)
                    .unwrap();
            });
            // Load artist's related artists
            data.artist.related_artists.defer(album_link.clone());
            let link = album_link.clone();
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            self.spawn(move || {
                let result = web.get_related_artists(&link.id);
                sink.submit_command(cmd::UPDATE_ARTIST_RELATED, (link, result), Target::Auto)
                    .unwrap();
            });
            // Load artist albums
            data.artist.albums.defer(album_link.clone());
            let link = album_link.clone();
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            self.spawn(move || {
                let result = web.get_artist_albums(&link.id);
                sink.submit_command(cmd::UPDATE_ARTIST_ALBUMS, (link, result), Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ARTIST_DETAIL).cloned() {
            if data.artist.artist.is_deferred(&link) {
                data.artist.artist.resolve_or_reject(result);
            }
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ARTIST_ALBUMS).cloned() {
            if data.artist.albums.is_deferred(&link) {
                data.artist.albums.resolve_or_reject(result);
            }
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ARTIST_TOP_TRACKS).cloned() {
            if data.artist.top_tracks.is_deferred(&link) {
                data.artist
                    .top_tracks
                    .resolve_or_reject(result.map(|tracks| ArtistTracks {
                        id: link.id,
                        name: link.name,
                        tracks,
                    }));
            }
            Handled::Yes
        } else if let Some((link, result)) = cmd.get(cmd::UPDATE_ARTIST_RELATED).cloned() {
            if data.artist.related_artists.is_deferred(&link) {
                data.artist.related_artists.resolve_or_reject(result);
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_search(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
    ) -> Handled {
        if let Some(query) = cmd.get(cmd::GOTO_SEARCH_RESULTS).cloned() {
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            data.route = Nav::SearchResults(query.clone());
            data.search.results.defer(query.clone());
            self.spawn(move || {
                let result = web.search(&query);
                sink.submit_command(cmd::UPDATE_SEARCH_RESULTS, result, Target::Auto)
                    .unwrap();
            });
            Handled::Yes
        } else if let Some(result) = cmd.get(cmd::UPDATE_SEARCH_RESULTS).cloned() {
            data.search.results.resolve_or_reject(result);
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn command_playback(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
    ) -> Handled {
        if cmd.is(cmd::PLAYBACK_PLAYING) {
            let (item, progress) = cmd.get_unchecked(cmd::PLAYBACK_PLAYING);

            data.playback.current.as_mut().map(|current| {
                current.analysis.defer(item.clone());
            });
            let item = item.clone();
            let web = self.webapi.clone();
            let sink = ctx.get_external_handle();
            self.spawn(move || {
                let result = web.get_audio_analysis(&item.to_base62());
                sink.submit_command(cmd::UPDATE_AUDIO_ANALYSIS, (item, result), Target::Auto)
                    .unwrap();
            });

            Handled::No
        } else {
            Handled::No
        }
    }
}
