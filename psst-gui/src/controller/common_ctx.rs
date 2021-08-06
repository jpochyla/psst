use druid::{
    widget::{prelude::*, Controller},
    Selector,
};

use crate::data::AppState;

pub struct CommonCtxController;

impl CommonCtxController {
    const UPDATE_COMMON_CTX: Selector = Selector::new("app.update_common_ctx");
}

impl<W> Controller<AppState, W> for CommonCtxController
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
            Event::Command(cmd) if cmd.is(Self::UPDATE_COMMON_CTX) => {
                data.update_common_ctx();
                ctx.set_handled();
            }
            _ => child.event(ctx, event, data, env),
        }
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &Env,
    ) {
        if !data.library.same(&old_data.library) {
            ctx.submit_command(Self::UPDATE_COMMON_CTX);
        }
        child.update(ctx, old_data, data, env)
    }
}
