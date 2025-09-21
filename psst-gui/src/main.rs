#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::new_without_default, clippy::type_complexity)]

mod cmd;
mod controller;
mod data;
mod delegate;
mod error;
mod ui;
mod webapi;
mod widget;

use druid::AppLauncher;
use env_logger::{Builder, Env};
use webapi::WebApi;

use psst_core::cache::Cache;
use psst_core::oauth::refresh_access_token;

use crate::{
    data::{AppState, Config},
    delegate::Delegate,
};

const ENV_LOG: &str = "PSST_LOG";
const ENV_LOG_STYLE: &str = "PSST_LOG_STYLE";

fn main() {
    // Setup logging from the env variables, with defaults.
    Builder::from_env(
        Env::new()
            .filter_or(ENV_LOG, "info")
            .write_style(ENV_LOG_STYLE),
    )
    .init();

    // Load configuration
    let config = Config::load().unwrap_or_default();

    let paginated_limit = config.paginated_limit;
    let mut state = AppState::default_with_config(config.clone());

    if let Some(cache_dir) = Config::cache_dir() {
        match Cache::new(cache_dir) {
            Ok(cache) => {
                state.preferences.cache = Some(cache);
            }
            Err(err) => {
                log::error!("Failed to create cache: {err}");
            }
        }
    }

    WebApi::new(
        Config::proxy().as_deref(),
        Config::cache_dir(),
        paginated_limit,
    )
    .install_as_global();

    // Apply persisted OAuth bearer if present; otherwise try refresh once if a refresh token exists.
    if let Some(tok) = state.config.oauth_bearer.clone() {
        state.session.set_oauth_bearer(Some(tok.clone()));
        WebApi::global().set_oauth_bearer(Some(tok));
        if let Some(rtok) = state.config.oauth_refresh_token.clone() {
            state.session.set_oauth_refresh_token(Some(rtok.clone()));
            WebApi::global().set_oauth_refresh_token(Some(rtok));
        }
    } else if let Some(rtok) = state.config.oauth_refresh_token.clone() {
        match refresh_access_token(&rtok) {
            Ok((new_access, new_refresh)) => {
                state.session.set_oauth_bearer(Some(new_access.clone()));
                WebApi::global().set_oauth_bearer(Some(new_access.clone()));
                state.config.oauth_bearer = Some(new_access);
                if let Some(r) = new_refresh {
                    state.config.oauth_refresh_token = Some(r);
                }
                state.config.save();
            }
            Err(e) => {
                log::warn!("Failed to refresh OAuth token: {e}");
            }
        }
    }

    let delegate;
    let launcher;
    if state.config.has_credentials() {
        // Credentials are configured, open the main window.
        let window = ui::main_window(&state.config);
        delegate = Delegate::with_main(window.id);
        launcher = AppLauncher::with_window(window).configure_env(ui::theme::setup);

        // Load user's local tracks for the WebApi.
        WebApi::global().load_local_tracks(state.config.username().unwrap());
    } else {
        // No configured credentials, open the account setup.
        let window = ui::account_setup_window();
        delegate = Delegate::with_preferences(window.id);
        launcher = AppLauncher::with_window(window).configure_env(ui::theme::setup);
    };

    launcher
        .delegate(delegate)
        .launch(state)
        .expect("Application launch");
}
