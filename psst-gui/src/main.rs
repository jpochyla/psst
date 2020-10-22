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

fn main() {
    env_logger::init();

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
