use druid::widget::{prelude::*, Controller};
use druid::{EventCtx, Widget, Event};

use crate::data::{config::SortOrder, AppState};
use crate::cmd;


pub struct SortController;

impl<W> Controller<AppState, W> for SortController
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
            Event::Command(cmd) if cmd.is(cmd::TOGGLE_SORT_ORDER) => {
                if data.config.sort_order == SortOrder::Ascending {
                    data.config.sort_order = SortOrder::Descending;
                } else {
                    data.config.sort_order = SortOrder::Ascending;
                }
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }
}