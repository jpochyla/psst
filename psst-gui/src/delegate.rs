use std::{fs, sync::Arc};

use directories::UserDirs;
use druid::{
    commands, AppDelegate, Application, Command, DelegateCtx, Env, Event, Handled, Target,
    WindowDesc, WindowId,
};
use threadpool::ThreadPool;
use ureq;

#[cfg(target_os = "windows")]
mod windows_tray;

use crate::{
    cmd,
    data::{AppState, Config},
    ui,
    webapi::WebApi,
    widget::remote_image,
};

use crate::ui::playlist::{
    RENAME_PLAYLIST, RENAME_PLAYLIST_CONFIRM, UNFOLLOW_PLAYLIST, UNFOLLOW_PLAYLIST_CONFIRM,
};
use crate::ui::theme;
use crate::ui::DOWNLOAD_ARTWORK;

#[cfg(target_os = "windows")]
use windows_tray::WindowsTray;

pub struct Delegate {
    main_window: Option<WindowId>,
    preferences_window: Option<WindowId>,
    credits_window: Option<WindowId>,
    artwork_window: Option<WindowId>,
    image_pool: ThreadPool,
    size_updated: bool,
    #[cfg(target_os = "windows")]
    systray: Option<Arc<std::sync::Mutex<WindowsTray>>>,
}

