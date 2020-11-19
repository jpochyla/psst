#![recursion_limit = "256"]

mod cmd;
mod data;
mod delegate;
mod error;
mod ui;
mod web;
mod widget;

use crate::{
    data::{Config, State},
    delegate::DelegateHolder,
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

    let config = Config::load().unwrap_or_default();

    let (launcher, delegate) = if config.has_credentials() {
        let win = ui::make_main_window();
        let win_id = win.id;
        let launcher = AppLauncher::with_window(win).configure_env(ui::theme::setup);
        let mut delegate = DelegateHolder::new(launcher.get_external_handle());
        delegate.configure(&config);
        delegate.main_window.replace(win_id);
        (launcher, delegate)
    } else {
        let win = ui::make_config_window();
        let win_id = win.id;
        let launcher =
            AppLauncher::with_window(ui::make_config_window()).configure_env(ui::theme::setup);
        let mut delegate = DelegateHolder::new(launcher.get_external_handle());
        delegate.config_window.replace(win_id);
        (launcher, delegate)
    };

    let state = State {
        config,
        ..State::default()
    };
    launcher
        .delegate(delegate)
        .launch(state)
        .expect("Application launch");
}
