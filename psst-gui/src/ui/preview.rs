//! Local UI review without an account. Compiled only in development builds.
use crate::{
    data::{AppState, Config, Theme, Track},
    splitify::Assignment,
};
use druid::{AppDelegate, AppLauncher, Command, DelegateCtx, Env, Handled, WindowDesc};
use std::sync::Arc;

struct PreviewDelegate;
impl AppDelegate<AppState> for PreviewDelegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _: druid::Target,
        command: &Command,
        data: &mut AppState,
        _: &Env,
    ) -> Handled {
        if let Some(track) = command.get(super::playlist_picker::OPEN) {
            data.playlist_picker_track = Some(*track);
            ctx.new_window(super::playlist_picker::window());
            return Handled::Yes;
        }
        // Prevent remote requests and writes while reviewing fixture screens.
        let local_ui_command = [
            "app.playable.reveal-playing",
            "app.playlist.set-sort",
            "app.show-finder",
            "app.set-focus",
            "find",
            "report-match",
            "focus-match",
            "find-in-playlist",
            "find-in-saved-tracks",
        ]
        .iter()
        .any(|symbol| command.is(druid::Selector::<()>::new(symbol)));
        if command.is(druid::commands::QUIT_APP)
            || command.is(druid::commands::CLOSE_WINDOW)
            || local_ui_command
        {
            Handled::No
        } else {
            Handled::Yes
        }
    }
}

