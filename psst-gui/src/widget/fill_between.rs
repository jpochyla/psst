use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Size, UpdateCtx, Widget, WidgetPod,
};

/// A widget that positions two children, allowing the right child to fill the remaining space.
/// The left child is measured first, and the right child is given the rest of the available width.
pub struct FillBetween<T: Data> {
    left: WidgetPod<T, Box<dyn Widget<T>>>,
    right: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> FillBetween<T> {
    pub fn new(left: impl Widget<T> + 'static, right: impl Widget<T> + 'static) -> Self {
        Self {
            left: WidgetPod::new(Box::new(left)),
            right: WidgetPod::new(Box::new(right)),
        }
    }
}

impl<T: Data> Widget<T> for FillBetween<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.right.event(ctx, event, data, env);
        self.left.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.right.lifecycle(ctx, event, data, env);
        self.left.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.left.update(ctx, data, env);
        self.right.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let max_width = bc.max().width;

        // Measure the left child with our max width constraint.
        let left_bc = BoxConstraints::new(
            Size::new(0.0, bc.min().height),
            Size::new(max_width, bc.max().height),
        );
        let left_size = self.left.layout(ctx, &left_bc, data, env);

        // Layout the right child in the remaining space.
        let right_width = (max_width - left_size.width).max(0.0);
        let right_bc = BoxConstraints::tight(Size::new(right_width, left_size.height));
        let right_size = self.right.layout(ctx, &right_bc, data, env);

        // Vertically center children.
        let total_height = left_size.height.max(right_size.height);
        let left_y = (total_height - left_size.height) / 2.0;
        let right_y = (total_height - right_size.height) / 2.0;

        self.left.set_origin(ctx, Point::new(0.0, left_y));
        self.right
            .set_origin(ctx, Point::new(left_size.width, right_y));

        Size::new(max_width, total_height)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.left.paint(ctx, data, env);
        self.right.paint(ctx, data, env);
    }
}
