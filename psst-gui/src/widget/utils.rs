use druid::{
    kurbo::{Line, Shape},
    widget::{prelude::*, BackgroundBrush, Painter},
    Color, Data, KeyOrValue,
};

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

pub enum Border {
    Top,
    Bottom,
}

impl Border {
    pub fn with_color<T: Data>(
        self,
        color: impl Into<KeyOrValue<Color>>,
    ) -> impl Into<BackgroundBrush<T>> {
        let color = color.into();

        Painter::new(move |ctx, _, env| {
            let h = 1.0;
            let y = match self {
                Self::Top => h / 2.0,
                Self::Bottom => ctx.size().height - h / 2.0,
            };
            let line = Line::new((0.0, y), (ctx.size().width, y));
            ctx.stroke(line, &color.resolve(env), h);
        })
    }
}

pub struct Logger<W> {
    inner: W,
    label: &'static str,
    event: bool,
    lifecycle: bool,
    update: bool,
    layout: bool,
    paint: bool,
}

#[allow(dead_code)]
impl<W> Logger<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            label: "logger",
            event: false,
            lifecycle: false,
            update: false,
            layout: false,
            paint: false,
        }
    }

    pub fn with_label(mut self, title: &'static str) -> Self {
        self.label = title;
        self
    }

    pub fn with_event(mut self) -> Self {
        self.event = true;
        self
    }

    pub fn with_lifecycle(mut self) -> Self {
        self.lifecycle = true;
        self
    }

    pub fn with_update(mut self) -> Self {
        self.update = true;
        self
    }

    pub fn with_layout(mut self) -> Self {
        self.layout = true;
        self
    }

    pub fn with_paint(mut self) -> Self {
        self.paint = true;
        self
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Logger<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if self.event {
            log::info!("{:?} event: {:?}", self.label, event);
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if self.lifecycle {
            log::info!("{:?} lifecycle: {:?}", self.label, event);
        }
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if self.update {
            log::info!("{:?} update", self.label);
        }
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if self.layout {
            log::info!("{:?} layout", self.label);
        }
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.paint {
            log::info!("{:?} paint", self.label);
        }
        self.inner.paint(ctx, data, env)
    }
}
