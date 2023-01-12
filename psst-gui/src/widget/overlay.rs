use druid::{widget::prelude::*, Data, Point, Vec2, WidgetPod};

pub enum OverlayPosition {
    Bottom,
}

pub struct Overlay<T, W, O> {
    inner: W,
    overlay: WidgetPod<T, O>,
    position: OverlayPosition,
}

impl<T, W, O> Overlay<T, W, O>
where
    O: Widget<T>,
{
    pub fn bottom(inner: W, overlay: O) -> Self {
        Self {
            inner,
            overlay: WidgetPod::new(overlay),
            position: OverlayPosition::Bottom,
        }
    }
}

impl<T, W, O> Widget<T> for Overlay<T, W, O>
where
    T: Data,
    W: Widget<T>,
    O: Widget<T>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
        self.overlay.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
        self.overlay.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
        self.overlay.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let inner_size = self.inner.layout(ctx, bc, data, env);
        let over_size = self.overlay.layout(ctx, bc, data, env);
        let pos = match self.position {
            OverlayPosition::Bottom => {
                Point::ORIGIN + Vec2::new(0.0, inner_size.height - over_size.height)
            }
        };
        self.overlay.set_origin(ctx, data, env, pos);
        inner_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
        self.overlay.paint(ctx, data, env);
    }
}
