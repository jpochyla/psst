use std::{f64::consts::PI, time::Duration};

use druid::{
    kurbo::Circle,
    widget::{prelude::*, CrossAxisAlignment, Flex, Label, SizedBox},
    Data, Vec2, Widget, WidgetExt,
};

use crate::{error::Error, widget::icons};

use super::theme;

struct Spinner {
    t: f64,
}

impl Spinner {
    pub fn new() -> Self {
        Self { t: 0.0 }
    }
}

impl<T: Data> Widget<T> for Spinner {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        if let Event::AnimFrame(interval) = event {
            self.t += (*interval as f64) * 1e-9;
            if self.t >= 1.0 {
                self.t = 0.0;
            }
            ctx.request_anim_frame();
            ctx.request_paint();
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &T, _env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            ctx.request_anim_frame();
            ctx.request_paint();
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.constrain(Size::new(theme::grid(6.0), theme::grid(16.0)))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let center = ctx.size().to_rect().center();
        let c0 = env.get(theme::GREY_500);
        let c1 = env.get(theme::GREY_400);
        let active = 7 - (1 + (6.0 * self.t).floor() as i32);
        for i in 1..=6 {
            let step = f64::from(i);
            let angle = Vec2::from_angle((step / 6.0) * -2.0 * PI);
            let dot_center = center + angle * theme::grid(2.0);
            let dot = Circle::new(dot_center, theme::grid(0.8));
            if i == active {
                ctx.fill(dot, &c1);
            } else {
                ctx.fill(dot, &c0);
            }
        }
    }
}

pub fn placeholder_widget<T: Data>() -> impl Widget<T> {
    SizedBox::empty().background(theme::BACKGROUND_DARK)
}

pub fn spinner_widget<T: Data>() -> impl Widget<T> {
    Spinner::new().center()
}

pub fn error_widget() -> impl Widget<Error> {
    let icon = icons::ERROR
        .scale((theme::grid(3.0), theme::grid(3.0)))
        .with_color(theme::PLACEHOLDER_COLOR);
    let error = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::new("Error:")
                .with_font(theme::UI_FONT_MEDIUM)
                .with_text_color(theme::PLACEHOLDER_COLOR),
        )
        .with_child(
            Label::dynamic(|err: &Error, _| err.to_string())
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .with_text_color(theme::PLACEHOLDER_COLOR),
        );
    Flex::row()
        .with_child(icon)
        .with_default_spacer()
        .with_child(error)
        .padding((0.0, theme::grid(6.0)))
        .center()
}

pub fn as_minutes_and_seconds(dur: &Duration) -> String {
    let minutes = dur.as_secs() / 60;
    let seconds = dur.as_secs() % 60;
    format!("{}∶{:02}", minutes, seconds)
}
