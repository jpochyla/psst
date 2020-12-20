use crate::ui::theme;
use druid::{kurbo::BezPath, widget::prelude::*, Affine, Color, KeyOrValue, Size};

pub static TIME: SvgIcon = SvgIcon {
    svg_path: "M8 0.633095C3.92308 0.633095 0.615383 3.93285 0.615383 8C0.615383 12.0671 3.92308 15.3669 8 15.3669C12.0769 15.3669 15.3846 12.0671 15.3846 8C15.3846 3.93285 12.0769 0.633095 8 0.633095Z M8 3.08873V8.61391H11.6923",
    svg_size: Size::new(16.0, 16.0),
    op: PaintOp::Stroke { width: 1.0 },
};
pub static HEART: SvgIcon = SvgIcon {
    svg_path: "M11.7099 0.642857C9.22488 0.642857 8 3.13636 8 3.13636C8 3.13636 6.77512 0.642857 4.29014 0.642857C2.27062 0.642857 0.671387 2.3626 0.650717 4.41467C0.608612 8.67428 3.97053 11.7035 7.6555 14.2492C7.75709 14.3196 7.87713 14.3572 8 14.3572C8.12287 14.3572 8.24291 14.3196 8.3445 14.2492C12.0291 11.7035 15.391 8.67428 15.3493 4.41467C15.3286 2.3626 13.7294 0.642857 11.7099 0.642857V0.642857Z",
    svg_size: Size::new(16.0, 15.0),
    op: PaintOp::Stroke { width: 1.0 },
};
#[allow(dead_code)]
pub static SEARCH: SvgIcon = SvgIcon {
    svg_path: "M14.2716 13.1684L11.3313 10.2281C12.0391 9.28573 12.4213 8.13865 12.42 6.96C12.42 3.94938 9.97062 1.5 6.96 1.5C3.94938 1.5 1.5 3.94938 1.5 6.96C1.5 9.97062 3.94938 12.42 6.96 12.42C8.13865 12.4213 9.28573 12.0391 10.2281 11.3313L13.1684 14.2716C13.3173 14.4046 13.5114 14.4756 13.711 14.47C13.9105 14.4645 14.1004 14.3827 14.2415 14.2415C14.3827 14.1004 14.4645 13.9105 14.47 13.711C14.4756 13.5114 14.4046 13.3173 14.2716 13.1684ZM3.06 6.96C3.06 6.18865 3.28873 5.43463 3.71727 4.79328C4.14581 4.15192 4.7549 3.65205 5.46753 3.35687C6.18017 3.06169 6.96433 2.98446 7.72085 3.13494C8.47738 3.28542 9.17229 3.65686 9.71772 4.20228C10.2631 4.74771 10.6346 5.44262 10.7851 6.19915C10.9355 6.95567 10.8583 7.73983 10.5631 8.45247C10.2679 9.1651 9.76808 9.77419 9.12672 10.2027C8.48537 10.6313 7.73135 10.86 6.96 10.86C5.92604 10.8588 4.93478 10.4475 4.20365 9.71635C3.47253 8.98522 3.06124 7.99396 3.06 6.96Z",
    svg_size: Size::new(16.0, 16.0),
    op: PaintOp::Fill,
};
pub static BACK: SvgIcon = SvgIcon {
    svg_path: "M9.70711 0.292893C10.0976 0.683417 10.0976 1.31658 9.70711 1.70711L2.41421 9L9.70711 16.2929C10.0976 16.6834 10.0976 17.3166 9.70711 17.7071C9.31658 18.0976 8.68342 18.0976 8.29289 17.7071L0.292893 9.70711C-0.0976311 9.31658 -0.0976311 8.68342 0.292893 8.29289L8.29289 0.292893C8.68342 -0.0976311 9.31658 -0.0976311 9.70711 0.292893Z",
    svg_size: Size::new(10.0, 18.0),
    op: PaintOp::Fill,
};
pub static PLAY: SvgIcon = SvgIcon {
    svg_path: "M4.92623 21.4262L19.9262 12.4262L4.92623 3.42623V21.4262Z",
    svg_size: Size::new(24.0, 24.0),
    op: PaintOp::Fill,
};
pub static PAUSE: SvgIcon = SvgIcon {
    svg_path: "M10.9262 20.6762H7.17623V4.17623H10.9262V20.6762ZM17.6762 20.6762H13.9262V4.17623H17.6762V20.6762Z",
    svg_size: Size::new(24.0, 24.0),
    op: PaintOp::Fill,
};
pub static SKIP_BACK: SvgIcon = SvgIcon {
    svg_path: "M7.15139 3.42623V11.0912L19.9262 3.42623V21.4262L7.15139 13.7612V21.4262H4.92623V3.42623H7.15139Z",
    svg_size: Size::new(24.0, 24.0),
    op: PaintOp::Fill,
};
pub static SKIP_FORWARD: SvgIcon = SvgIcon {
    svg_path: "M17.7011 3.42623V11.0912L4.92623 3.42623V21.4262L17.7011 13.7612V21.4262H19.9262V3.42623H17.7011Z",
    svg_size: Size::new(24.0, 24.0),
    op: PaintOp::Fill,
};
pub static SAD_FACE: SvgIcon = SvgIcon {
    svg_path: "M5.42858 8.00001C5.90197 8.00001 6.28573 7.61625 6.28573 7.14286C6.28573 6.66948 5.90197 6.28572 5.42858 6.28572C4.9552 6.28572 4.57144 6.66948 4.57144 7.14286C4.57144 7.61625 4.9552 8.00001 5.42858 8.00001Z M8.00002 9.14285C9.62216 9.14285 10.9864 10.1975 11.4182 11.6368C11.4304 11.6797 11.4322 11.725 11.4237 11.7688C11.4152 11.8126 11.3965 11.8539 11.3692 11.8892C11.3419 11.9245 11.3066 11.9529 11.2664 11.9722C11.2261 11.9914 11.1818 12.0009 11.1372 12H4.86252C4.81802 12.0006 4.77398 11.9909 4.73391 11.9716C4.69385 11.9522 4.65885 11.9237 4.63173 11.8885C4.6046 11.8532 4.58609 11.8121 4.57767 11.7684C4.56925 11.7247 4.57115 11.6796 4.58323 11.6368C5.01144 10.1975 6.37609 9.14285 8.00002 9.14285Z M10.5714 8.00001C11.0448 8.00001 11.4286 7.61625 11.4286 7.14286C11.4286 6.66948 11.0448 6.28572 10.5714 6.28572C10.0981 6.28572 9.71429 6.66948 9.71429 7.14286C9.71429 7.61625 10.0981 8.00001 10.5714 8.00001Z M8.00001 1.07144C4.17347 1.07144 1.07144 4.17347 1.07144 8.00001C1.07144 11.8266 4.17347 14.9286 8.00001 14.9286C11.8266 14.9286 14.9286 11.8266 14.9286 8.00001C14.9286 4.17347 11.8266 1.07144 8.00001 1.07144ZM0.0714417 8.00001C0.0714417 3.62118 3.62118 0.0714417 8.00001 0.0714417C12.3788 0.0714417 15.9286 3.62118 15.9286 8.00001C15.9286 12.3788 12.3788 15.9286 8.00001 15.9286C3.62118 15.9286 0.0714417 12.3788 0.0714417 8.00001Z",
    svg_size: Size::new(16.0, 16.0),
    op: PaintOp::Fill,
};

