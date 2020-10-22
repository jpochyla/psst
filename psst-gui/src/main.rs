#![recursion_limit = "256"]

mod commands;
mod consts;
mod data;
mod database;
mod delegate;
mod error;
mod promise;
mod ui;
mod widgets;

use crate::{
    data::{Config, State},
    delegate::Delegate,
};
use druid::{AppLauncher, WindowDesc};
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

    let main_window = WindowDesc::new(ui::make_root)
        .title("Psst")
        .window_size((1000.0, 800.0));
    let app_launcher = AppLauncher::with_window(main_window).configure_env(ui::theme::setup);

    let config = Config::load().unwrap_or_default();
    let delegate = Delegate::new(&config, app_launcher.get_external_handle());
    let app_state = State {
        config,
        ..State::default()
    };
    app_launcher
        .delegate(delegate)
        .launch(app_state)
        .expect("Application launch");
}
