use crate::{data::AppState, ui::theme};
use druid::widget::prelude::*;

pub struct ThemeScope<W> {
    inner: W,
    cached_env: Option<Env>,
    resolved_theme: Option<crate::data::Theme>,
    theme_timer: druid::TimerToken,
}

impl<W> ThemeScope<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            cached_env: None,
            resolved_theme: None,
            theme_timer: druid::TimerToken::INVALID,
        }
    }

    fn set_env(&mut self, data: &AppState, outer_env: &Env) {
        self.resolved_theme = Some(crate::ui::system_theme::resolve(data.config.theme));
        let mut themed_env = outer_env.clone();
        theme::setup(&mut themed_env, data);
        self.cached_env.replace(themed_env);
    }
}

impl<W: Widget<AppState>> Widget<AppState> for ThemeScope<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        if matches!(event, Event::WindowConnected) {
            self.theme_timer = ctx.request_timer(std::time::Duration::from_secs(2));
        }
        if matches!(event, Event::Timer(token) if *token == self.theme_timer) {
            if self.resolved_theme != Some(crate::ui::system_theme::resolve(data.config.theme)) {
                self.set_env(data, env);
                ctx.request_layout();
                ctx.request_paint();
            }
            self.theme_timer = ctx.request_timer(std::time::Duration::from_secs(2));
        }
        self.inner
            .event(ctx, event, data, self.cached_env.as_ref().unwrap_or(env))
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        if let LifeCycle::WidgetAdded = &event {
            self.set_env(data, env);
        }
        self.inner
            .lifecycle(ctx, event, data, self.cached_env.as_ref().unwrap_or(env))
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppState, data: &AppState, env: &Env) {
        if !data.config.theme.same(&old_data.config.theme) {
            self.set_env(data, env);
            ctx.request_layout();
            ctx.request_paint();
        }
        self.inner
            .update(ctx, old_data, data, self.cached_env.as_ref().unwrap_or(env));
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppState,
        env: &Env,
    ) -> Size {
        self.inner
            .layout(ctx, bc, data, self.cached_env.as_ref().unwrap_or(env))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
        self.inner
            .paint(ctx, data, self.cached_env.as_ref().unwrap_or(env));
    }
}