pub fn run_if_requested() -> bool {
    let Some(view) =
        std::env::args().find_map(|arg| arg.strip_prefix("--preview-ui=").map(str::to_owned))
    else {
        return false;
    };
    let mut config = Config::default();
    config.native_connect = false;
    if std::env::args().any(|arg| arg == "--dark") {
        config.theme = Theme::Dark;
    } else if std::env::args().any(|arg| arg == "--light") {
        config.theme = Theme::Light;
    }
    config.webapi_client_id = Some("Tu Client ID de Spotify".into());
    config.window_size = druid::Size::new(1120.0, 800.0);
    let mut state = AppState::default_with_config(config);
    let window = match view.as_str() {
        "playlist-picker" | "playlist-picker-error" => {
            if view == "playlist-picker" {
                fixture_library(&mut state);
            } else {
                let until = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3600;
                state.with_library_mut(|library| {
                    library
                        .playlists
                        .reject((), crate::error::Error::rate_limited(until))
                });
            }
            super::playlist_picker::window()
        }
        "editor" | "editor-empty" => {
            if view == "editor" {
                state.splitify.source =
                    "https://open.spotify.com/playlist/37i9dQZF1DX4WYpdgoIcn6".into();
                state.splitify.categories = "Indie, Electrónica, Chill".into();
                state.splitify.status =
                    "Vista previa lista. Revisa las categorías antes de guardar.".into();
                for (index, (name, artist, category)) in [
                    (
                        "Instant Crush",
                        "Daft Punk · Julian Casablancas",
                        "Electrónica",
                    ),
                    ("The Less I Know the Better", "Tame Impala", "Indie"),
                    ("Intro", "The xx", "Chill"),
                    ("Midnight City", "M83", "Electrónica"),
                    ("Everything In Its Right Place", "Radiohead", "Chill"),
                    ("Electric Feel", "MGMT", "Indie"),
                    (
                        "A very long song title to check truncation and narrow windows",
                        "An artist with a long display name",
                        "Indie",
                    ),
                    ("Get Lucky", "Daft Punk · Pharrell Williams", "Electrónica"),
                ]
                .iter()
                .enumerate()
                {
                    let track: Track = serde_json::from_value(serde_json::json!({
                        "name": name, "artists": [{"id":"preview", "name":artist}],
                        "duration_ms":240000, "disc_number":1, "track_number":index+1,
                        "explicit":false, "is_local":false, "is_playable":true
                    }))
                    .expect("valid UI fixture");
                    state.splitify.rows.push_back(Assignment {
                        track: Arc::new(track),
                        category: (*category).into(),
                        keep: true,
                    });
                }
            }
            crate::splitify::window()
        }
        "queue" | "queue-large" | "queue-empty" | "lyrics" | "lyrics-follow" | "menus" => {
            if view == "menus" {
                fixture_library(&mut state);
            }
            state.nav = crate::data::Nav::Home;
            state.queue_panel_open = !matches!(view.as_str(), "lyrics" | "lyrics-follow");
            state.config.show_now_playing = true;
            if view != "queue-empty" {
                for (index, (name, artist)) in [
                    ("Instant Crush", "Daft Punk"),
                    ("Midnight City", "M83"),
                    ("Intro", "The xx"),
                    ("The Less I Know the Better", "Tame Impala"),
                    ("Electric Feel", "MGMT"),
                ]
                .iter()
                .enumerate()
                {
                    let track: Track = serde_json::from_value(serde_json::json!({
                        "name":name, "artists":[{"id":"preview","name":artist}], "duration_ms":240000,
                        "disc_number":1, "track_number":index+1, "explicit":false, "is_local":false, "is_playable":true
                    })).unwrap();
                    let entry = crate::data::QueueEntry {
                        item: crate::data::Playable::Track(Arc::new(track)),
                        origin: crate::data::PlaybackOrigin::Home,
                    };
                    if index == 0 {
                        state.start_playback(
                            entry.item.clone(),
                            entry.origin.clone(),
                            std::time::Duration::from_secs(42),
                        );
                    } else {
                        state.playback.up_next.push_back(entry);
                    }
                }
            }
            if view == "queue-large" {
                let template = state
                    .playback
                    .up_next
                    .front()
                    .cloned()
                    .expect("queue fixture");
                state.playback.up_next = (0..5000)
                    .map(|index| {
                        let mut entry = template.clone();
                        if let crate::data::Playable::Track(track) = &mut entry.item {
                            let track = Arc::make_mut(track);
                            track.name = format!("Canción de prueba {}", index + 1).into();
                            track.id.0.id = index as u128 + 1;
                        }
                        entry
                    })
                    .collect();
            }
            if matches!(view.as_str(), "lyrics" | "lyrics-follow") {
                state.nav = crate::data::Nav::Lyrics;
                state.lyrics.resolve(
                    state
                        .playback
                        .now_playing
                        .as_ref()
                        .unwrap()
                        .item
                        .id()
                        .to_base62(),
                    crate::data::Lyrics {
                        notice: "Vista previa local · Letra sincronizada de ejemplo".into(),
                        lines: [
                            (0, 30_000, "Primera línea de ejemplo"),
                            (30_000, 50_000, "Esta línea coincide con la posición actual"),
                            (50_000, 240_000, "Pulsa una línea para cambiar la posición"),
                        ]
                        .into_iter()
                        .map(|(start, end, words)| crate::data::TrackLines {
                            start_time_ms: start.to_string(),
                            end_time_ms: end.to_string(),
                            words: words.into(),
                        })
                        .collect(),
                    },
                );
            }
            if view == "lyrics-follow" {
                state.progress_playback(std::time::Duration::from_millis(152_500));
                state.lyrics.resolve(
                    state
                        .playback
                        .now_playing
                        .as_ref()
                        .unwrap()
                        .item
                        .id()
                        .to_base62(),
                    crate::data::Lyrics {
                        notice: "LRCLIB preview: synchronized line following".into(),
                        lines: (0..80)
                            .map(|index| crate::data::TrackLines {
                                start_time_ms: (index * 3000).to_string(),
                                end_time_ms: ((index + 1) * 3000).to_string(),
                                words: format!("Line {index}: synchronized lyric preview"),
                            })
                            .collect(),
                    },
                );
            }
            super::main_window(&state.config)
        }
        "rate-limit" => {
            state.nav = crate::data::Nav::SavedTracks;
            let until = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 10;
            state.with_library_mut(|library| {
                library
                    .saved_tracks
                    .reject((), crate::error::Error::rate_limited(until))
            });
            super::main_window(&state.config)
        }
        "news" => {
            state.nav = crate::data::Nav::Notifications;
            state.common_ctx_mut().nav = state.nav.clone();
            let releases = ["Nuevo sencillo", "Nuevo álbum", "Una canción con un título largo para comprobar el espacio"].into_iter().enumerate().map(|(index, name)| {
                let album: crate::data::Album = serde_json::from_value(serde_json::json!({
                    "id":format!("preview{index}"),"name":name,"album_type":"single","images":[],"artists":[{"id":"preview","name":"Artista seguido"}],
                    "release_date":"2026-10-01","release_date_precision":"day"
                })).expect("news preview");
                crate::data::news::Release { album:Arc::new(album),artist:if index < 2 { "Artista seguido".into() } else { "Otro artista con un nombre largo".into() },artist_id:if index < 2 { "artist-a".into() } else { "artist-b".into() },date:"2026-10-01".into(),unread:true }
            }).collect();
            state.news.feed.resolve(
                (),
                crate::data::news::NewsFeed {
                    releases,
                    followed_count: 199,
                    failed_count: 0,
                    notice: String::new(),
                },
            );
            super::main_window(&state.config)
        }
        "roadmap" | "roadmap-find" => {
            if view == "roadmap-find" {
                state.finder.show = true;
            }
            let playlist: crate::data::Playlist = serde_json::from_value(serde_json::json!({
                "id":"preview-playlist", "name":"Playlist grande", "description":"Vista previa local",
                "owner":{"id":"preview","display_name":"Angel"}, "items":{"total":750},
                "collaborative":false, "public":false, "images":[]
            })).unwrap();
            let tracks: druid::im::Vector<_> = (0..750).map(|index| {
                let mut track: Track = serde_json::from_value(serde_json::json!({
                    "name":format!("Song {index}"), "artists":[{"id":"preview","name":"Artista de ejemplo"}],
                    "duration_ms":240000,"disc_number":1,"track_number":index+1,"explicit":false,"is_local":false
                })).unwrap();
                track.id = crate::data::TrackId(psst_core::item_id::ItemId::new(index as u128 + 1, psst_core::item_id::ItemIdType::Track));
                track.track_pos = index;
                Arc::new(track)
            }).collect();
            state.with_library_mut(|library| {
                library
                    .playlists
                    .resolve((), druid::im::vector![playlist.clone()])
            });
            state.nav = crate::data::Nav::PlaylistDetail(playlist.link());
            state.common_ctx_mut().nav = state.nav.clone();
            state
                .playlist_detail
                .playlist
                .resolve(playlist.link(), playlist.clone());
            state.playlist_detail.tracks.resolve(
                playlist.link(),
                crate::data::PlaylistTracks {
                    query: String::new(),
                    id: playlist.id.clone(),
                    name: playlist.name.clone(),
                    tracks: tracks.clone(),
                },
            );
            state.playback.queue = tracks
                .iter()
                .map(|track| crate::data::QueueEntry {
                    item: crate::data::Playable::Track(track.clone()),
                    origin: crate::data::PlaybackOrigin::Playlist(playlist.link()),
                })
                .collect();
            state.start_playback(
                crate::data::Playable::Track(tracks[612].clone()),
                crate::data::PlaybackOrigin::Playlist(playlist.link()),
                std::time::Duration::from_secs(42),
            );
            state.playback.state = crate::data::PlaybackState::Paused;
            state.playback.up_next = state.playback.queue.iter().skip(613).cloned().collect();
            super::main_window(&state.config)
        }
        "home-scroll" => {
            let playlists = (0..12).map(|index| {
                serde_json::from_value(serde_json::json!({
                    "id":format!("preview-playlist-{index}"),
                    "name":format!("Daily Mix {}", index + 1),
                    "description":"Feid, Jowell & Randy, Rauw Alejandro and more music for your day.",
                    "owner":{"id":"preview","display_name":"Angel"},
                    "public":false,"collaborative":false,"images":[],"items":{"total":50}
                })).unwrap()
            }).collect();
            state.home_detail.made_for_you.resolve(
                (),
                crate::data::MixedView {
                    title: "Made For Angelo".into(),
                    playlists,
                    albums: Default::default(),
                    artists: Default::default(),
                    shows: Default::default(),
                },
            );
            state.home_detail.jump_back_in.resolve(
                (),
                crate::data::MixedView {
                    title: "Jump back in".into(),
                    playlists: Default::default(),
                    albums: Default::default(),
                    artists: Default::default(),
                    shows: Default::default(),
                },
            );
            super::main_window(&state.config)
        }
        "player" => super::main_window(&state.config),
        "login" => super::account_setup_window(),
        _ => WindowDesc::new(druid::widget::Label::new("Unknown UI preview")),
    };
    AppLauncher::with_window(window)
        .configure_env(super::theme::setup)
        .delegate(PreviewDelegate)
        .launch(state)
        .expect("UI preview");
    true
}

fn fixture_library(state: &mut AppState) {
    state.with_library_mut(|library| {
        library.user_profile.resolve((), crate::data::UserProfile { id: "fixture".into(), display_name: "Angel".into(), email: "".into() });
        let playlists = [("Us", "fixture", false), ("La", "fixture", false), ("Compartida", "friend", true), ("Solo lectura", "friend", false)].into_iter().map(|(name, owner, collaborative)| {
            serde_json::from_value(serde_json::json!({
                "id":"37i9dQZF1DX4WYpdgoIcn6", "name":name,"description":"", "images":[],
                "owner":{"id":owner,"display_name":"Angel"},"collaborative":collaborative,"public":false
            })).unwrap()
        }).collect();
        library.playlists.resolve((), playlists);
    });
}
