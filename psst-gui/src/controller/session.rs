use druid::widget::{prelude::*, Controller};

use crate::{cmd, data::AppState};

pub struct SessionController;

impl SessionController {
    pub fn new() -> Self {
        Self
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
                    data.session.set_config(data.config.session());
                    ctx.submit_command(cmd::SESSION_CONNECTED);
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
            if data.config.has_credentials() {
                data.session.set_config(data.config.session());
                ctx.submit_command(cmd::SESSION_CONNECTED);
            }
        }
        child.lifecycle(ctx, event, data, env)
    }
}
