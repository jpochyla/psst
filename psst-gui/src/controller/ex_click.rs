use druid::{
    widget::Controller, Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, MouseButton,
    MouseEvent, Widget,
};

pub struct ExClick<T> {
    button: Option<MouseButton>,
    action: Box<dyn Fn(&mut EventCtx, &MouseEvent, &mut T, &Env)>,
}

impl<T: Data> ExClick<T> {
    pub fn new(
        button: Option<MouseButton>,
        action: impl Fn(&mut EventCtx, &MouseEvent, &mut T, &Env) + 'static,
    ) -> Self {
        ExClick {
            button,
            action: Box::new(action),
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for ExClick<T> {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button == self.button.unwrap_or(mouse_event.button) {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(mouse_event) => {
                if mouse_event.button == self.button.unwrap_or(mouse_event.button)
                    && ctx.is_active()
                {
                    ctx.set_active(false);
                    if ctx.is_hot() {
                        (self.action)(ctx, mouse_event, data, env);
                    }
                    ctx.request_paint();
                }
            }
            _ => {}
        }

        child.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        if let LifeCycle::HotChanged(_) | LifeCycle::FocusChanged(_) = event {
            ctx.request_paint();
        }

        child.lifecycle(ctx, event, data, env);
    }
}
