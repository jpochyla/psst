use druid::{
    kurbo::{BezPath, Size},
    piet::{LineCap, LineJoin, LinearGradient, RenderContext, StrokeStyle, UnitPoint},
    theme,
    widget::{prelude::*, Label, LabelText},
    Affine,
};

pub struct Checkbox {
    label: Label<bool>,
}

impl Checkbox {
    pub fn new(text: impl Into<LabelText<bool>>) -> Checkbox {
        Checkbox {
            label: Label::new(text),
        }
    }
}

impl Widget<bool> for Checkbox {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                if !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() && !ctx.is_disabled() {
                    if ctx.is_hot() {
                        *data = !*data;
                    }
                    ctx.request_paint();
                }
                ctx.set_active(false);
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &bool, env: &Env) {
        self.label.lifecycle(ctx, event, data, env);
        if matches!(
            event,
            LifeCycle::HotChanged(_) | LifeCycle::DisabledChanged(_)
        ) {
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, env: &Env) {
        self.label.update(ctx, old_data, data, env);
        ctx.request_paint();
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &bool, env: &Env) -> Size {
        let x_padding = env.get(theme::WIDGET_CONTROL_COMPONENT_PADDING);
        let check_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let label_size = self.label.layout(ctx, bc, data, env);

        let desired_size = Size::new(
            check_size + x_padding + label_size.width,
            check_size.max(label_size.height),
        );
        let our_size = bc.constrain(desired_size);
        let baseline = self.label.baseline_offset() + (our_size.height - label_size.height);
        ctx.set_baseline_offset(baseline);
        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &bool, env: &Env) {
        let size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let x_padding = env.get(theme::WIDGET_CONTROL_COMPONENT_PADDING);
        let border_width = 1.;

        let rect = Size::new(size, size)
            .to_rect()
            .inset(-border_width / 2.)
            .to_rounded_rect(2.);

        // Paint the background
        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );

        ctx.fill(rect, &background_gradient);

        let border_color = if ctx.is_hot() && !ctx.is_disabled() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        ctx.stroke(rect, &border_color, border_width);

        if *data {
            // Paint the checkmark
            let mut path = BezPath::new();
            path.move_to((4.0, 9.0));
            path.line_to((8.0, 13.0));
            path.line_to((14.0, 5.0));

            let style = StrokeStyle::new()
                .line_cap(LineCap::Round)
                .line_join(LineJoin::Round);

            let brush = if ctx.is_disabled() {
                env.get(theme::DISABLED_TEXT_COLOR)
            } else {
                env.get(theme::TEXT_COLOR)
            };

            ctx.with_save(|ctx| {
                ctx.transform(Affine::scale(size / 18.0));
                ctx.stroke_styled(path, &brush, 2., &style);
            })
        }

        // Paint the text label
        self.label.draw_at(ctx, (size + x_padding, 0.0));
    }
}
