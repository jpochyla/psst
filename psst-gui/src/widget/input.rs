use crate::cmd;
use druid::{
    text::{EditAction, Movement},
    widget::{prelude::*, Controller, TextBox},
    HotKey, KbKey, RawMods,
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
        };
        match event {
            Event::Command(command) if command.is(cmd::SET_FOCUS) => {
                ctx.request_focus();
                ctx.request_paint();
            }
            Event::KeyDown(k_e) if HotKey::new(None, KbKey::Enter).matches(k_e) => {
                ctx.resign_focus();
                ctx.request_paint();
                if let Some(on_submit) = &self.on_submit {
                    on_submit(ctx, data, env);
                }
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "B").matches(k_e) => {
                perform_edit(EditAction::Move(Movement::Left));
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "F").matches(k_e) => {
                perform_edit(EditAction::Move(Movement::Right));
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "A").matches(k_e) => {
                perform_edit(EditAction::Move(Movement::PrecedingLineBreak));
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "E").matches(k_e) => {
                perform_edit(EditAction::Move(Movement::NextLineBreak));
            }
            Event::KeyDown(k_e) if HotKey::new(RawMods::Ctrl, "K").matches(k_e) => {
                perform_edit(EditAction::ModifySelection(Movement::NextLineBreak));
                perform_edit(EditAction::Delete);
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }
}
