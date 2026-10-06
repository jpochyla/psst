use crate::{
    core::{Error, SpotifyId, SpotifyUri},
    protocol::{
        context::Context,
        context_page::ContextPage,
        context_track::ContextTrack,
        player::{ContextIndex, ProvidedTrack},
        restrictions::Restrictions,
    },
    shuffle_vec::ShuffleVec,
    state::{
        ConnectState, SPOTIFY_MAX_NEXT_TRACKS_SIZE, StateError,
        metadata::Metadata,
        provider::{IsProvider, Provider},
    },
};
use protobuf::MessageField;
use std::collections::HashMap;
use uuid::Uuid;

const LOCAL_FILES_IDENTIFIER: &str = "spotify:local-files";
const SEARCH_IDENTIFIER: &str = "spotify:search";

#[derive(Debug)]
pub struct StateContext {
    pub tracks: ShuffleVec<ProvidedTrack>,
    pub metadata: HashMap<String, String>,
    pub restrictions: Option<Restrictions>,
    /// is used to keep track which tracks are already loaded into the next_tracks
    pub index: ContextIndex,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum ContextType {
    #[default]
    Default,
    Autoplay,
}

pub enum ResetContext<'s> {
    Completely,
    DefaultIndex,
    WhenDifferent(&'s str),
}

/// Extracts the spotify uri from a given page_url
///
/// Just extracts "spotify/album/5LFzwirfFwBKXJQGfwmiMY" and replaces the slash's with colon's
///
/// Expected `page_url` should look something like the following:
/// `hm://artistplaycontext/v1/page/spotify/album/5LFzwirfFwBKXJQGfwmiMY/km_artist`
fn page_url_to_uri(page_url: &str) -> String {
    let split = if let Some(rest) = page_url.strip_prefix("hm://") {
        rest.split('/')
    } else {
        warn!("page_url didn't start with hm://. got page_url: {page_url}");
        page_url.split('/')
    };

    split
        .skip_while(|s| s != &"spotify")
        .take(3)
        .collect::<Vec<&str>>()
        .join(":")
}

impl ConnectState {
    pub fn find_index_in_context<F: Fn(&ProvidedTrack) -> bool>(
        ctx: &StateContext,
        f: F,
    ) -> Result<usize, StateError> {
        ctx.tracks
            .iter()
            .position(f)
            .ok_or(StateError::CanNotFindTrackInContext(None, ctx.tracks.len()))
    }

    pub fn get_context(&self, ty: ContextType) -> Result<&StateContext, StateError> {
        match ty {
            ContextType::Default => self.context.as_ref(),
            ContextType::Autoplay => self.autoplay_context.as_ref(),
        }
        .ok_or(StateError::NoContext(ty))
    }

    pub fn get_context_mut(&mut self, ty: ContextType) -> Result<&mut StateContext, StateError> {
        match ty {
            ContextType::Default => self.context.as_mut(),
            ContextType::Autoplay => self.autoplay_context.as_mut(),
        }
        .ok_or(StateError::NoContext(ty))
    }

    pub fn context_uri(&self) -> &String {
        &self.player().context_uri
    }

    fn different_context_uri(&self, uri: &str) -> bool {
        // search identifier is always different
        self.context_uri() != uri || uri.starts_with(SEARCH_IDENTIFIER)
    }

    pub fn reset_context(&mut self, mut reset_as: ResetContext) {
        if matches!(reset_as, ResetContext::WhenDifferent(ctx) if self.different_context_uri(ctx)) {
            reset_as = ResetContext::Completely
        }

        if let Ok(ctx) = self.get_context_mut(ContextType::Default) {
            ctx.remove_shuffle_seed();
            ctx.remove_initial_track();
            ctx.tracks.unshuffle()
        }

        match reset_as {
            ResetContext::WhenDifferent(_) => debug!("context didn't change, no reset"),
            ResetContext::Completely => {
                self.context = None;
                self.autoplay_context = None;

                let player = self.player_mut();
                player.context_uri.clear();
                player.context_url.clear();
            }
            ResetContext::DefaultIndex => {
                for ctx in [self.context.as_mut(), self.autoplay_context.as_mut()]
                    .into_iter()
                    .flatten()
                {
                    ctx.index.track = 0;
                    ctx.index.page = 0;
                }
            }
        }

        self.fill_up_context = ContextType::Default;
        self.set_active_context(ContextType::Default);
        self.update_restrictions()
    }

