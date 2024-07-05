// psst-gui/src/data/view.rs
#[derive(Clone, Data, Lens)]
pub struct View {
    pub id: Arc<str>,
    pub title: Arc<str>,
    pub subtitle: Option<Arc<str>>,
    pub items: Vector<ViewItem>,
}

#[derive(Clone, Data)]
pub enum ViewItem {
    Playlist(Arc<Playlist>),
    Album(Arc<Album>),
    Artist(Arc<Artist>),
    Show(Arc<Show>),
}