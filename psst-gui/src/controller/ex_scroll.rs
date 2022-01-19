use druid::{
    widget::Controller, Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, MouseEvent, Widget,
};

pub struct ExScroll<T> {
    action: Box<dyn Fn(&mut EventCtx, &MouseEvent, &mut T, &Env)>,
}

impl<T: Data> ExScroll<T> {
    pub fn new(action: impl Fn(&mut EventCtx, &MouseEvent, &mut T, &Env) + 'static) -> Self {
        ExScroll {
            action: Box::new(action),
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for ExScroll<T> {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Wheel(mouse_event) => {
                ctx.set_active(true);
                (self.action)(ctx, mouse_event, data, env);
                ctx.set_active(false);
                ctx.request_paint()
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
