use std::{sync::Arc, thread};

use druid::{
    commands, image, AppDelegate, Application, Command, DelegateCtx, Env, Handled, ImageBuf,
    Target, WindowId,
};
use lru_cache::LruCache;

use crate::{cmd, data::AppState, ui, webapi::WebApi, widget::remote_image};

pub struct Delegate {
    image_cache: LruCache<Arc<str>, ImageBuf>,
    main_window: Option<WindowId>,
    preferences_window: Option<WindowId>,
}

impl Delegate {
    pub fn new() -> Self {
        const IMAGE_CACHE_SIZE: usize = 256;
        let image_cache = LruCache::new(IMAGE_CACHE_SIZE);

        Self {
            image_cache,
            main_window: None,
            preferences_window: None,
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
            match self.main_window {
                Some(id) => {
                    ctx.submit_command(commands::SHOW_WINDOW.to(id));
                }
                None => {
                    let window = ui::main_window();
                    self.main_window.replace(window.id);
                    ctx.new_window(window);
                }
            }
            Handled::Yes
        } else if cmd.is(commands::SHOW_PREFERENCES) {
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
        _ctx: &mut DelegateCtx,
    ) {
        if self.preferences_window == Some(id) {
            self.preferences_window.take();
            data.preferences.reset();
        }
        if self.main_window == Some(id) {
            self.main_window.take();
        }
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
            if let Some(image_buf) = self.image_cache.get_mut(&location).cloned() {
                let payload = remote_image::ImagePayload {
                    location,
                    image_buf,
                };
                sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                    .unwrap();
            } else {
                self.spawn(move || {
                    let image_format = match &location[location.rfind(".").unwrap_or(0)..] {
                        ".png" => image::ImageFormat::Png,
                        _ => image::ImageFormat::Jpeg
                    };
                    let dyn_image = WebApi::global()
                        .get_image(&location, image_format)
                        .unwrap();
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
}
