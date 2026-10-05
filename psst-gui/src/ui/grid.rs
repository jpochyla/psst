use crate::data::WithCtx;
use druid::{
    im::Vector,
    lens::Map,
    widget::{Button, Flex, Label, List},
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Size, UpdateCtx, Widget, WidgetExt, WidgetPod,
};

const PAGE_SIZE: usize = 40;
#[derive(Clone, Data, Lens)]
struct GridPage<T: Data> {
    rows: Vector<Vector<T>>,
    values: Vector<T>,
    page_size: usize,
    page: usize,
    total: usize,
}

pub fn with_context<T: Data>(
    factory: impl Fn() -> Box<dyn Widget<WithCtx<T>>> + Clone + 'static,
) -> impl Widget<WithCtx<Vector<T>>> {
    widget(factory).lens(Map::new(
        |data: &WithCtx<Vector<T>>| {
            data.data
                .iter()
                .map(|item| WithCtx {
                    ctx: data.ctx.clone(),
                    data: item.clone(),
                })
                .collect()
        },
        |_, _| {},
    ))
}

pub fn widget<T: Data>(
    factory: impl Fn() -> Box<dyn Widget<T>> + Clone + 'static,
) -> impl Widget<Vector<T>> {
    let toolbar = toolbar();
    let rows = List::new(move || {
        let factory = factory.clone();
        List::new(move || factory().fix_width(152.0))
            .horizontal()
            .with_spacing(8.0)
    })
    .with_spacing(8.0)
    .lens(GridPage::<T>::rows);
    Grid {
        child: WidgetPod::new(Flex::column().with_child(toolbar).with_child(rows).boxed()),
        data: GridPage {
            rows: Vector::new(),
            values: Vector::new(),
            page_size: PAGE_SIZE,
            page: 0,
            total: 0,
        },
    }
}

fn toolbar<T: Data>() -> impl Widget<GridPage<T>> {
    Flex::row()
        .with_child(
            Button::new("Anterior")
                .on_click(|ctx, data: &mut GridPage<T>, _| {
                    data.page = data.page.saturating_sub(1);
                    ctx.request_update();
                })
                .disabled_if(|data, _| data.page == 0),
        )
        .with_flex_child(
            Label::dynamic(|data: &GridPage<T>, _| {
                format!(
                    "{} / {}",
                    data.page + 1,
                    data.total.div_ceil(data.page_size).max(1)
                )
            })
            .center(),
            1.0,
        )
        .with_child(
            Button::new("Siguiente")
                .on_click(|ctx, data: &mut GridPage<T>, _| {
                    data.page += 1;
                    ctx.request_update();
                })
                .disabled_if(|data, _| (data.page + 1) * data.page_size >= data.total),
        )
        .padding((8.0, 10.0))
}

pub fn list<T: Data>(factory: impl Fn() -> Box<dyn Widget<T>> + 'static) -> impl Widget<Vector<T>> {
    Grid {
        child: WidgetPod::new(
            Flex::column()
                .with_child(toolbar())
                .with_child(List::new(factory).lens(GridPage::<T>::values))
                .boxed(),
        ),
        data: GridPage {
            rows: Vector::new(),
            values: Vector::new(),
            page_size: 100,
            page: 0,
            total: 0,
        },
    }
}

struct Grid<T: Data> {
    child: WidgetPod<GridPage<T>, Box<dyn Widget<GridPage<T>>>>,
    data: GridPage<T>,
}
impl<T: Data> Grid<T> {
    fn rebuild(&mut self, source: &Vector<T>) {
        self.data.total = source.len();
        self.data.page = self
            .data
            .page
            .min(source.len().saturating_sub(1) / self.data.page_size);
        let values: Vec<_> = source
            .iter()
            .skip(self.data.page * self.data.page_size)
            .take(self.data.page_size)
            .cloned()
            .collect();
        self.data.values = values.iter().cloned().collect();
        self.data.rows = values
            .chunks(2)
            .map(|row| row.iter().cloned().collect())
            .collect();
    }
}
impl<T: Data> Widget<Vector<T>> for Grid<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut Vector<T>, env: &Env) {
        self.child.event(ctx, event, &mut self.data, env);
    }
    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Vector<T>,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) {
            self.rebuild(data);
        }
        self.child.lifecycle(ctx, event, &self.data, env);
    }
    fn update(&mut self, ctx: &mut UpdateCtx, _: &Vector<T>, data: &Vector<T>, env: &Env) {
        self.rebuild(data);
        self.child.update(ctx, &self.data, env);
    }
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _: &Vector<T>,
        env: &Env,
    ) -> Size {
        let size = self.child.layout(ctx, bc, &self.data, env);
        self.child.set_origin(ctx, Point::ORIGIN);
        size
    }
    fn paint(&mut self, ctx: &mut PaintCtx, _: &Vector<T>, env: &Env) {
        self.child.paint(ctx, &self.data, env);
    }
}
