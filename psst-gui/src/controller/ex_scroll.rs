use crate::data::SliderScrollScale;
use druid::{widget::Controller, Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, Widget};

pub struct ExScroll<T> {
    scale_picker: Box<dyn Fn(&mut T) -> &SliderScrollScale>,
    action: Box<dyn Fn(&mut EventCtx, &mut T, &Env, f64)>,
}

impl<T: Data> ExScroll<T> {
    pub fn new(
        scale_picker: impl Fn(&mut T) -> &SliderScrollScale + 'static,
        action: impl Fn(&mut EventCtx, &mut T, &Env, f64) + 'static,
    ) -> Self {
        ExScroll {
            scale_picker: Box::new(scale_picker),
            action: Box::new(action),
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for ExScroll<T> {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::Wheel(mouse_event) = event {
            ctx.set_active(true);

            let delta = mouse_event.wheel_delta;
            let scale_config = (self.scale_picker)(data);
            let scale = scale_config.scale / 100.;

            let (directional_scale, delta) = if delta.x == 0. {
                (scale_config.y, -delta.y)
            } else {
                (scale_config.x, delta.x)
            };
            let scaled_delta = delta.signum() * scale * 1. / directional_scale;
            (self.action)(ctx, data, env, scaled_delta);

            ctx.set_active(false);
            ctx.request_paint()
        }

        child.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        if let LifeCycle::HotChanged(_) | LifeCycle::FocusChanged(_) = event {
            ctx.request_paint();
        }

        child.lifecycle(ctx, event, data, env);
    }
}
