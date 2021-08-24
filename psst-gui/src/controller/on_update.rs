use druid::{widget::Controller, Data, Env, UpdateCtx, Widget};

pub struct OnUpdate<F> {
    handler: F,
}

impl<F> OnUpdate<F> {
    pub fn new<T>(handler: F) -> Self
    where
        F: Fn(&mut UpdateCtx, &T, &T, &Env),
    {
        Self { handler }
    }
}

impl<T, F, W> Controller<T, W> for OnUpdate<F>
where
    T: Data,
    F: Fn(&mut UpdateCtx, &T, &T, &Env),
    W: Widget<T>,
{
    fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        (self.handler)(ctx, old_data, data, env);
        child.update(ctx, old_data, data, env);
    }
}
