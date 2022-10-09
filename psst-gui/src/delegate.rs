use druid::{
    commands, AppDelegate, Application, Command, DelegateCtx, Env, Event, Handled, Target, WindowId,
};
use threadpool::ThreadPool;

use crate::{
    cmd,
    data::{AppState, Config},
    ui,
    webapi::WebApi,
    widget::remote_image,
};

pub struct Delegate {
    main_window: Option<WindowId>,
    preferences_window: Option<WindowId>,
    image_pool: ThreadPool,
}

impl Delegate {
    pub fn new() -> Self {
        const MAX_IMAGE_THREADS: usize = 32;

        Self {
            main_window: None,
            preferences_window: None,
            image_pool: ThreadPool::with_name("image_loading".into(), MAX_IMAGE_THREADS),
        }
    }

    pub fn with_main(main_window: WindowId) -> Self {
        let mut this = Self::new();
        this.main_window.replace(main_window);
        this
    }

    pub fn with_preferences(preferences_window: WindowId) -> Self {
        let mut this = Self::new();
        this.preferences_window.replace(preferences_window);
        this
    }

    fn show_main(&mut self, config: &Config, ctx: &mut DelegateCtx) {
        match self.main_window {
            Some(id) => {
                ctx.submit_command(commands::SHOW_WINDOW.to(id));
            }
            None => {
                let window = ui::main_window(config);
                self.main_window.replace(window.id);
                ctx.new_window(window);
            }
        }
    }

    fn show_account_setup(&mut self, ctx: &mut DelegateCtx) {
        match self.preferences_window {
            Some(id) => {
                ctx.submit_command(commands::SHOW_WINDOW.to(id));
            }
            None => {
                let window = ui::account_setup_window();
                self.preferences_window.replace(window.id);
                ctx.new_window(window);
            }
        }
    }

    fn show_preferences(&mut self, ctx: &mut DelegateCtx) {
        match self.preferences_window {
            Some(id) => {
                ctx.submit_command(commands::SHOW_WINDOW.to(id));
            }
            None => {
                let window = ui::preferences_window();
                self.preferences_window.replace(window.id);
                ctx.new_window(window);
            }
        }
    }

    fn close_all_windows(&mut self, ctx: &mut DelegateCtx) {
        ctx.submit_command(commands::CLOSE_ALL_WINDOWS);
        self.main_window = None;
        self.preferences_window = None;
    }
}

impl AppDelegate<AppState> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        data: &mut AppState,
        _env: &Env,
    ) -> Handled {
        if cmd.is(cmd::SHOW_MAIN) {
            self.show_main(&data.config, ctx);
            Handled::Yes
        } else if cmd.is(cmd::SHOW_ACCOUNT_SETUP) {
            self.show_account_setup(ctx);
            Handled::Yes
        } else if cmd.is(commands::SHOW_PREFERENCES) {
            self.show_preferences(ctx);
            Handled::Yes
        } else if cmd.is(cmd::CLOSE_ALL_WINDOWS) {
            self.close_all_windows(ctx);
            Handled::Yes
        } else if let Some(text) = cmd.get(cmd::COPY) {
            Application::global().clipboard().put_string(&text);
            Handled::Yes
        } else if let Handled::Yes = self.command_image(ctx, target, cmd, data) {
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn window_removed(
        &mut self,
        id: WindowId,
        data: &mut AppState,
        _env: &Env,
        ctx: &mut DelegateCtx,
    ) {
        if self.preferences_window == Some(id) {
            self.preferences_window.take();
            data.preferences.reset();
        }
        if self.main_window == Some(id) {
            ctx.submit_command(commands::CLOSE_ALL_WINDOWS);
            ctx.submit_command(commands::QUIT_APP);
        }
    }

    fn event(
        &mut self,
        _ctx: &mut DelegateCtx,
        _window_id: WindowId,
        event: Event,
        data: &mut AppState,
        _env: &Env,
    ) -> Option<Event> {
        if let Event::WindowSize(size) = event {
            data.config.window_size = size;
            data.config.save();
        }
        Some(event)
    }
}

impl Delegate {
    fn command_image(
        &mut self,
        ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        _data: &mut AppState,
    ) -> Handled {
        if let Some(location) = cmd.get(remote_image::REQUEST_DATA).cloned() {
            let sink = ctx.get_external_handle();
            if let Some(image_buf) = WebApi::global().get_cached_image(&location) {
                let payload = remote_image::ImagePayload {
                    location,
                    image_buf,
                };
                sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                    .unwrap();
            } else {
                self.image_pool.execute(move || {
                    let image_buf = WebApi::global().get_image(location.clone()).unwrap();
                    let payload = remote_image::ImagePayload {
                        location,
                        image_buf,
                    };
                    sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                        .unwrap();
                });
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
