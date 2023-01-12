use std::{
    sync::Arc,
    thread::{self, JoinHandle},
};

use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Selector, SingleUse, Size, Target, UpdateCtx, Widget, WidgetPod,
};

type AsyncCmdPre<T, U> = Box<dyn Fn(&mut EventCtx, &mut T, U)>;
type AsyncCmdReq<U, V> = Arc<dyn Fn(U) -> V + Sync + Send + 'static>;
type AsyncCmdRes<T, U, V> = Box<dyn Fn(&mut EventCtx, &mut T, (U, V))>;

pub struct OnCommandAsync<W, T, U, V> {
    child: WidgetPod<T, W>,
    selector: Selector<U>,
    preflight_fn: AsyncCmdPre<T, U>,
    request_fn: AsyncCmdReq<U, V>,
    response_fn: AsyncCmdRes<T, U, V>,
    thread: Option<JoinHandle<()>>,
}

impl<W, T, U, V> OnCommandAsync<W, T, U, V>
where
    W: Widget<T>,
{
    const RESPONSE: Selector<SingleUse<(U, V)>> = Selector::new("on_cmd_async.response");

    pub fn new(
        child: W,
        selector: Selector<U>,
        preflight_fn: AsyncCmdPre<T, U>,
        request_fn: AsyncCmdReq<U, V>,
        response_fn: AsyncCmdRes<T, U, V>,
    ) -> Self {
        Self {
            child: WidgetPod::new(child),
            selector,
            preflight_fn,
            request_fn,
            response_fn,
            thread: None,
        }
    }
}

impl<W, T, U, V> Widget<T> for OnCommandAsync<W, T, U, V>
where
    W: Widget<T>,
    T: Data,
    U: Send + Clone + 'static,
    V: Send + 'static,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(self.selector) => {
                let req = cmd.get_unchecked(self.selector);

                (self.preflight_fn)(ctx, data, req.to_owned());

                let old_thread = self.thread.replace(thread::spawn({
                    let req_fn = self.request_fn.clone();
                    let req = req.to_owned();
                    let sink = ctx.get_external_handle();
                    let self_id = ctx.widget_id();

                    move || {
                        let res = req_fn(req.clone());
                        sink.submit_command(
                            Self::RESPONSE,
                            SingleUse::new((req, res)),
                            Target::Widget(self_id),
                        )
                        .unwrap();
                    }
                }));
                if old_thread.is_some() {
                    log::warn!("async action pending");
                }
            }
            Event::Command(cmd) if cmd.is(Self::RESPONSE) => {
                let res = cmd.get_unchecked(Self::RESPONSE).take().unwrap();
                (self.response_fn)(ctx, data, res);
                self.thread.take();
                ctx.set_handled();
            }
            _ => {
                self.child.event(ctx, event, data, env);
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.child.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
    }
}
