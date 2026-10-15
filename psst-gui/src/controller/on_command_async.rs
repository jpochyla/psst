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
    request_serial: u64,
    latest_only: bool,
}

impl<W, T, U, V> OnCommandAsync<W, T, U, V>
where
    W: Widget<T>,
{
    const RESPONSE: Selector<SingleUse<(u64, U, V)>> = Selector::new("on_cmd_async.response");

    /// For replaceable reads; writes retain every completion by default.
    pub fn latest_only(mut self) -> Self {
        self.latest_only = true;
        self
    }

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
            request_serial: 0,
            latest_only: false,
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
                self.request_serial = self.request_serial.wrapping_add(1);
                let serial = self.request_serial;

                let old_thread = self.thread.replace(thread::spawn({
                    let req_fn = self.request_fn.clone();
                    let req = req.to_owned();
                    let sink = ctx.get_external_handle();
                    let self_id = ctx.widget_id();

                    move || {
                        let res = req_fn(req.clone());
                        let _ = sink.submit_command(
                            Self::RESPONSE,
                            SingleUse::new((serial, req, res)),
                            Target::Widget(self_id),
                        );
                    }
                }));
                if old_thread.is_some() {
                    log::warn!("async action pending");
                }
            }
            Event::Command(cmd) if cmd.is(Self::RESPONSE) => {
                let (serial, req, res) = cmd.get_unchecked(Self::RESPONSE).take().unwrap();
                if accepts_response(self.request_serial, serial, self.latest_only) {
                    (self.response_fn)(ctx, data, (req, res));
                }
                if serial == self.request_serial {
                    self.thread.take();
                }
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

fn accepts_response(current: u64, response: u64, latest_only: bool) -> bool {
    !latest_only || response == current
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn returning_to_the_same_query_does_not_accept_its_previous_response() {
        // Query A, then B, then A again. A string comparison alone accepts request 1.
        assert!(!accepts_response(3, 1, true));
        assert!(!accepts_response(3, 2, true));
        assert!(accepts_response(3, 3, true));
        // Independent playlist/library writes must still deliver all completions.
        assert!(accepts_response(3, 1, false));
    }
}
