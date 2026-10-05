use std::time::Duration;

use druid::{
    piet::{FontFamily, Text, TextLayout, TextLayoutBuilder},
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Rect, RenderContext, Size, TimerToken, UpdateCtx, Widget,
};

use crate::ui::theme;

/// Paint above the entire window so hints also work in clipped lists and the bottom player.
pub struct Tooltip<W> {
    inner: W,
    text: String,
    timer: TimerToken,
    visible: bool,
}

impl<W> Tooltip<W> {
    pub fn new(inner: W, text: impl Into<String>) -> Self {
        Self {
            inner,
            text: text.into(),
            timer: TimerToken::INVALID,
            visible: false,
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Tooltip<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseMove(_)
                if ctx.is_hot() && !self.visible && self.timer == TimerToken::INVALID =>
            {
                self.timer = ctx.request_timer(Duration::from_millis(500));
            }
            Event::Timer(token) if *token == self.timer => {
                self.timer = TimerToken::INVALID;
                self.visible = ctx.is_hot() && !ctx.is_active();
                ctx.window().invalidate();
            }
            Event::MouseDown(_) | Event::KeyDown(_) | Event::WindowDisconnected => {
                self.timer = TimerToken::INVALID;
                if self.visible {
                    self.visible = false;
                    ctx.window().invalidate();
                }
            }
            _ => {}
        }
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if matches!(event, LifeCycle::HotChanged(false)) {
            self.timer = TimerToken::INVALID;
            if self.visible {
                self.visible = false;
                ctx.window().invalidate();
            }
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
        if !self.visible || !ctx.is_hot() {
            return;
        }
        let window = ctx.window().get_size();
        let origin = ctx.window_origin();
        let size = ctx.size();
        let background = env.get(theme::WINDOW_BACKGROUND_COLOR);
        let foreground = env.get(theme::TEXT_COLOR);
        let border = env.get(theme::GREY_400);
        let Ok(layout) = ctx
            .text()
            .new_text_layout(self.text.clone())
            .font(FontFamily::SYSTEM_UI, 12.0)
            .text_color(foreground)
            .max_width(280.0_f64.min((window.width - 40.0).max(1.0)))
            .build()
        else {
            return;
        };
        let hint = Size::new(layout.size().width + 20.0, layout.size().height + 16.0);
        let x = (origin.x + size.width / 2.0 - hint.width / 2.0)
            .clamp(8.0, (window.width - hint.width - 8.0).max(8.0));
        let above = origin.y - hint.height - 8.0;
        let y = if above >= 8.0 {
            above
        } else {
            origin.y + size.height + 8.0
        };
        let position = Point::new(x - origin.x, y - origin.y);
        ctx.paint_with_z_index(u32::MAX, move |ctx| {
            let rect = Rect::from_origin_size(position, hint).to_rounded_rect(6.0);
            ctx.fill(rect, &background);
            ctx.stroke(rect, &border, 1.0);
            ctx.draw_text(&layout, position + (10.0, 8.0));
        });
    }
}