impl Delegate {
    pub fn new() -> Self {
        const MAX_IMAGE_THREADS: usize = 32;

        let delegate = Self {
            main_window: None,
            preferences_window: None,
            credits_window: None,
            artwork_window: None,
            image_pool: ThreadPool::with_name("image_loading".into(), MAX_IMAGE_THREADS),
            size_updated: false,
            #[cfg(target_os = "windows")]
            systray: None,
        };

        delegate
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

    fn show_or_create_window<F>(
        window_id_option: &mut Option<WindowId>,
        create_window_fn: F,
        ctx: &mut DelegateCtx,
    ) where
        F: FnOnce() -> WindowDesc<AppState>,
    {
        if let Some(id) = window_id_option {
            ctx.submit_command(commands::SHOW_WINDOW.to(*id));
        } else {
            let window = create_window_fn();
            *window_id_option = Some(window.id);
            ctx.new_window(window);
        }
    }

    fn show_main(&mut self, config: &Config, ctx: &mut DelegateCtx) {
        let config_clone = config.clone();
        Self::show_or_create_window(
            &mut self.main_window,
            || ui::main_window(&config_clone),
            ctx,
        );
    }

    fn show_account_setup(&mut self, ctx: &mut DelegateCtx) {
        Self::show_or_create_window(&mut self.preferences_window, ui::account_setup_window, ctx);
    }

    fn show_preferences(&mut self, ctx: &mut DelegateCtx) {
        Self::show_or_create_window(&mut self.preferences_window, ui::preferences_window, ctx);
    }

    fn close_all_windows(&mut self, ctx: &mut DelegateCtx) {
        ctx.submit_command(commands::CLOSE_ALL_WINDOWS);
        self.main_window = None;
        self.preferences_window = None;
        self.credits_window = None;
    }

    fn close_preferences(&mut self, ctx: &mut DelegateCtx) {
        if let Some(id) = self.preferences_window.take() {
            ctx.submit_command(commands::CLOSE_WINDOW.to(id));
        }
    }

    fn close_credits(&mut self, ctx: &mut DelegateCtx) {
        if let Some(id) = self.credits_window.take() {
            ctx.submit_command(commands::CLOSE_WINDOW.to(id));
        }
    }

    fn show_credits(&mut self, ctx: &mut DelegateCtx) -> WindowId {
        match self.credits_window {
            Some(id) => {
                ctx.submit_command(commands::SHOW_WINDOW.to(id));
                id
            }
            None => {
                let window = WindowDesc::new(ui::credits::credits_widget())
                    .title("Track Credits")
                    .window_size((theme::grid(50.0), theme::grid(55.0)))
                    .resizable(false);
                let window_id = window.id;
                self.credits_window = Some(window_id);
                ctx.new_window(window);
                window_id
            }
        }
    }

    fn show_artwork(&mut self, ctx: &mut DelegateCtx) {
        Self::show_or_create_window(&mut self.artwork_window, ui::artwork_window, ctx);
    }

    fn quit_app_clean(&mut self, ctx: &mut DelegateCtx, data: &mut AppState) {
        data.config.volume = data.playback.volume;
        data.config.save();

        ctx.submit_command(commands::CLOSE_ALL_WINDOWS);
        ctx.submit_command(commands::QUIT_APP);
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
        if cmd.is(cmd::SHOW_CREDITS_WINDOW) {
            let _window_id = self.show_credits(ctx);
            if let Some(track) = cmd.get(cmd::SHOW_CREDITS_WINDOW) {
                ctx.submit_command(
                    cmd::LOAD_TRACK_CREDITS
                        .with(track.clone())
                        .to(Target::Global),
                );
            }
            Handled::Yes
        } else if cmd.is(cmd::SHOW_MAIN) {
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
        } else if cmd.is(commands::CLOSE_WINDOW) {
            if let Some(window_id) = self.preferences_window {
                if target == Target::Window(window_id) {
                    self.close_preferences(ctx);
                    return Handled::Yes;
                }
            } else if let Some(window_id) = self.credits_window {
                if target == Target::Window(window_id) {
                    self.close_credits(ctx);
                    return Handled::Yes;
                }
            } else if let Some(window_id) = self.main_window {
                if target == Target::Window(window_id) {
                    if data.config.close_to_tray {
                        #[cfg(target_os = "windows")]
                        {
                            if self.systray.is_none() {
                                let mut app = WindowsTray::new().expect("Could not create systray");

                                let tmp_icon_path = std::env::temp_dir().join("psst_tray.ico");
                                std::fs::write(
                                    &tmp_icon_path,
                                    include_bytes!("../assets/logo.ico"),
                                )
                                .expect("write ico");
                                app.set_icon(&tmp_icon_path).expect("tray icon");

                                let win_id = window_id;
                                let handle = ctx.get_external_handle();
                                let handle_prev = ctx.get_external_handle();
                                let handle_pp = ctx.get_external_handle();
                                let handle_next = ctx.get_external_handle();
                                let handle_restore = handle.clone();

                                let handle_prev = handle_prev.clone();
                                let handle_pp = handle_pp.clone();
                                let handle_next = handle_next.clone();

                                app.set_callbacks(
                                    move || {
                                        handle_restore
                                            .submit_command(
                                                druid::commands::SHOW_WINDOW,
                                                (),
                                                druid::Target::Window(win_id),
                                            )
                                            .ok();
                                        handle_restore
                                            .submit_command(
                                                crate::cmd::TRAY_REMOVED,
                                                (),
                                                druid::Target::Global,
                                            )
                                            .ok();
                                    },
                                    move |menu_id| match menu_id {
                                        1001 => {
                                            handle_prev
                                                .submit_command(
                                                    crate::cmd::PLAY_PREVIOUS,
                                                    (),
                                                    druid::Target::Global,
                                                )
                                                .ok();
                                        }
                                        1002 => {
                                            handle_pp
                                                .submit_command(
                                                    crate::cmd::PLAY_PAUSE_TOGGLE,
                                                    (),
                                                    druid::Target::Global,
                                                )
                                                .ok();
                                        }
                                        1003 => {
                                            handle_next
                                                .submit_command(
                                                    crate::cmd::PLAY_NEXT,
                                                    (),
                                                    druid::Target::Global,
                                                )
                                                .ok();
                                        }
                                        1004 => {
                                            handle_next
                                                .submit_command(
                                                    crate::cmd::QUIT_APP_CLEAN,
                                                    (),
                                                    druid::Target::Global,
                                                )
                                                .ok();
                                        }
                                        _ => (),
                                    },
                                )
                                .expect("set callbacks");

                                let tray_arc = Arc::new(std::sync::Mutex::new(app));
                                WindowsTray::set_as_global_instance(tray_arc.clone());
                                self.systray = Some(tray_arc);
                            }
                        }
                        ctx.submit_command(commands::HIDE_WINDOW.to(window_id));
                        return Handled::Yes;
                    } else {
                        self.quit_app_clean(ctx, data);
                        return Handled::Yes;
                    }
                }
            }
            Handled::No
        } else if let Some(text) = cmd.get(cmd::COPY) {
            Application::global().clipboard().put_string(text);
            Handled::Yes
        } else if let Some(text) = cmd.get(cmd::GO_TO_URL) {
            let _ = open::that(text);
            Handled::Yes
        } else if let Handled::Yes = self.command_image(ctx, target, cmd, data) {
            Handled::Yes
        } else if let Some(link) = cmd.get(UNFOLLOW_PLAYLIST_CONFIRM) {
            ctx.submit_command(UNFOLLOW_PLAYLIST.with(link.clone()));
            Handled::Yes
        } else if let Some(link) = cmd.get(RENAME_PLAYLIST_CONFIRM) {
            ctx.submit_command(RENAME_PLAYLIST.with(link.clone()));
            Handled::Yes
        } else if cmd.is(cmd::QUIT_APP_WITH_SAVE) {
            ctx.submit_command(commands::QUIT_APP);
            Handled::Yes
        } else if cmd.is(cmd::QUIT_APP_CLEAN) {
            self.quit_app_clean(ctx, data);
            Handled::Yes
        } else if cmd.is(commands::QUIT_APP) {
            Handled::No
        } else if cmd.is(crate::cmd::SHOW_ARTWORK) {
            self.show_artwork(ctx);
            Handled::Yes
        } else if cmd.is(crate::cmd::TRAY_REMOVED) {
            #[cfg(target_os = "windows")]
            {
                WindowsTray::clear_global_instance();
                self.systray = None;
            }
            Handled::Yes
        } else if let Some((url, title)) = cmd.get(DOWNLOAD_ARTWORK) {
            let safe_title = title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            let file_name = format!("{} cover.jpg", safe_title);

            if let Some(user_dirs) = UserDirs::new() {
                if let Some(download_dir) = user_dirs.download_dir() {
                    let path = download_dir.join(file_name);

                    match ureq::get(url)
                        .call()
                        .and_then(|response| -> Result<(), ureq::Error> {
                            let mut file = fs::File::create(&path)?;
                            let mut reader = response.into_body().into_reader();
                            std::io::copy(&mut reader, &mut file)?;
                            Ok(())
                        }) {
                        Ok(_) => data.info_alert("Cover saved to Downloads folder."),
                        Err(_) => data.error_alert("Failed to download and save artwork"),
                    }
                }
            }
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
        if self.credits_window == Some(id) {
            self.credits_window = None;
            data.credits = None;
        }
        if self.preferences_window == Some(id) {
            self.preferences_window.take();
            data.preferences.reset();
            data.preferences.auth.clear();
        }
        if self.main_window == Some(id) {
            self.quit_app_clean(ctx, data);
        }
        if self.artwork_window == Some(id) {
            self.artwork_window = None;
        }
    }

    fn event(
        &mut self,
        ctx: &mut DelegateCtx,
        window_id: WindowId,
        event: Event,
        data: &mut AppState,
        _env: &Env,
    ) -> Option<Event> {
        if self.main_window == Some(window_id) {
            if let Event::WindowSize(size) = event {
                if !self.size_updated {
                    self.size_updated = true;
                } else {
                    data.config.window_size = size;
                }
            }
        } else if [
            self.preferences_window,
            self.artwork_window,
            self.credits_window,
        ]
        .contains(&Some(window_id))
        {
            if let Event::KeyDown(key_event) = &event {
                if key_event.key == druid::KbKey::Escape {
                    ctx.submit_command(commands::CLOSE_WINDOW.to(window_id));
                    return None;
                }
            }
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
                    let result = WebApi::global().get_image(location.clone());
                    match result {
                        Ok(image_buf) => {
                            let payload = remote_image::ImagePayload {
                                location,
                                image_buf,
                            };
                            sink.submit_command(remote_image::PROVIDE_DATA, payload, target)
                                .unwrap();
                        }
                        Err(err) => {
                            log::warn!("failed to fetch image: {}", err)
                        }
                    }
                });
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
