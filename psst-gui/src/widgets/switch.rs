use druid::{widget::prelude::*, Data, WidgetPod};
use std::{collections::HashMap, hash::Hash};

type ChildPicker<T, U> = dyn Fn(&T, &Env) -> U;
type ChildBuilder<T, U> = dyn Fn(&U, &T, &Env) -> Box<dyn Widget<T>>;

pub struct ViewDispatcher<T, U> {
    child_picker: Box<ChildPicker<T, U>>,
    child_builder: Box<ChildBuilder<T, U>>,
    children: HashMap<U, WidgetPod<T, Box<dyn Widget<T>>>>,
    active_child_id: Option<U>,
}

impl<T: Data, U: Data + Eq + Hash> ViewDispatcher<T, U> {
    pub fn new(
        child_picker: impl Fn(&T, &Env) -> U + 'static,
        child_builder: impl Fn(&U, &T, &Env) -> Box<dyn Widget<T>> + 'static,
    ) -> Self {
        Self {
            child_picker: Box::new(child_picker),
            child_builder: Box::new(child_builder),
            children: HashMap::new(),
            active_child_id: None,
        }
    }

    fn active_child(&mut self) -> Option<&mut WidgetPod<T, Box<dyn Widget<T>>>> {
        if let Some(id) = self.active_child_id.as_ref() {
            self.children.get_mut(id)
        } else {
            None
        }
    }
}

impl<T: Data, U: Data + Eq + Hash> Widget<T> for ViewDispatcher<T, U> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(child) = self.active_child() {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            let child_id = (self.child_picker)(data, env);
            let child = (self.child_builder)(&child_id, data, env);
            self.children
                .insert(child_id.clone(), WidgetPod::new(child));
            self.active_child_id = Some(child_id);
        }
        if let Some(child) = self.active_child() {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let child_id = (self.child_picker)(data, env);
        // Safe to unwrap because self.active_child_id should not be empty
        if !child_id.same(self.active_child_id.as_ref().unwrap()) {
            if !self.children.contains_key(&child_id) {
                let child = (self.child_builder)(&child_id, data, env);
                self.children
                    .insert(child_id.clone(), WidgetPod::new(child));
            }
            self.active_child_id = Some(child_id);
            ctx.children_changed();
        // Because the new child has not yet been initialized, we have to skip
        // the update after switching.
        } else if let Some(child) = self.active_child() {
            child.update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        match self.active_child() {
            Some(child) => {
                let size = child.layout(ctx, bc, data, env);
                child.set_layout_rect(ctx, data, env, size.to_rect());
                size
            }
            None => bc.max(),
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(ref mut child) = self.active_child() {
            child.paint_raw(ctx, data, env);
        }
    }
}
