use druid::widget::{prelude::*, Controller};

use crate::{cmd, data::AppState};

pub struct SessionController;

impl SessionController {
    fn connect(&self, data: &mut AppState) {
        // Update the session configuration, any active session will get shut down.
        data.session.update_config(data.config.session());

        // Reload the global, usually visible data.
        data.with_library_mut(|library| {
            library.playlists.defer_default();
        });
        data.personalized.made_for_you.defer_default();
        data.user_profile.defer_default();
    }
}

impl<W> Controller<AppState, W> for SessionController
where
    W: Widget<AppState>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(cmd::SESSION_CONNECT) => {
                if data.config.has_credentials() {
                    self.connect(data);
                }
                ctx.set_handled();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &AppState,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            ctx.submit_command(cmd::SESSION_CONNECT);
        }
        child.lifecycle(ctx, event, data, env)
    }
}
