use druid::widget::{prelude::*, Controller};
use druid::{EventCtx, Widget, Event};

use crate::data::config::SortCriteria;
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
                data.config.save();
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(cmd::SORT_BY_TITLE) => {
                if data.config.sort_criteria != SortCriteria::Title || true{
                    data.config.sort_criteria = SortCriteria::Title;
                    ctx.submit_command(cmd::NAVIGATE_REFRESH); 
                    data.config.save();
                    ctx.set_handled();
                } 
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }
}