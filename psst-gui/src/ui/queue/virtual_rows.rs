//! Keep widgets and artwork requests proportional to the viewport, not queue length.
use super::{page_range, row, QueueRow};
use crate::cmd;
use crate::data::AppState;
use druid::{kurbo::Line, widget::prelude::*, Point, Rect, WidgetExt, WidgetPod};
use psst_core::item_id::ItemId;

const HEIGHT: f64 = 64.0;
const OVERSCAN: usize = 2;

fn visible_range(clip: Rect, len: usize) -> std::ops::Range<usize> {
    let start = ((clip.y0.max(0.0) / HEIGHT).floor() as usize)
        .saturating_sub(OVERSCAN)
        .min(len);
    let end = ((clip.y1.max(0.0) / HEIGHT).ceil() as usize)
        .saturating_add(OVERSCAN)
        .min(len);
    start..end.max(start).min(start.saturating_add(96))
}

pub(super) struct VirtualQueue {
    clip: Rect,
    children: Vec<(usize, WidgetPod<QueueRow, Box<dyn Widget<QueueRow>>>)>,
    drag: Option<(usize, usize, Point, bool, Vec<ItemId>)>,
}

impl Default for VirtualQueue {
    fn default() -> Self {
        Self {
            clip: Rect::new(0.0, 0.0, 1000.0, 800.0),
            children: Vec::new(),
            drag: None,
        }
    }
}

impl VirtualQueue {
    fn data(data: &AppState, index: usize) -> Option<QueueRow> {
        let range = page_range(
            data.playback.up_next.len(),
            data.queue_page,
            data.config.tracks_per_page,
        );
        if !range.contains(&index) {
            return None;
        }
        Some(QueueRow {
            entry: data.playback.up_next.get(index)?.clone(),
            index: Some(index),
            ctx: data.common_ctx.clone(),
        })
    }

    fn reconcile(&mut self, data: &AppState) -> bool {
        let page = page_range(
            data.playback.up_next.len(),
            data.queue_page,
            data.config.tracks_per_page,
        );
        let local = visible_range(self.clip, page.len());
        let range = page.start + local.start..page.start + local.end;
        let old_len = self.children.len();
        self.children.retain(|(index, _)| range.contains(index));
        let mut changed = old_len != self.children.len();
        for index in range {
            if !self.children.iter().any(|(existing, _)| *existing == index) {
                self.children
                    .push((index, WidgetPod::new(row(false).fix_height(HEIGHT).boxed())));
                changed = true;
            }
        }
        self.children.sort_unstable_by_key(|(index, _)| *index);
        changed
    }
}

