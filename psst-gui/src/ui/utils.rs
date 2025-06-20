use std::{f64::consts::PI, time::Duration};

use druid::{
    kurbo::Circle,
    widget::{prelude::*, CrossAxisAlignment, Flex, Label, SizedBox},
    Data, Point, Vec2, Widget, WidgetExt, WidgetPod,
};
use time_humanize::HumanTime;

use crate::{data::WithCtx, error::Error, widget::icons};

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

pub fn stat_row<T: Data>(
    label: &'static str,
    value_func: impl Fn(&T) -> String + 'static,
) -> impl Widget<WithCtx<T>> {
    Flex::row()
        .with_child(
            Label::new(label)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .with_text_color(theme::PLACEHOLDER_COLOR),
        )
        .with_spacer(theme::grid(0.5))
        .with_child(
            Label::new(move |ctx: &WithCtx<T>, _env: &_| value_func(&ctx.data))
                .with_text_size(theme::TEXT_SIZE_SMALL),
        )
        .align_left()
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

pub fn as_minutes_and_seconds(dur: Duration) -> String {
    let minutes = dur.as_secs() / 60;
    let seconds = dur.as_secs() % 60;
    format!("{}∶{:02}", minutes, seconds)
}

pub fn as_human(dur: Duration) -> String {
    HumanTime::from(dur).to_text_en(
        time_humanize::Accuracy::Rough,
        time_humanize::Tense::Present,
    )
}

pub fn format_number_with_commas(n: i64) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }
    // Reverse the string, chunk it, then reverse the chunks to process from left to right.
    s.chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .rev()
        // Reverse the characters in each chunk back to their original order and collect into a string.
        .map(|chunk| chunk.iter().rev().collect::<String>())
        .collect::<Vec<_>>()
        // Join the chunks with commas.
        .join(",")
}

pub struct InfoLayout<T, B, S> {
    biography: WidgetPod<T, B>,
    stats: WidgetPod<T, S>,
}

impl<T, B, S> InfoLayout<T, B, S>
where
    T: Data,
    B: Widget<T>,
    S: Widget<T>,
{
    pub fn new(biography: B, stats: S) -> Self {
        Self {
            biography: WidgetPod::new(biography),
            stats: WidgetPod::new(stats),
        }
    }
}

impl<T: Data, B: Widget<T>, S: Widget<T>> Widget<T> for InfoLayout<T, B, S> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.biography.event(ctx, event, data, env);
        self.stats.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.biography.lifecycle(ctx, event, data, env);
        self.stats.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.biography.update(ctx, data, env);
        self.stats.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let max = bc.max();
        let wide_layout = max.width > theme::grid(60.0) + theme::GRID * 3.45;
        let padding = theme::grid(1.0);
        let image_height = theme::grid(16.0);

        if wide_layout {
            // In wide layout, the biography is left of the stats.
            // The biography's height is constrained to the image height.
            let biography_width = max.width * 0.67 - padding / 2.0;
            let stats_width = max.width * 0.33 - padding / 2.0;

            let biography_bc =
                BoxConstraints::new(Size::ZERO, Size::new(biography_width, image_height));
            let stats_bc = BoxConstraints::new(Size::ZERO, Size::new(stats_width, max.height));

            let biography_size = self.biography.layout(ctx, &biography_bc, data, env);
            let stats_size = self.stats.layout(ctx, &stats_bc, data, env);

            self.biography.set_origin(ctx, Point::ORIGIN);
            self.stats
                .set_origin(ctx, Point::new(biography_width + padding, 0.0));

            Size::new(max.width, biography_size.height.max(stats_size.height))
        } else {
            // In narrow view, the biography and stats are stacked vertically, and
            // their combined height should be equal to the image height.
            let stats_bc = BoxConstraints::new(Size::ZERO, Size::new(max.width, max.height));
            let stats_size = self.stats.layout(ctx, &stats_bc, data, env);

            let biography_height = (image_height - stats_size.height - padding).max(0.0);
            let biography_bc = BoxConstraints::tight(Size::new(max.width, biography_height));
            let biography_size = self.biography.layout(ctx, &biography_bc, data, env);

            self.biography.set_origin(ctx, Point::ORIGIN);
            self.stats
                .set_origin(ctx, Point::new(0.0, biography_size.height + padding));

            Size::new(max.width, image_height)
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.biography.paint(ctx, data, env);
        self.stats.paint(ctx, data, env);
    }
}
