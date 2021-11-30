use std::time::Duration;

use druid::{
    widget::Controller, Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, TimerToken, Widget,
};

type DelayFunc<T> = Box<dyn FnOnce(&mut EventCtx, &mut T, &Env)>;

pub struct AfterDelay<T> {
    duration: Duration,
    timer: TimerToken,
    func: Option<DelayFunc<T>>,
}

impl<T> AfterDelay<T> {
    pub fn new(
        duration: Duration,
        func: impl FnOnce(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> Self {
        Self {
            duration,
            timer: TimerToken::INVALID,
            func: Some(Box::new(func)),
        }
    }
}

impl<T, W> Controller<T, W> for AfterDelay<T>
where
    T: Data,
    W: Widget<T>,
{
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Timer(token) if token == &self.timer => {
                if let Some(func) = self.func.take() {
                    func(ctx, data, env);
                }
                self.timer = TimerToken::INVALID;
            }
            _ => child.event(ctx, event, data, env),
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.timer = ctx.request_timer(self.duration);
        }
        child.lifecycle(ctx, event, data, env)
    }
}