impl Widget<AppState> for VirtualQueue {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        let page = page_range(
            data.playback.up_next.len(),
            data.queue_page,
            data.config.tracks_per_page,
        );
        match event {
            Event::MouseDown(mouse)
                if mouse.button.is_left() && ctx.is_hot() && !page.is_empty() =>
            {
                let index = page.start + (mouse.pos.y.max(0.0) / HEIGHT) as usize;
                if index < page.end {
                    self.drag = Some((
                        index,
                        index,
                        mouse.pos,
                        false,
                        data.playback.up_next.iter().map(|q| q.item.id()).collect(),
                    ));
                    ctx.set_active(true);
                    ctx.set_handled();
                    return;
                }
            }
            Event::MouseMove(mouse) if self.drag.is_some() => {
                let (_, target, start, dragging, _) = self.drag.as_mut().unwrap();
                *dragging |= mouse.pos.distance(*start) > 5.0;
                *target = (page.start + (mouse.pos.y.max(0.0) / HEIGHT) as usize)
                    .min(page.end.saturating_sub(1));
                if *dragging {
                    ctx.scroll_area_to_view(Rect::new(
                        0.0,
                        (mouse.pos.y - HEIGHT).max(0.0),
                        1.0,
                        mouse.pos.y + HEIGHT,
                    ));
                    ctx.request_paint();
                }
                ctx.set_handled();
                return;
            }
            Event::MouseUp(mouse) if mouse.button.is_left() && self.drag.is_some() => {
                let (from, to, _, dragging, expected) = self.drag.take().unwrap();
                ctx.set_active(false);
                if dragging && from != to {
                    ctx.submit_command(cmd::MOVE_UPCOMING.with((from, to, expected)));
                } else if !dragging && ctx.is_hot() {
                    let actual: Vec<_> =
                        data.playback.up_next.iter().map(|q| q.item.id()).collect();
                    if actual == expected {
                        ctx.submit_command(cmd::PLAY_UPCOMING.with((from, expected[from])));
                    }
                }
                ctx.request_paint();
                ctx.set_handled();
                return;
            }
            _ => {}
        }
        for (index, child) in &mut self.children {
            if let Some(mut row) = Self::data(data, *index) {
                if child.is_initialized() {
                    child.event(ctx, event, &mut row, env);
                }
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        if let LifeCycle::ViewContextChanged(view) = event {
            self.clip = view.clip;
        }
        if matches!(
            event,
            LifeCycle::WidgetAdded | LifeCycle::ViewContextChanged(_)
        ) && self.reconcile(data)
        {
            ctx.children_changed();
            ctx.request_layout();
        }
        for (index, child) in &mut self.children {
            if let Some(row) = Self::data(data, *index) {
                if !child.is_initialized() && !matches!(event, LifeCycle::WidgetAdded) {
                    child.lifecycle(ctx, &LifeCycle::WidgetAdded, &row, env);
                }
                child.lifecycle(ctx, event, &row, env);
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &AppState, data: &AppState, env: &Env) {
        let page_changed = page_range(
            old.playback.up_next.len(),
            old.queue_page,
            old.config.tracks_per_page,
        )
        .start
            != page_range(
                data.playback.up_next.len(),
                data.queue_page,
                data.config.tracks_per_page,
            )
            .start;
        if old.config.tracks_per_page != data.config.tracks_per_page
            || page_changed
            || !old.playback.up_next.same(&data.playback.up_next)
        {
            if self.reconcile(data) {
                ctx.children_changed();
            }
            ctx.request_layout();
            ctx.request_paint();
            if page_changed {
                ctx.scroll_area_to_view(Rect::new(0.0, 0.0, ctx.size().width, HEIGHT));
            }
        }
        for (index, child) in &mut self.children {
            if let Some(row) = Self::data(data, *index) {
                if child.is_initialized() {
                    child.update(ctx, &row, env);
                }
            }
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppState,
        env: &Env,
    ) -> Size {
        let width = bc.max().width;
        let page = page_range(
            data.playback.up_next.len(),
            data.queue_page,
            data.config.tracks_per_page,
        );
        let child_bc = BoxConstraints::tight(Size::new(width, HEIGHT));
        for (index, child) in &mut self.children {
            if let Some(row) = Self::data(data, *index) {
                child.layout(ctx, &child_bc, &row, env);
                child.set_origin(ctx, Point::new(0.0, (*index - page.start) as f64 * HEIGHT));
            }
        }
        bc.constrain(Size::new(
            width,
            page_range(
                data.playback.up_next.len(),
                data.queue_page,
                data.config.tracks_per_page,
            )
            .len() as f64
                * HEIGHT,
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
        for (index, child) in &mut self.children {
            if let Some(row) = Self::data(data, *index) {
                child.paint(ctx, &row, env);
            }
        }
        if let Some((from, to, _, true, _)) = &self.drag {
            let page = page_range(
                data.playback.up_next.len(),
                data.queue_page,
                data.config.tracks_per_page,
            );
            if page.contains(to) {
                let y = (to - page.start + usize::from(to > from)) as f64 * HEIGHT;
                let width = ctx.size().width;
                ctx.stroke(
                    Line::new(Point::new(0.0, y), Point::new(width, y)),
                    &env.get(super::theme::BLUE_200),
                    3.0,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_pages_keep_absolute_selection_indices_even_with_duplicate_tracks() {
        use crate::data::{Config, Playable, PlaybackOrigin, QueueEntry, Track};
        use std::sync::Arc;
        let mut state = AppState::default_with_config(Config::default());
        let track: Track = serde_json::from_value(serde_json::json!({
            "name":"Repeated song", "artists":[], "duration_ms":240000,
            "disc_number":1,"track_number":1,"explicit":false,"is_local":false
        }))
        .unwrap();
        let entry = QueueEntry {
            item: Playable::Track(Arc::new(track)),
            origin: PlaybackOrigin::Home,
        };
        state.playback.up_next = (0..125).map(|_| entry.clone()).collect();
        state.config.tracks_per_page = 50;
        state.queue_page = 1;
        assert_eq!(VirtualQueue::data(&state, 50).unwrap().index, Some(50));
        assert_eq!(VirtualQueue::data(&state, 99).unwrap().index, Some(99));
        assert!(VirtualQueue::data(&state, 0).is_none());
        assert!(VirtualQueue::data(&state, 100).is_none());
        state.queue_page = 2;
        assert_eq!(VirtualQueue::data(&state, 124).unwrap().index, Some(124));
        assert!(VirtualQueue::data(&state, 125).is_none());
        let mut rows = VirtualQueue::default();
        state.queue_page = 0;
        assert!(rows.reconcile(&state));
        state.queue_page = 2;
        assert!(rows.reconcile(&state));
        assert!(rows
            .children
            .iter()
            .all(|(index, _)| (100..125).contains(index)));
        state.playback.up_next = (0..17).map(|_| entry.clone()).collect();
        assert_eq!(VirtualQueue::data(&state, 16).unwrap().index, Some(16));
        assert!(VirtualQueue::data(&state, 17).is_none());
    }

    #[test]
    fn fifty_thousand_entries_keep_only_the_scrolled_viewport_alive() {
        let at_start = visible_range(Rect::new(0.0, 0.0, 400.0, 640.0), 50_000);
        let near_end = visible_range(
            Rect::new(0.0, 49_990.0 * HEIGHT, 400.0, 50_000.0 * HEIGHT),
            50_000,
        );
        assert!(at_start.len() <= 14 && near_end.len() <= 14);
        assert!(near_end.contains(&49_995));
        assert_eq!(near_end.end, 50_000);
        assert!(visible_range(Rect::new(0.0, 999_999.0, 400.0, 1_000_100.0), 0).is_empty());
    }
}