    pub fn valid_resolve_uri(uri: &str) -> Option<&str> {
        if uri.is_empty() || uri.starts_with(SEARCH_IDENTIFIER) {
            None
        } else {
            Some(uri)
        }
    }

    pub fn find_valid_uri<'s>(
        context_uri: Option<&'s str>,
        first_page: Option<&'s ContextPage>,
    ) -> Option<&'s str> {
        context_uri
            .and_then(Self::valid_resolve_uri)
            .or_else(|| first_page.and_then(|p| p.tracks.first().and_then(|t| t.uri.as_deref())))
    }

    pub fn set_active_context(&mut self, new_context: ContextType) {
        self.active_context = new_context;

        let player = self.player_mut();

        player.context_metadata = Default::default();
        player.context_restrictions = MessageField::some(Default::default());
        player.restrictions = MessageField::some(Default::default());

        let ctx = match self.get_context(new_context) {
            Err(why) => {
                warn!("couldn't load context info because: {why}");
                return;
            }
            Ok(ctx) => ctx,
        };

        let mut restrictions = ctx.restrictions.clone();
        let metadata = ctx.metadata.clone();

        let player = self.player_mut();

        if let Some(restrictions) = restrictions.take() {
            player.restrictions = MessageField::some(restrictions.into());
        }

        for (key, value) in metadata {
            player.context_metadata.insert(key, value);
        }
    }

    pub fn update_context(
        &mut self,
        mut context: Context,
        ty: ContextType,
    ) -> Result<Option<Vec<String>>, Error> {
        if context.pages.iter().all(|p| p.tracks.is_empty()) {
            error!("context didn't have any tracks: {context:#?}");
            Err(StateError::ContextHasNoTracks)?;
        } else if matches!(context.uri, Some(ref uri) if uri.starts_with(LOCAL_FILES_IDENTIFIER)) {
            Err(StateError::UnsupportedLocalPlayback)?;
        }

        let mut next_contexts = Vec::new();
        let mut first_page = None;
        for page in context.pages {
            if first_page.is_none() && !page.tracks.is_empty() {
                first_page = Some(page);
            } else {
                next_contexts.push(page)
            }
        }

        let page = match first_page {
            None => Err(StateError::ContextHasNoTracks)?,
            Some(p) => p,
        };

        debug!(
            "updated context {ty:?} to <{:?}> ({} tracks)",
            context.uri,
            page.tracks.len()
        );

        match ty {
            ContextType::Default => {
                let mut new_context = self.state_context_from_page(
                    page,
                    context.metadata,
                    context.restrictions.take(),
                    context.uri.as_deref(),
                    Some(0),
                    None,
                );

                // when we update the same context, we should try to preserve the previous position
                // otherwise we might load the entire context twice, unless it's the search context
                if !self.context_uri().starts_with(SEARCH_IDENTIFIER)
                    && matches!(context.uri, Some(ref uri) if uri == self.context_uri())
                {
                    if let Some(new_index) = self.find_last_index_in_new_context(&new_context) {
                        new_context.index.track = match new_index {
                            Ok(i) => i,
                            Err(i) => {
                                self.player_mut().index = MessageField::none();
                                i
                            }
                        };

                        // enforce reloading the context
                        if let Ok(autoplay_ctx) = self.get_context_mut(ContextType::Autoplay) {
                            autoplay_ctx.index.track = 0
                        }
                        self.clear_next_tracks();
                    }
                }

                self.context = Some(new_context);

                if !matches!(context.url, Some(ref url) if url.contains(SEARCH_IDENTIFIER)) {
                    self.player_mut().context_url = context.url.take().unwrap_or_default();
                } else {
                    self.player_mut().context_url.clear()
                }
                self.player_mut().context_uri = context.uri.take().unwrap_or_default();
            }
            ContextType::Autoplay => {
                self.autoplay_context = Some(self.state_context_from_page(
                    page,
                    context.metadata,
                    context.restrictions.take(),
                    context.uri.as_deref(),
                    None,
                    Some(Provider::Autoplay),
                ))
            }
        }

        if next_contexts.is_empty() {
            return Ok(None);
        }

        // load remaining contexts
        let next_contexts = next_contexts
            .into_iter()
            .flat_map(|page| {
                if !page.tracks.is_empty() {
                    self.fill_context_from_page(page).ok()?;
                    None
                } else if matches!(page.page_url, Some(ref url) if !url.is_empty()) {
                    Some(page_url_to_uri(
                        &page.page_url.expect("checked by precondition"),
                    ))
                } else {
                    warn!("unhandled context page: {page:#?}");
                    None
                }
            })
            .collect();

        Ok(Some(next_contexts))
    }

    fn find_first_prev_track_index(&self, ctx: &StateContext) -> Option<usize> {
        let prev_tracks = self.prev_tracks();
        for i in (0..prev_tracks.len()).rev() {
            let prev_track = prev_tracks.get(i)?;
            if let Ok(idx) = Self::find_index_in_context(ctx, |t| prev_track.uri == t.uri) {
                return Some(idx);
            }
        }
        None
    }

    fn find_last_index_in_new_context(
        &self,
        new_context: &StateContext,
    ) -> Option<Result<u32, u32>> {
        let ctx = self.context.as_ref()?;

        let is_queued_item = self.current_track(|t| t.is_queue() || t.is_from_queue());

        let new_index = if ctx.index.track as usize >= SPOTIFY_MAX_NEXT_TRACKS_SIZE {
            Some(ctx.index.track as usize - SPOTIFY_MAX_NEXT_TRACKS_SIZE)
        } else if is_queued_item {
            self.find_first_prev_track_index(new_context)
        } else {
            Self::find_index_in_context(new_context, |current| {
                self.current_track(|t| t.uri == current.uri)
            })
            .ok()
        }
        .map(|i| i as u32 + 1);

        Some(new_index.ok_or_else(|| {
            info!(
                "couldn't distinguish index from current or previous tracks in the updated context"
            );
            let fallback_index = self
                .player()
                .index
                .as_ref()
                .map(|i| i.track)
                .unwrap_or_default();
            info!("falling back to index {fallback_index}");
            fallback_index
        }))
    }

    fn state_context_from_page(
        &mut self,
        page: ContextPage,
        metadata: HashMap<String, String>,
        restrictions: Option<Restrictions>,
        new_context_uri: Option<&str>,
        context_length: Option<usize>,
        provider: Option<Provider>,
    ) -> StateContext {
        let new_context_uri = new_context_uri.unwrap_or(self.context_uri());

        let tracks = page
            .tracks
            .iter()
            .enumerate()
            .flat_map(|(i, track)| {
                match self.context_to_provided_track(
                    track,
                    Some(new_context_uri),
                    context_length.map(|l| l + i),
                    Some(&page.metadata),
                    provider.clone(),
                ) {
                    Ok(t) => Some(t),
                    Err(why) => {
                        error!("couldn't convert {track:#?} into ProvidedTrack: {why}");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        StateContext {
            tracks: tracks.into(),
            restrictions,
            metadata,
            index: ContextIndex::new(),
        }
    }

    pub fn is_skip_track(&self, track: &ProvidedTrack, iteration: Option<u32>) -> bool {
        let ctx = match self.get_context(self.active_context).ok() {
            None => return false,
            Some(ctx) => ctx,
        };

        if ctx.get_initial_track().is_none_or(|uri| uri != &track.uri) {
            return false;
        }

        iteration.is_none_or(|i| i == 0)
    }

    pub fn merge_context(&mut self, new_page: Option<ContextPage>) -> Option<()> {
        let current_context = self.get_context_mut(ContextType::Default).ok()?;

        for new_track in new_page?.tracks {
            if new_track.uri.is_none() || matches!(new_track.uri, Some(ref uri) if uri.is_empty()) {
                continue;
            }

            let new_track_uri = new_track.uri.unwrap_or_default();
            if let Ok(position) =
                Self::find_index_in_context(current_context, |t| t.uri == new_track_uri)
            {
                let context_track = current_context.tracks.get_mut(position)?;

                for (key, value) in new_track.metadata {
                    context_track.metadata.insert(key, value);
                }

                // the uid provided from another context might be actual uid of an item
                if new_track.uid.is_some()
                    || matches!(new_track.uid, Some(ref uid) if uid.is_empty())
                {
                    context_track.uid = new_track.uid.unwrap_or_default();
                }
            }
        }

        Some(())
    }

    pub(super) fn update_context_index(
        &mut self,
        ty: ContextType,
        new_index: usize,
    ) -> Result<(), StateError> {
        let context = self.get_context_mut(ty)?;

        context.index.track = new_index as u32;
        Ok(())
    }

    pub fn context_to_provided_track(
        &self,
        ctx_track: &ContextTrack,
        context_uri: Option<&str>,
        context_index: Option<usize>,
        page_metadata: Option<&HashMap<String, String>>,
        provider: Option<Provider>,
    ) -> Result<ProvidedTrack, Error> {
        let id = match (ctx_track.uri.as_ref(), ctx_track.gid.as_ref()) {
            (Some(uri), _) if uri.contains(['?']) => {
                Err(StateError::InvalidTrackUri(Some(uri.clone())))?
            }
            (Some(uri), _) if !uri.is_empty() => SpotifyUri::from_uri(uri)?,
            (_, Some(gid)) if !gid.is_empty() => SpotifyUri::Track {
                id: SpotifyId::from_raw(gid)?,
            },
            _ => Err(StateError::InvalidTrackUri(None))?,
        };

        let uri = id.to_uri()?.replace("unknown", "track");

        let provider = if self.unavailable_uri.contains(&uri) {
            Provider::Unavailable
        } else {
            provider.unwrap_or(Provider::Context)
        };

        // assumption: the uid is used as unique-id of any item
        //  - queue resorting is done by each client and orients itself by the given uid
        //  - if no uid is present, resorting doesn't work or behaves not as intended
        let uid = match ctx_track.uid.as_ref() {
            Some(uid) if !uid.is_empty() => uid.to_string(),
            // so providing a unique id should allow to resort the queue
            _ => Uuid::new_v4().as_simple().to_string(),
        };

        let mut metadata = page_metadata.cloned().unwrap_or_default();
        for (k, v) in &ctx_track.metadata {
            metadata.insert(k.to_string(), v.to_string());
        }

        let mut track = ProvidedTrack {
            uri,
            uid,
            metadata,
            provider: provider.to_string(),
            ..Default::default()
        };

        if let Some(context_uri) = context_uri {
            track.set_entity_uri(context_uri);
            track.set_context_uri(context_uri);
        }

        if let Some(index) = context_index {
            track.set_context_index(index);
        }

        if matches!(provider, Provider::Autoplay) {
            track.set_from_autoplay(true)
        }

        Ok(track)
    }

    pub fn fill_context_from_page(&mut self, page: ContextPage) -> Result<(), Error> {
        let ctx_len = self.context.as_ref().map(|c| c.tracks.len());
        let context = self.state_context_from_page(page, HashMap::new(), None, None, ctx_len, None);

        let ctx = self
            .context
            .as_mut()
            .ok_or(StateError::NoContext(ContextType::Default))?;

        for t in context.tracks {
            ctx.tracks.push(t)
        }

        Ok(())
    }
}
