use druid::kurbo::Shape;
use druid::{widget::prelude::*, Data, WidgetPod};

pub struct Clip<S, W> {
    shape: S,
    inner: W,
}

impl<S, W> Clip<S, W> {
    pub fn new(shape: S, inner: W) -> Self {
        Self { shape, inner }
    }
}

impl<T: Data, S: Shape, W: Widget<T>> Widget<T> for Clip<S, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
        // TODO: Clip the returned size.
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.with_save(|ctx| {
            ctx.clip(&self.shape);
            self.inner.paint(ctx, data, env);
        });
    }
}