#[derive(Copy, Clone)]
pub enum PaintOp {
    Fill,
    Stroke { width: f64 },
}

pub struct SvgIcon {
    svg_path: &'static str,
    svg_size: Size,
    op: PaintOp,
}

impl SvgIcon {
    pub fn scale(&self, to_size: impl Into<Size>) -> Icon {
        let to_size = to_size.into();
        let bez_path = BezPath::from_svg(self.svg_path).expect("Failed to parse SVG");
        let scale = Affine::scale_non_uniform(
            to_size.width / self.svg_size.width,
            to_size.height / self.svg_size.height,
        );
        Icon::new(self.op, bez_path, to_size, scale)
    }
}

pub struct Icon {
    op: PaintOp,
    bez_path: BezPath,
    size: Size,
    scale: Affine,
    color: KeyOrValue<Color>,
}

impl Icon {
    pub fn new(op: PaintOp, bez_path: BezPath, size: Size, scale: Affine) -> Self {
        Icon {
            op,
            bez_path,
            size,
            scale,
            color: theme::ICON_COLOR.into(),
        }
    }

    pub fn with_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.set_color(color);
        self
    }

    pub fn set_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        self.color = color.into();
    }
}

impl<T> Widget<T> for Icon {
    fn event(&mut self, _ctx: &mut EventCtx, _ev: &Event, _data: &mut T, _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _ev: &LifeCycle, _data: &T, _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        bc.constrain(self.size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let color = self.color.resolve(env);
        ctx.with_save(|ctx| {
            ctx.transform(self.scale);
            match self.op {
                PaintOp::Fill => ctx.fill(&self.bez_path, &color),
                PaintOp::Stroke { width } => ctx.stroke(&self.bez_path, &color, width),
            }
        });
    }
}
