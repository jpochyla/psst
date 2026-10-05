use std::{mem, sync::Arc};

use druid::{
    im::Vector,
    kurbo::Line,
    lens::Map,
    piet::StrokeStyle,
    widget::{
        Button, Controller, ControllerHost, Flex, Label, List, ListIter, Painter, ViewSwitcher,
    },
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, RenderContext, Selector, Size, UpdateCtx, Widget, WidgetExt, WidgetPod,
};

use crate::{
    cmd,
    data::{
        CommonCtx, FindQuery, MatchFindQuery, Playable, PlaybackOrigin, PlaybackPayload,
        PlaylistTracks, Recommendations, SavedTracks, SearchResults, ShowEpisodes, Track, WithCtx,
    },
    ui::theme,
};

use super::{
    episode,
    find::{Find, Findable},
    track,
};

#[derive(Copy, Clone)]
pub struct Display {
    pub track: track::Display,
}

pub fn list_widget<T>(display: Display) -> impl Widget<WithCtx<T>>
where
    T: PlayableIter + Data,
{
    ControllerHost::new(PagedTracks::new(display, None), PlayController)
}

pub fn list_widget_with_find<T>(
    display: Display,
    selector: Selector<Find>,
) -> impl Widget<WithCtx<T>>
where
    T: PlayableIter + Data,
{
    ControllerHost::new(PagedTracks::new(display, Some(selector)), PlayController)
}

const PAGE_SIZE: usize = 100;

#[derive(Clone, Data, Lens)]
struct TrackPage {
    rows: Vector<PlayRow<Playable>>,
    page: usize,
    total: usize,
}

fn page_bar() -> impl Widget<TrackPage> {
    Flex::row()
        .with_child(
            Button::new("Anterior")
                .on_click(|ctx, data: &mut TrackPage, _| {
                    data.page = data.page.saturating_sub(1);
                    ctx.request_update();
                })
                .disabled_if(|data, _| data.page == 0),
        )
        .with_flex_child(
            Label::dynamic(|data: &TrackPage, _| {
                format!(
                    "Página {} / {} · {} canciones",
                    data.page + 1,
                    data.total.div_ceil(PAGE_SIZE).max(1),
                    data.total
                )
            })
            .center(),
            1.0,
        )
        .with_child(
            Button::new("Siguiente")
                .on_click(|ctx, data: &mut TrackPage, _| {
                    data.page += 1;
                    ctx.request_update();
                })
                .disabled_if(|data, _| (data.page + 1) * PAGE_SIZE >= data.total),
        )
        .padding((0.0, 10.0))
}

struct PagedTracks<T: Data> {
    child: WidgetPod<TrackPage, Box<dyn Widget<TrackPage>>>,
    page: TrackPage,
    query: Option<FindQuery>,
    selector: Option<Selector<Find>>,
    last_playing: Option<Playable>,
    marker: std::marker::PhantomData<T>,
}

impl<T: Data + PlayableIter> PagedTracks<T> {
    fn new(display: Display, selector: Option<Selector<Find>>) -> Self {
        let list = List::new(move || {
            let row = playable_widget(display).controller(RevealPlaying::default());
            if let Some(selector) = selector {
                Findable::new(row, selector).boxed()
            } else {
                row.boxed()
            }
        })
        .lens(TrackPage::rows);
        Self {
            child: WidgetPod::new(
                Flex::column()
                    .with_child(page_bar())
                    .with_child(list)
                    .with_child(page_bar())
                    .boxed(),
            ),
            page: TrackPage {
                rows: Vector::new(),
                page: 0,
                total: 0,
            },
            query: None,
            selector,
            last_playing: None,
            marker: std::marker::PhantomData,
        }
    }

