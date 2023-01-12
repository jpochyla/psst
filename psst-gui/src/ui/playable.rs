use std::{mem, sync::Arc};

use druid::{
    im::Vector,
    kurbo::Line,
    lens::Map,
    piet::StrokeStyle,
    widget::{Controller, ControllerHost, List, ListIter, Painter, ViewSwitcher},
    Data, Env, Event, EventCtx, Lens, RenderContext, Selector, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        Album, ArtistTracks, CommonCtx, FindQuery, MatchFindQuery, Playable, PlaybackOrigin,
        PlaybackPayload, PlaylistTracks, Recommendations, SavedTracks, SearchResults, ShowEpisodes,
        WithCtx,
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
    ControllerHost::new(List::new(move || playable_widget(display)), PlayController)
}

pub fn list_widget_with_find<T>(
    display: Display,
    selector: Selector<Find>,
) -> impl Widget<WithCtx<T>>
where
    T: PlayableIter + Data,
{
    ControllerHost::new(
        List::new(move || Findable::new(playable_widget(display), selector)),
        PlayController,
    )
}

fn playable_widget(display: Display) -> impl Widget<PlayRow<Playable>> {
    ViewSwitcher::new(
        |row: &PlayRow<Playable>, _| mem::discriminant(&row.item),
        move |_, row: &PlayRow<Playable>, _| match row.item.clone() {
            // TODO: Do the lenses some other way.
            Playable::Track(track) => track::playable_widget(display.track)
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

        let line = Line::new((0.0, 0.0), (ctx.size().width, 0.0));
        let color = if *is_playing {
            env.get(theme::GREY_200)
        } else {
            env.get(theme::GREY_500)
        };
        ctx.stroke_styled(line, &color, 1.0, &STYLE);
    })
    .fix_height(1.0)
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

impl PlayableIter for Arc<Album> {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Album(self.link())
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

impl PlayableIter for ArtistTracks {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Artist(self.link())
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
                    is_playing: self.ctx.is_playing(&item),
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
                    is_playing: self.ctx.is_playing(&item),
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
