use std::sync::Arc;

use druid::{
    widget::{prelude::*, Image},
    Data, ImageBuf, Point, Selector, WidgetPod,
};

pub const REQUEST_DATA: Selector<Arc<str>> = Selector::new("remote-image.request-data");
pub const PROVIDE_DATA: Selector<ImagePayload> = Selector::new("remote-image.provide-data");

#[derive(Clone)]
pub struct ImagePayload {
    pub location: Arc<str>,
    pub image_buf: ImageBuf,
}

pub struct RemoteImage<T> {
    placeholder: WidgetPod<T, Box<dyn Widget<T>>>,
    image: Option<WidgetPod<T, Image>>,
    locator: Box<dyn Fn(&T, &Env) -> Option<Arc<str>>>,
    location: Option<Arc<str>>,
}

impl<T: Data> RemoteImage<T> {
    pub fn new(
        placeholder: impl Widget<T> + 'static,
        locator: impl Fn(&T, &Env) -> Option<Arc<str>> + 'static,
    ) -> Self {
        Self {
            placeholder: WidgetPod::new(placeholder).boxed(),
            locator: Box::new(locator),
            location: None,
            image: None,
        }
    }
}

impl<T: Data> Widget<T> for RemoteImage<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::Command(cmd) = event {
            if let Some(payload) = cmd.get(PROVIDE_DATA) {
                if Some(&payload.location) == self.location.as_ref() {
                    self.image
                        .replace(WidgetPod::new(Image::new(payload.image_buf.clone())));
                    ctx.children_changed();
                }
                return;
            }
        }
        if let Some(image) = self.image.as_mut() {
            image.event(ctx, event, data, env);
        } else {
            self.placeholder.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            let location = (self.locator)(data, env);
            self.image = None;
            self.location = location.clone();
            if let Some(location) = location {
                ctx.submit_command(REQUEST_DATA.with(location).to(ctx.widget_id()));
            }
        }
        if let Some(image) = self.image.as_mut() {
            image.lifecycle(ctx, event, data, env);
        } else {
            self.placeholder.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let location = (self.locator)(data, env);
        if location != self.location {
            self.image = None;
            self.location = location.clone();
            if let Some(location) = location {
                ctx.submit_command(REQUEST_DATA.with(location).to(ctx.widget_id()));
            }
            ctx.children_changed();
        }
        if let Some(image) = self.image.as_mut() {
            image.update(ctx, data, env);
        } else {
            self.placeholder.update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if let Some(image) = self.image.as_mut() {
            let size = image.layout(ctx, bc, data, env);
            image.set_origin(ctx, Point::ORIGIN);
            size
        } else {
            let size = self.placeholder.layout(ctx, bc, data, env);
            self.placeholder.set_origin(ctx, Point::ORIGIN);
            size
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(image) = self.image.as_mut() {
            image.paint(ctx, data, env)
        } else {
            self.placeholder.paint(ctx, data, env)
        }
    }
}
