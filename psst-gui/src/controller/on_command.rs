use druid::{widget::Controller, Data, Env, Event, EventCtx, Selector, Widget};

pub struct OnCommand<U, F> {
    selector: Selector<U>,
    handler: F,
}

impl<U, F> OnCommand<U, F> {
    pub fn new<T>(selector: Selector<U>, handler: F) -> Self
    where
        F: Fn(&mut EventCtx, &U, &mut T),
    {
        Self { selector, handler }
    }
}

impl<T, U, F, W> Controller<T, W> for OnCommand<U, F>
where
    T: Data,
    U: 'static,
    F: Fn(&mut EventCtx, &U, &mut T),
    W: Widget<T>,
{
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(self.selector) => {
                (self.handler)(ctx, cmd.get_unchecked(self.selector), data);
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}
