use crate::{
    error::Error,
    promise::{Promise, PromiseState},
};
use druid::{widget::prelude::*, Data, WidgetExt, WidgetPod};

pub struct Promised<T, D, E> {
    def_maker: Box<dyn Fn() -> Box<dyn Widget<D>>>,
    res_maker: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    err_maker: Box<dyn Fn() -> Box<dyn Widget<E>>>,
    widget: DefWidget<T, D, E>,
}

#[allow(clippy::large_enum_variant)]
enum DefWidget<T, D, E> {
    Empty,
    Deferred(WidgetPod<D, Box<dyn Widget<D>>>),
    Resolved(WidgetPod<T, Box<dyn Widget<T>>>),
    Rejected(WidgetPod<E, Box<dyn Widget<E>>>),
}

impl<D: Data, T: Data, E: Data> Promised<T, D, E> {
    pub fn new<WD, WT, WE>(
        def_maker: impl Fn() -> WD + 'static,
        res_maker: impl Fn() -> WT + 'static,
        err_maker: impl Fn() -> WE + 'static,
    ) -> Self
    where
        WD: Widget<D> + 'static,
        WT: Widget<T> + 'static,
        WE: Widget<E> + 'static,
    {
        Self {
            def_maker: Box::new(move || def_maker().boxed()),
            res_maker: Box::new(move || res_maker().boxed()),
            err_maker: Box::new(move || err_maker().boxed()),
            widget: DefWidget::Empty,
        }
    }

    fn rebuild_widget(&mut self, state: PromiseState) {
        self.widget = match state {
            PromiseState::Empty => DefWidget::Empty,
            PromiseState::Deferred => DefWidget::Deferred(WidgetPod::new((self.def_maker)())),
            PromiseState::Resolved => DefWidget::Resolved(WidgetPod::new((self.res_maker)())),
            PromiseState::Rejected => DefWidget::Rejected(WidgetPod::new((self.err_maker)())),
        };
    }
}

impl<D: Data, T: Data, E: Data> Widget<Promise<T, D, E>> for Promised<T, D, E> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Promise<T, D, E>, env: &Env) {
        if data.state() == self.widget.state() {
            match data {
                Promise::Empty => {}
                Promise::Deferred(d) => {
                    self.widget.with_deferred(|w| w.event(ctx, event, d, env));
                }
                Promise::Resolved(o) => {
                    self.widget.with_resolved(|w| w.event(ctx, event, o, env));
                }
                Promise::Rejected(e) => {
                    self.widget.with_rejected(|w| w.event(ctx, event, e, env));
                }
            };
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Promise<T, D, E>,
        env: &Env,
    ) {
        if data.state() != self.widget.state() {
            // possible if getting lifecycle after an event that changed the data,
            // or on WidgetAdded
            self.rebuild_widget(data.state());
        }
        assert_eq!(data.state(), self.widget.state(), "{:?}", event);
        match data {
            Promise::Empty => {}
            Promise::Deferred(d) => {
                self.widget
                    .with_deferred(|w| w.lifecycle(ctx, event, d, env));
            }
            Promise::Resolved(o) => {
                self.widget
                    .with_resolved(|w| w.lifecycle(ctx, event, o, env));
            }
            Promise::Rejected(e) => {
                self.widget
                    .with_rejected(|w| w.lifecycle(ctx, event, e, env));
            }
        };
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &Promise<T, D, E>,
        data: &Promise<T, D, E>,
        env: &Env,
    ) {
        if old_data.state() != data.state() {
            self.rebuild_widget(data.state());
            ctx.children_changed();
        } else {
            match data {
                Promise::Empty => {}
                Promise::Deferred(d) => {
                    self.widget.with_deferred(|w| w.update(ctx, d, env));
                }
                Promise::Resolved(o) => {
                    self.widget.with_resolved(|w| w.update(ctx, o, env));
                }
                Promise::Rejected(e) => {
                    self.widget.with_rejected(|w| w.update(ctx, e, env));
                }
            };
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Promise<T, D, E>,
        env: &Env,
    ) -> Size {
        match data {
            Promise::Empty => None,
            Promise::Deferred(d) => self.widget.with_deferred(|w| {
                let size = w.layout(ctx, bc, d, env);
                w.set_layout_rect(ctx, d, env, size.to_rect());
                size
            }),
            Promise::Resolved(o) => self.widget.with_resolved(|w| {
                let size = w.layout(ctx, bc, o, env);
                w.set_layout_rect(ctx, o, env, size.to_rect());
                size
            }),
            Promise::Rejected(e) => self.widget.with_rejected(|w| {
                let size = w.layout(ctx, bc, e, env);
                w.set_layout_rect(ctx, e, env, size.to_rect());
                size
            }),
        }
        .unwrap_or_default()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Promise<T, D, E>, env: &Env) {
        match data {
            Promise::Empty => {}
            Promise::Deferred(d) => {
                self.widget.with_deferred(|w| w.paint(ctx, d, env));
            }
            Promise::Resolved(o) => {
                self.widget.with_resolved(|w| w.paint(ctx, o, env));
            }
            Promise::Rejected(e) => {
                self.widget.with_rejected(|w| w.paint(ctx, e, env));
            }
        };
    }
}

impl<T, D, E> DefWidget<T, D, E> {
    fn state(&self) -> PromiseState {
        match self {
            Self::Empty => PromiseState::Empty,
            Self::Deferred(_) => PromiseState::Deferred,
            Self::Resolved(_) => PromiseState::Resolved,
            Self::Rejected(_) => PromiseState::Rejected,
        }
    }

    fn with_deferred<R, F: FnOnce(&mut WidgetPod<D, Box<dyn Widget<D>>>) -> R>(
        &mut self,
        f: F,
    ) -> Option<R> {
        if let Self::Deferred(widget) = self {
            Some(f(widget))
        } else {
            None
        }
    }

    fn with_resolved<R, F: FnOnce(&mut WidgetPod<T, Box<dyn Widget<T>>>) -> R>(
        &mut self,
        f: F,
    ) -> Option<R> {
        if let Self::Resolved(widget) = self {
            Some(f(widget))
        } else {
            None
        }
    }

    fn with_rejected<R, F: FnOnce(&mut WidgetPod<E, Box<dyn Widget<E>>>) -> R>(
        &mut self,
        f: F,
    ) -> Option<R> {
        if let Self::Rejected(widget) = self {
            Some(f(widget))
        } else {
            None
        }
    }
}
