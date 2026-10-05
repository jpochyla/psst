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
        _: &mut DelegateCtx,
        _: druid::Target,
        command: &Command,
        _: &mut AppState,
        _: &Env,
    ) -> Handled {
        // Prevent remote requests and writes while reviewing fixture screens.
        if command.is(druid::commands::QUIT_APP) || command.is(druid::commands::CLOSE_WINDOW) {
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
    if std::env::args().any(|arg| arg == "--dark") {
        config.theme = Theme::Dark;
    } else if std::env::args().any(|arg| arg == "--light") {
        config.theme = Theme::Light;
    }
    config.webapi_client_id = Some("Tu Client ID de Spotify".into());
    config.window_size = druid::Size::new(1120.0, 800.0);
    let mut state = AppState::default_with_config(config);
    let window = match view.as_str() {
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