    fn rebuild(&mut self, data: &WithCtx<T>) {
        let origin = Arc::new(data.data.origin());
        let mut matches = Vector::new();
        data.data.for_each(|item, position| {
            let row = PlayRow {
                is_playing: data.ctx.is_playing_at(&item, &origin),
                item,
                position,
                origin: origin.clone(),
                ctx: data.ctx.clone(),
            };
            if self
                .query
                .as_ref()
                .is_none_or(|q| q.is_empty() || row.matches_query(q))
            {
                matches.push_back(row);
            }
        });
        if !self.last_playing.same(&data.ctx.now_playing) {
            if let Some(index) = matches.iter().position(|row| row.is_playing) {
                self.page.page = index / PAGE_SIZE;
            }
            self.last_playing = data.ctx.now_playing.clone();
        }
        self.page.total = matches.len();
        self.page.page = self
            .page
            .page
            .min(matches.len().saturating_sub(1) / PAGE_SIZE);
        self.page.rows = matches
            .iter()
            .skip(self.page.page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .cloned()
            .collect();
    }
}

impl<T: Data + PlayableIter> Widget<WithCtx<T>> for PagedTracks<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut WithCtx<T>, env: &Env) {
        if let (Some(selector), Event::Command(command)) = (self.selector, event) {
            if let Some(find) = command.get(selector) {
                self.query = Some(find.query.clone());
                self.page.page = 0;
                self.rebuild(data);
                ctx.request_update();
            }
        }
        let old_page = self.page.page;
        self.child.event(ctx, event, &mut self.page, env);
        if old_page != self.page.page {
            self.rebuild(data);
            ctx.request_update();
            ctx.scroll_area_to_view(druid::Rect::from_origin_size(
                Point::ORIGIN,
                Size::new(1.0, 1.0),
            ));
        }
    }
    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &WithCtx<T>,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) {
            self.rebuild(data);
        }
        self.child.lifecycle(ctx, event, &self.page, env);
    }
    fn update(&mut self, ctx: &mut UpdateCtx, _: &WithCtx<T>, data: &WithCtx<T>, env: &Env) {
        self.rebuild(data);
        self.child.update(ctx, &self.page, env);
    }
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _: &WithCtx<T>,
        env: &Env,
    ) -> Size {
        let size = self.child.layout(ctx, bc, &self.page, env);
        self.child.set_origin(ctx, Point::ORIGIN);
        size
    }
    fn paint(&mut self, ctx: &mut PaintCtx, _: &WithCtx<T>, env: &Env) {
        self.child.paint(ctx, &self.page, env);
    }
}

#[derive(Default)]
struct RevealPlaying {
    pending: bool,
}
pub(crate) const REVEAL_PLAYING: Selector = Selector::new("app.playable.reveal-playing");
pub struct KeepCurrentVisible;
impl<W: Widget<crate::data::AppState>> Controller<crate::data::AppState, W> for KeepCurrentVisible {
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &crate::data::AppState,
        env: &Env,
    ) {
        child.lifecycle(ctx, event, data, env);
        if matches!(event, LifeCycle::Size(_)) {
            ctx.submit_command(REVEAL_PLAYING);
        }
    }
}
impl<W: Widget<PlayRow<Playable>>> Controller<PlayRow<Playable>, W> for RevealPlaying {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut PlayRow<Playable>,
        env: &Env,
    ) {
        if matches!(event, Event::Command(command) if command.is(REVEAL_PLAYING)) && data.is_playing
        {
            self.pending = true;
            ctx.request_anim_frame();
        }
        if matches!(event, Event::AnimFrame(_)) && self.pending {
            self.pending = false;
            if data.is_playing {
                ctx.scroll_to_view();
            }
        }
        child.event(ctx, event, data, env);
    }
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &PlayRow<Playable>,
        env: &Env,
    ) {
        child.lifecycle(ctx, event, data, env);
        if matches!(event, LifeCycle::WidgetAdded | LifeCycle::Size(_)) && data.is_playing {
            self.pending = true;
            ctx.request_anim_frame();
        }
    }
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old: &PlayRow<Playable>,
        data: &PlayRow<Playable>,
        env: &Env,
    ) {
        child.update(ctx, old, data, env);
        if !old.is_playing && data.is_playing {
            self.pending = true;
            ctx.request_anim_frame();
        }
    }
}

fn playable_widget(display: Display) -> impl Widget<PlayRow<Playable>> {
    ViewSwitcher::new(
        |row: &PlayRow<Playable>, _| (mem::discriminant(&row.item), row.item.id().to_base62()),
        move |_, row: &PlayRow<Playable>, _| match row.item.clone() {
            // TODO: Do the lenses some other way.
            Playable::Track(track) => track::playable_widget(&track, display.track)
                .lens(Map::new(
                    move |pb: &PlayRow<Playable>| pb.with(track.clone()),
                    |_, _| {
                        // Ignore mutation.
                    },
                ))
                .boxed(),
            Playable::Episode(episode) => {
                episode::playable_widget()
                    .lens(Map::new(
                        move |pb: &PlayRow<Playable>| pb.with(episode.clone()),
                        |_, _| {
                            // Ignore mutation.
                        },
                    ))
                    .boxed()
            }
        },
    )
}

