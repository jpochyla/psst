#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::new_without_default, clippy::type_complexity)]

mod cmd;
mod controller;
mod data;
mod delegate;
mod error;
mod token_utils;
mod ui;
mod webapi;
mod widget;

use druid::AppLauncher;
use env_logger::{Builder, Env};
use token_utils::TokenUtils;
use webapi::WebApi;

use psst_core::{cache::Cache, oauth::refresh_access_token};

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

    if let Some(refresh_token) = state.config.oauth_refresh_token.clone() {
        match refresh_access_token(&refresh_token) {
            Ok((access_token, maybe_refresh_token)) => {
                TokenUtils::apply_refresh_result(
                    &state.session,
                    &mut state.config,
                    access_token,
                    maybe_refresh_token,
                    true,
                );
            }
            Err(e) => {
                log::warn!(
                    "Failed to refresh OAuth token at startup: {e}. Falling back to persisted access token if any."
                );
                // Install tokens from config into runtime holders as-is
                TokenUtils::install_from_config(&state.session, &state.config);
            }
        }
    } else {
        // No refresh token; install any persisted tokens as-is
        TokenUtils::install_from_config(&state.session, &state.config);
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
