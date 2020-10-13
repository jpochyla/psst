use druid::{
    widget::Controller, Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, MouseButton,
    MouseEvent, Widget,
};

pub struct ExClick<T> {
    /// A closure that will be invoked when the child widget is clicked.
    action: Box<dyn Fn(&mut EventCtx, &MouseEvent, &mut T, &Env)>,
    /// Mouse button this widget reacts to. Defaults to the left button.
    button: MouseButton,
}

impl<T: Data> ExClick<T> {
    /// Create a new clickable [`Controller`] widget.
    pub fn new(action: impl Fn(&mut EventCtx, &MouseEvent, &mut T, &Env) + 'static) -> Self {
        ExClick {
            action: Box::new(action),
            button: MouseButton::Left,
        }
    }

    /// Builder-style method for setting the mouse button.
    pub fn with_button(mut self, button: MouseButton) -> Self {
        self.button = button;
        self
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for ExClick<T> {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button == self.button {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(mouse_event) => {
                if ctx.is_active() && mouse_event.button == self.button {
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