pub fn is_playing_marker_widget() -> impl Widget<bool> {
    Painter::new(|ctx, is_playing, env| {
        const STYLE: StrokeStyle = StrokeStyle::new().dash_pattern(&[1.0, 2.0]);

        let y = ctx.size().height / 2.0;
        let line = Line::new((0.0, y), (ctx.size().width, y));
        let color = if *is_playing {
            env.get(theme::GREY_300)
        } else {
            env.get(theme::GREY_500)
        };
        ctx.stroke_styled(line, &color, 1.0, &STYLE);
    })
}

#[derive(Clone, Data, Lens)]
pub struct PlayRow<T> {
    pub item: T,
    pub ctx: Arc<CommonCtx>,
    pub origin: Arc<PlaybackOrigin>,
    pub position: usize,
    pub is_playing: bool,
}

impl<T> PlayRow<T> {
    fn with<U>(&self, item: U) -> PlayRow<U> {
        PlayRow {
            item,
            ctx: self.ctx.clone(),
            origin: self.origin.clone(),
            position: self.position,
            is_playing: self.is_playing,
        }
    }
}

impl MatchFindQuery for PlayRow<Playable> {
    fn find_result_key(&self) -> usize {
        self.position
    }
    fn matches_query(&self, q: &FindQuery) -> bool {
        match &self.item {
            Playable::Track(track) => {
                q.matches_str(&track.name)
                    || track.album.iter().any(|a| q.matches_str(&a.name))
                    || track.artists.iter().any(|a| q.matches_str(&a.name))
            }
            Playable::Episode(episode) => {
                q.matches_str(&episode.name)
                    || q.matches_str(&episode.description)
                    || q.matches_str(&episode.show.name)
            }
        }
    }
}

pub trait PlayableIter {
    fn origin(&self) -> PlaybackOrigin;
    fn count(&self) -> usize;
    fn for_each(&self, cb: impl FnMut(Playable, usize));
}

// This should change to a more specific name as it could be confusing for others
// As at the moment this is only used for the home page!
impl PlayableIter for Vector<Arc<Track>> {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Home
    }

    fn for_each(&self, mut cb: impl FnMut(Playable, usize)) {
        for (position, track) in self.iter().enumerate() {
            cb(Playable::Track(track.to_owned()), position);
        }
    }

    fn count(&self) -> usize {
        self.len()
    }
}

impl PlayableIter for PlaylistTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Playlist(self.link())
    }

    fn for_each(&self, mut cb: impl FnMut(Playable, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(Playable::Track(track.to_owned()), position);
        }
    }

    fn count(&self) -> usize {
        self.tracks.len()
    }
}

impl PlayableIter for SavedTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Library
    }

    fn for_each(&self, mut cb: impl FnMut(Playable, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(Playable::Track(track.to_owned()), position);
        }
    }

    fn count(&self) -> usize {
        self.tracks.len()
    }
}

impl PlayableIter for SearchResults {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Search(self.query.clone())
    }

    fn for_each(&self, mut cb: impl FnMut(Playable, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(Playable::Track(track.to_owned()), position);
        }
    }

    fn count(&self) -> usize {
        self.tracks.len()
    }
}

impl PlayableIter for Recommendations {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Recommendations(self.request.clone())
    }

    fn for_each(&self, mut cb: impl FnMut(Playable, usize)) {
        for (position, track) in self.tracks.iter().enumerate() {
            cb(Playable::Track(track.to_owned()), position);
        }
    }

    fn count(&self) -> usize {
        self.tracks.len()
    }
}

impl PlayableIter for ShowEpisodes {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Show(self.show.clone())
    }

    fn for_each(&self, mut cb: impl FnMut(Playable, usize)) {
        for (position, episode) in self.episodes.iter().enumerate() {
            cb(Playable::Episode(episode.to_owned()), position);
        }
    }

    fn count(&self) -> usize {
        self.episodes.len()
    }
}

