use std::{
    any::Any,
    sync::Arc,
    thread::{self, JoinHandle},
};

use crate::data::{Promise, PromiseState};
use druid::{
    widget::{prelude::*, Controller},
    Data, ExtEventSink, Point, Selector, SingleUse, Target, WidgetExt, WidgetPod,
};

pub struct AsyncAction<T, D, E> {
    func: Arc<dyn Fn(&D) -> Result<T, E> + Sync + Send + 'static>,
    handle: Option<JoinHandle<()>>,
}

struct AsyncResult {
    result: Box<dyn Any + Send>,
    deferred: Box<dyn Any + Send>,
}

const ASYNC_RESULT: Selector<SingleUse<AsyncResult>> = Selector::new("promise.async_result");

impl<T, D, E> AsyncAction<T, D, E>
where
    T: Send + 'static,
    D: Send + 'static,
    E: Send + 'static,
{
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&D) -> Result<T, E> + Sync + Send + 'static,
    {
        Self {
            func: Arc::new(func),
            handle: None,
        }
    }

    fn spawn_action(&mut self, self_id: WidgetId, event_sink: ExtEventSink, deferred: D) {
        let old_handle = self.handle.replace(thread::spawn({
            let func = self.func.clone();
            move || {
                let result = AsyncResult {
                    result: Box::new(func(&deferred)),
                    deferred: Box::new(deferred),
                };
                event_sink
                    .submit_command(
                        ASYNC_RESULT,
                        SingleUse::new(result),
                        Target::Widget(self_id),
                    )
                    .unwrap();
            }
        }));
        if old_handle.is_some() {
            log::warn!("async action pending");
        }
    }
}

impl<T, D, E, W> Controller<Promise<T, D, E>, W> for AsyncAction<T, D, E>
where
    T: Send + Data,
    D: Send + Data + PartialEq,
    E: Send + Data,
    W: Widget<Promise<T, D, E>>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Promise<T, D, E>,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(ASYNC_RESULT) => {
                let payload = cmd.get_unchecked(ASYNC_RESULT).take().unwrap();
                let result = payload.result.downcast().unwrap();
                let deferred = payload.deferred.downcast().unwrap();
                if data.is_deferred(&deferred) {
                    data.resolve_or_reject(*result);
                }
                self.handle.take();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Promise<T, D, E>,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if let Promise::Deferred(deferred) = data {
                self.spawn_action(
                    ctx.widget_id(),
                    ctx.get_external_handle(),
                    deferred.to_owned(),
                );
            }
        }
        child.lifecycle(ctx, event, data, env)
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &Promise<T, D, E>,
        data: &Promise<T, D, E>,
        env: &Env,
    ) {
        if !old_data.same(data) {
            if let Promise::Deferred(deferred) = data {
                self.spawn_action(
                    ctx.widget_id(),
                    ctx.get_external_handle(),
                    deferred.to_owned(),
                );
            }
        }
        child.update(ctx, old_data, data, env)
    }
}

pub struct Async<T, D, E> {
    def_maker: Box<dyn Fn() -> Box<dyn Widget<D>>>,
    res_maker: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    err_maker: Box<dyn Fn() -> Box<dyn Widget<E>>>,
    widget: PromiseWidget<T, D, E>,
}

#[allow(clippy::large_enum_variant)]
enum PromiseWidget<T, D, E> {
    Empty,
    Deferred(WidgetPod<D, Box<dyn Widget<D>>>),
    Resolved(WidgetPod<T, Box<dyn Widget<T>>>),
    Rejected(WidgetPod<E, Box<dyn Widget<E>>>),
}

impl<D: Data, T: Data, E: Data> Async<T, D, E> {
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
            widget: PromiseWidget::Empty,
        }
    }

    fn rebuild_widget(&mut self, state: PromiseState) {
        self.widget = match state {
            PromiseState::Empty => PromiseWidget::Empty,
            PromiseState::Deferred => PromiseWidget::Deferred(WidgetPod::new((self.def_maker)())),
            PromiseState::Resolved => PromiseWidget::Resolved(WidgetPod::new((self.res_maker)())),
            PromiseState::Rejected => PromiseWidget::Rejected(WidgetPod::new((self.err_maker)())),
        };
    }
}

impl<D: Data, T: Data, E: Data> Widget<Promise<T, D, E>> for Async<T, D, E> {
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
                w.set_origin(ctx, d, env, Point::ORIGIN);
                size
            }),
            Promise::Resolved(o) => self.widget.with_resolved(|w| {
                let size = w.layout(ctx, bc, o, env);
                w.set_origin(ctx, o, env, Point::ORIGIN);
                size
            }),
            Promise::Rejected(e) => self.widget.with_rejected(|w| {
                let size = w.layout(ctx, bc, e, env);
                w.set_origin(ctx, e, env, Point::ORIGIN);
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

impl<T, D, E> PromiseWidget<T, D, E> {
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
