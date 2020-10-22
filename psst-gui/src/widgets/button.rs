use crate::widgets::ExClick;
use druid::{
    widget::{prelude::*, BackgroundBrush, ControllerHost},
    Color, Data, Key, KeyOrValue, MouseEvent, WidgetPod,
};

pub const HOVER_HOT_COLOR: Key<Color> = Key::new("app.hover-hot-color");
pub const HOVER_COLD_COLOR: Key<Color> = Key::new("app.hover-cold-color");

pub struct Hover<T> {
    inner: WidgetPod<T, Box<dyn Widget<T>>>,
    corner_radius: KeyOrValue<f64>,
}

impl<T: Data> Hover<T> {
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self {
            inner: WidgetPod::new(inner).boxed(),
            corner_radius: 0.0.into(),
        }
    }

    pub fn rounded(mut self, radius: impl Into<KeyOrValue<f64>>) -> Self {
        self.set_rounded(radius);
        self
    }

    pub fn set_rounded(&mut self, radius: impl Into<KeyOrValue<f64>>) {
        self.corner_radius = radius.into();
    }
}

impl<T: Data> Widget<T> for Hover<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.inner.layout(ctx, bc, data, env);
        self.inner.set_layout_rect(ctx, data, env, size.to_rect());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let mut background: BackgroundBrush<T> = if ctx.is_hot() {
            env.get(HOVER_HOT_COLOR).into()
        } else {
            env.get(HOVER_COLD_COLOR).into()
        };
        let corner_radius = self.corner_radius.resolve(env);
        let panel = ctx.size().to_rounded_rect(corner_radius);
        ctx.with_save(|ctx| {
            ctx.clip(panel);
            background.paint(ctx, data, env);
        });
        self.inner.paint(ctx, data, env);
    }
}

pub trait HoverExt<T: Data>: Widget<T> + Sized + 'static {
    fn hover(self) -> Hover<T> {
        Hover::new(self)
    }

    fn on_ex_click(
        self,
        f: impl Fn(&mut EventCtx, &MouseEvent, &mut T, &Env) + 'static,
    ) -> ControllerHost<Self, ExClick<T>> {
        ControllerHost::new(self, ExClick::new(f))
    }
}

impl<T: Data, W: Widget<T> + 'static> HoverExt<T> for W {}
