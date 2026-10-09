use std::time::Duration;

use druid::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point,
    Rect, RenderContext, Size, TimerToken, UpdateCtx, Widget, WidgetPod,
};
use psst_core::audio::meter::{self, BAND_COUNT};

use crate::data::{AppState, Playback, PlaybackState};

use super::theme;

const FRAME_TIME: Duration = Duration::from_millis(33);

/// A local paint-only animation: its timer never changes application data or queue rows.
pub struct SoundShadow<W> {
    child: WidgetPod<AppState, W>,
    levels: [f32; BAND_COUNT],
    timer: TimerToken,
}

impl<W: Widget<AppState>> SoundShadow<W> {
    pub fn new(child: W) -> Self {
        Self {
            child: WidgetPod::new(child),
            levels: [0.0; BAND_COUNT],
            timer: TimerToken::INVALID,
        }
    }
}

fn audible(data: &Playback) -> bool {
    data.state == PlaybackState::Playing && data.now_playing.is_some() && data.volume > 0.0
}

fn approach(levels: &mut [f32; BAND_COUNT], target: [f32; BAND_COUNT]) -> bool {
    let mut changed = false;
    for (level, target) in levels.iter_mut().zip(target) {
        let previous = *level;
        let amount = if target > previous { 0.65 } else { 0.22 };
        *level += (target - *level) * amount;
        if *level < 0.005 {
            *level = 0.0;
        }
        changed |= (*level - previous).abs() > 0.0001;
    }
    changed
}

impl<W: Widget<AppState>> Widget<AppState> for SoundShadow<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        if let Event::Timer(token) = event {
            if *token == self.timer {
                self.timer = TimerToken::INVALID;
                let target = if audible(&data.playback) {
                    meter::levels()
                } else {
                    [0.0; BAND_COUNT]
                };
                if approach(&mut self.levels, target) {
                    ctx.request_paint();
                }
                if audible(&data.playback) || self.levels.iter().any(|level| *level > 0.0) {
                    self.timer = ctx.request_timer(FRAME_TIME);
                }
                ctx.set_handled();
                return;
            }
        }
        if matches!(event, Event::WindowConnected)
            && audible(&data.playback)
            && self.timer == TimerToken::INVALID
        {
            self.timer = ctx.request_timer(FRAME_TIME);
        }
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old: &AppState, data: &AppState, env: &Env) {
        if audible(&data.playback) && self.timer == TimerToken::INVALID {
            self.timer = ctx.request_timer(FRAME_TIME);
        }
        self.child.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppState,
        env: &Env,
    ) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        self.child.set_origin(ctx, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
        let size = ctx.size();
        let columns = ((size.width / 10.0) as usize).clamp(1, 96);
        let spacing = size.width / columns as f64;
        let rows = ((size.height - 12.0) / 8.0).clamp(0.0, 9.0) as usize;
        let color = env.get(theme::BLUE_100);
        ctx.with_save(|ctx| {
            ctx.clip(size.to_rect());
            for column in 0..columns {
                // Mirror the real bands so bass pulses also reach the volume end.
                let position = column as f64 / columns.saturating_sub(1).max(1) as f64;
                let band = (1.0 - (position * 2.0 - 1.0).abs()) * (BAND_COUNT - 1) as f64;
                let left = band.floor() as usize;
                let right = (left + 1).min(BAND_COUNT - 1);
                let fraction = band.fract() as f32;
                let level = self.levels[left] * (1.0 - fraction) + self.levels[right] * fraction;
                let height = level * rows as f32;
                for row in 0..rows {
                    let coverage = (height - row as f32).clamp(0.0, 1.0) as f64;
                    if coverage <= 0.0 {
                        break;
                    }
                    let x = column as f64 * spacing + (spacing - 6.0) * 0.5;
                    let y = size.height - 6.0 - row as f64 * 8.0;
                    let square = Rect::new(x, y - 6.0, x + 6.0, y);
                    ctx.fill(
                        square.inflate(2.0, 2.0),
                        &color.with_alpha(0.025 * coverage),
                    );
                    ctx.fill(
                        square,
                        &color
                            .with_alpha((0.10 + 0.08 * row as f64 / rows.max(1) as f64) * coverage),
                    );
                }
            }
        });
        // Controls, text and artwork remain above the shadow and keep their hit targets.
        self.child.paint(ctx, data, env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_follows_levels_and_settles_completely_after_pause() {
        let mut levels = [0.0; BAND_COUNT];
        assert!(approach(&mut levels, [1.0; BAND_COUNT]));
        assert!(levels.iter().all(|level| *level > 0.5 && *level < 1.0));
        for _ in 0..30 {
            approach(&mut levels, [0.0; BAND_COUNT]);
        }
        assert_eq!(levels, [0.0; BAND_COUNT]);
        assert!(!approach(&mut levels, [0.0; BAND_COUNT]));
    }
}
