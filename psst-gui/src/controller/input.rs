use crate::cmd;
use druid::{
    commands,
    text::{EditAction, Movement},
    widget::{prelude::*, Controller, TextBox},
    HotKey, KbKey, RawMods, SysMods,
};

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
        let mut perform_edit = |edit_action| {
            let command = Event::Command(TextBox::PERFORM_EDIT.with(edit_action));
            child.event(ctx, &command, data, env);
            ctx.request_update();
            ctx.request_paint();
        };
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
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "b").matches(k_e) => {
                perform_edit(EditAction::Move(Movement::Left));
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "f").matches(k_e) => {
                perform_edit(EditAction::Move(Movement::Right));
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "a").matches(k_e) => {
                perform_edit(EditAction::Move(Movement::PrecedingLineBreak));
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "e").matches(k_e) => {
                perform_edit(EditAction::Move(Movement::NextLineBreak));
                ctx.set_handled();
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "k").matches(k_e) => {
                perform_edit(EditAction::ModifySelection(Movement::NextLineBreak));
                perform_edit(EditAction::Delete);
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