impl<T> ListIter<PlayRow<Playable>> for WithCtx<T>
where
    T: PlayableIter + Data,
{
    fn for_each(&self, mut cb: impl FnMut(&PlayRow<Playable>, usize)) {
        let origin = Arc::new(self.data.origin());
        self.data.for_each(|item, position| {
            cb(
                &PlayRow {
                    is_playing: self.ctx.is_playing_at(&item, &origin),
                    ctx: self.ctx.to_owned(),
                    origin: origin.clone(),
                    item,
                    position,
                },
                position,
            )
        });
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut PlayRow<Playable>, usize)) {
        let origin = Arc::new(self.data.origin());
        self.data.for_each(|item, position| {
            cb(
                &mut PlayRow {
                    is_playing: self.ctx.is_playing_at(&item, &origin),
                    ctx: self.ctx.to_owned(),
                    origin: origin.clone(),
                    item,
                    position,
                },
                position,
            )
        });
    }

    fn data_len(&self) -> usize {
        self.data.count()
    }
}

struct PlayController;

#[cfg(test)]
mod paging_tests {
    use super::*;
    use crate::data::{AppState, Config, PlaylistLink, TrackId};
    use psst_core::item_id::{ItemId, ItemIdType};

    fn tracks(count: usize) -> Vector<Arc<Track>> {
        (0..count)
            .map(|index| {
                let mut track: Track = serde_json::from_value(serde_json::json!({
                    "id":"5lfWrciYtohtIMVDVZd0Rf", "name":format!("Song {index}"),
                    "artists":[{"id":"example","name":"Artist"}], "duration_ms":240000,
                    "disc_number":1,"track_number":1,"explicit":false,"is_local":false
                }))
                .unwrap();
                track.id = TrackId(ItemId::new(index as u128 + 1, ItemIdType::Track));
                Arc::new(track)
            })
            .collect()
    }
    fn page() -> PagedTracks<Vector<Arc<Track>>> {
        PagedTracks::new(
            Display {
                track: track::Display::empty(),
            },
            None,
        )
    }
    #[test]
    fn pages_and_search_keep_original_playback_positions() {
        let state = AppState::default_with_config(Config::default());
        let source = WithCtx {
            ctx: state.common_ctx.clone(),
            data: tracks(750),
        };
        let mut widget = page();
        widget.page.page = 2;
        widget.rebuild(&source);
        assert_eq!(widget.page.rows.len(), 100);
        assert_eq!(widget.page.rows.front().unwrap().position, 200);
        assert_eq!(source.data.count(), 750);
        widget.query = Some(FindQuery::new("Song 733"));
        widget.rebuild(&source);
        assert_eq!(widget.page.rows.len(), 1);
        assert_eq!(widget.page.rows.front().unwrap().position, 733);
        widget.query = Some(FindQuery::new("no such song"));
        widget.rebuild(&source);
        assert_eq!(widget.page.page, 0);
        assert!(widget.page.rows.is_empty());
    }
    #[test]
    fn now_playing_reveals_its_page_and_matches_only_its_source() {
        let mut state = AppState::default_with_config(Config::default());
        let tracks = tracks(750);
        state.start_playback(
            Playable::Track(tracks[612].clone()),
            PlaybackOrigin::Home,
            std::time::Duration::ZERO,
        );
        let source = WithCtx {
            ctx: state.common_ctx.clone(),
            data: tracks,
        };
        let mut widget = page();
        widget.rebuild(&source);
        assert_eq!(widget.page.page, 6);
        assert!(widget.page.rows[12].is_playing);
        let another = PlaybackOrigin::Playlist(PlaylistLink {
            id: "other".into(),
            name: "Other".into(),
        });
        assert!(!source
            .ctx
            .is_playing_at(&widget.page.rows[12].item, &another));
        // Independent metadata objects from a restored snapshot still match by Spotify ID.
        let same = Playable::Track(Arc::new(source.data[612].as_ref().clone()));
        assert!(source.ctx.is_playing_at(&same, &PlaybackOrigin::Home));
    }
}

impl<T, W> Controller<WithCtx<T>, W> for PlayController
where
    T: PlayableIter + Data,
    W: Widget<WithCtx<T>>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut WithCtx<T>,
        env: &Env,
    ) {
        match event {
            Event::Notification(note) => {
                if let Some(position) = note.get(cmd::PLAY) {
                    let mut items = Vector::new();
                    data.data.for_each(|item, _| items.push_back(item));
                    let payload = PlaybackPayload {
                        items,
                        origin: data.data.origin(),
                        position: position.to_owned(),
                    };
                    ctx.submit_command(cmd::PLAY_TRACKS.with(payload));
                    ctx.set_handled();
                }
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
