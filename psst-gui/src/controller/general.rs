use crate::data::AppState;
use druid::{widget::Controller, Env, Event, EventCtx, Widget};

use crate::cmd;
use crate::data::config::Theme;

pub struct GeneralTabController;

impl<W: Widget<AppState>> Controller<AppState, W> for GeneralTabController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(cmd::DETECT_THEME) => {
                let new_theme = Theme::default();
                data.config.theme = new_theme;
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}
