use crate::data::AppState;
use druid::{widget::Controller, Env, Event, EventCtx, Widget};
use std::time::Duration;

pub struct AlertCleanupController;

const CLEANUP_INTERVAL: Duration = Duration::from_secs(1);

impl<W: Widget<AppState>> Controller<AppState, W> for AlertCleanupController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::WindowConnected => ctx.request_timer(CLEANUP_INTERVAL),
            Event::Timer(_) => {
                data.cleanup_alerts();
                ctx.request_timer(CLEANUP_INTERVAL);
            }
            _ => {}
        }
        child.event(ctx, event, data, env)
    }
}
