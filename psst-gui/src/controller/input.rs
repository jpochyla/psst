use druid::{
    commands,
    widget::{prelude::*, Controller, TextBox},
    HotKey, KbKey, SysMods,
};

use crate::cmd;
use druid::widget::ValueTextBox;

pub struct InputController {
    on_submit: Option<Box<dyn Fn(&mut EventCtx, &mut String, &Env)>>,
}

impl InputController {
    pub fn new() -> Self {
        Self { on_submit: None }
    }

    pub fn on_submit(
        mut self,
        on_submit: impl Fn(&mut EventCtx, &mut String, &Env) + 'static,
    ) -> Self {
        self.on_submit = Some(Box::new(on_submit));
        self
    }
}

impl Controller<String, TextBox<String>> for InputController {
    fn event(
        &mut self,
        child: &mut TextBox<String>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        self.match_event(child, ctx, event, data, env)
    }
}

impl Controller<String, ValueTextBox<String>> for InputController {
    fn event(
        &mut self,
        child: &mut ValueTextBox<String>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        self.match_event(child, ctx, event, data, env)
    }
}

impl InputController {
    fn match_event(
        &mut self,
        child: &mut impl Widget<String>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(cmd::SET_FOCUS) => {
                ctx.request_focus();
                ctx.request_paint();
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if HotKey::new(None, KbKey::Enter).matches(k_e) => {
                ctx.resign_focus();
                ctx.request_paint();
                ctx.set_handled();
                if let Some(on_submit) = &self.on_submit {
                    on_submit(ctx, data, env);
                }
            }
            Event::KeyDown(k_e) if k_e.key == KbKey::Escape => {
                ctx.resign_focus();
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if HotKey::new(SysMods::Cmd, "c").matches(k_e) => {
                ctx.submit_command(commands::COPY);
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if HotKey::new(SysMods::Cmd, "x").matches(k_e) => {
                ctx.submit_command(commands::CUT);
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if HotKey::new(SysMods::Cmd, "v").matches(k_e) => {
                ctx.submit_command(commands::PASTE);
                ctx.set_handled();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }
}
