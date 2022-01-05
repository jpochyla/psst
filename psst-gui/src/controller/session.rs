use druid::widget::{prelude::*, Controller};

use crate::{
    cmd,
    data::AppState,
    ui::{home, playlist, user},
};

pub struct SessionController;

impl SessionController {
    fn connect(&self, ctx: &mut EventCtx, data: &mut AppState) {
        // Update the session configuration, any active session will get shut down.
        data.session.update_config(data.config.session());

        // Reload the global, usually visible data.
        ctx.submit_command(playlist::LOAD_LIST);
        ctx.submit_command(home::LOAD_MADE_FOR_YOU);
        ctx.submit_command(user::LOAD_PROFILE);
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
                    self.connect(ctx, data);
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
