#![recursion_limit = "256"]

mod cmd;
mod controller;
mod data;
mod delegate;
mod error;
mod ui;
mod webapi;
mod widget;

use crate::{
    data::{Config, State},
    delegate::Delegate,
};
use druid::AppLauncher;
use env_logger::{Builder, Env};

const ENV_LOG: &str = "PSST_LOG";
const ENV_LOG_STYLE: &str = "PSST_LOG_STYLE";

fn main() {
    // Setup logging from the env variables, with defaults.
    Builder::from_env(
        Env::new()
            // `ureq` is a bit too noisy, log only warnings by default.
            .filter_or(ENV_LOG, "info,ureq::unit=warn")
            .write_style(ENV_LOG_STYLE),
    )
    .init();

    let state = State {
        config: Config::load().unwrap_or_default(),
        ..State::default()
    };
    let mut delegate = Delegate::new(state.session.clone());

    let launcher = if state.config.has_credentials() {
        // Credentials are configured, open the main window.
        let window = ui::make_main_window();
        delegate.main_window.replace(window.id);
        let launcher = AppLauncher::with_window(window).configure_env(ui::theme::setup);
        launcher
    } else {
        // No configured credentials, open the preferences.
        let window = ui::make_preferences_window();
        delegate.preferences_window.replace(window.id);
        let launcher = AppLauncher::with_window(window).configure_env(ui::theme::setup);
        launcher
    };

    launcher
        .delegate(delegate)
        .launch(state)
        .expect("Application launch");
}
