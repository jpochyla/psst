use std::time::Duration;

use druid::{widget::Controller, Data, Env, Event, EventCtx, TimerToken, UpdateCtx, Widget};

pub struct OnDebounce<T> {
    duration: Duration,
    timer: TimerToken,
    handler: Box<dyn Fn(&mut EventCtx, &mut T, &Env)>,
}

impl<T> OnDebounce<T> {
    pub fn trailing(
        duration: Duration,
        handler: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> Self {
        Self {
            duration,
            timer: TimerToken::INVALID,
            handler: Box::new(handler),
        }
    }
}

impl<T, W> Controller<T, W> for OnDebounce<T>
where
    T: Data,
    W: Widget<T>,
{
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Timer(token) if token == &self.timer => {
                (self.handler)(ctx, data, env);
                self.timer = TimerToken::INVALID;
                ctx.set_handled();
            }
            _ => child.event(ctx, event, data, env),
        }
    }

    fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if !old_data.same(data) {
            self.timer = ctx.request_timer(self.duration);
        }
        child.update(ctx, old_data, data, env)
    }
}
