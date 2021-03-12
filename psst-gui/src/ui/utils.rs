use crate::{error::Error, ui::theme, widget::icons};
use druid::{
    image,
    kurbo::Line,
    widget::{
        prelude::*, BackgroundBrush, CrossAxisAlignment, FillStrat, Flex, Image, Label, Painter,
        SizedBox,
    },
    Affine, Color, Data, ImageBuf, KeyOrValue, RenderContext, Widget, WidgetExt,
};
use std::{f64::consts::TAU, time::Duration};

pub enum Border {
    Top,
    Bottom,
}

impl Border {
    pub fn widget<T: Data>(
        self,
        color: impl Into<KeyOrValue<Color>>,
    ) -> impl Into<BackgroundBrush<T>> {
        let color = color.into();

        Painter::new(move |ctx, _, env| {
            let h = 1.0;
            let y = match self {
                Self::Top => h / 2.0,
                Self::Bottom => ctx.size().height - h / 2.0,
            };
            let color = color.resolve(&env);
            let line = Line::new((0.0, y), (ctx.size().width, y));
            ctx.stroke(line, &color, h);
        })
    }
}

struct WashingMachine<W> {
    inner: W,
    t: f64,
}

impl<W> WashingMachine<W> {
    pub fn new(inner: W) -> Self {
        Self { inner, t: 0.0 }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for WashingMachine<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::AnimFrame(interval) = event {
            self.t += (*interval as f64) * 1e-9;
            if self.t >= 1.0 {
                self.t = 0.0;
            }
            ctx.request_anim_frame();
            ctx.request_paint();
        }
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            ctx.request_anim_frame();
            ctx.request_paint();
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.with_save(|ctx| {
            let s = ctx.size();
            let tx = Affine::translate((s.width / 2.0, s.height / 2.0))
                * Affine::rotate(TAU * self.t)
                * Affine::translate((-s.width / 2.0, -s.height / 2.0));
            ctx.transform(tx);
            self.inner.paint(ctx, data, env);
        });
    }
}

pub fn placeholder_widget<T: Data>() -> impl Widget<T> {
    SizedBox::empty().background(theme::BACKGROUND_DARK)
}

pub fn spinner_widget<T: Data>() -> impl Widget<T> {
    let bytes = include_bytes!("../../assets/loader.png");
    let img = image::load_from_memory_with_format(&bytes[..], image::ImageFormat::Png).unwrap();
    let buf = ImageBuf::from_dynamic_image_with_alpha(img);
    let loader = Image::new(buf).fill_mode(FillStrat::None);
    let rotating = WashingMachine::new(loader);
    rotating
        .fix_size(theme::grid(4.0), theme::grid(4.0))
        .padding((0.0, theme::grid(10.0)))
        .center()
}

pub fn error_widget() -> impl Widget<Error> {
    let icon = icons::SAD_FACE
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
    format!("{}:{:02}", minutes, seconds)
}
