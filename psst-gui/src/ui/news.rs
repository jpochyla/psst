use super::{theme, utils};
use crate::{
    cmd,
    data::{
        news::{group_by_artist, ArtistReleases, NewsFeed, Release},
        AppState, ArtistLink, Nav,
    },
    webapi::WebApi,
    widget::{Async, MyWidgetExt, RemoteImage},
};
use druid::{
    im::Vector,
    lens::Map,
    widget::Painter,
    widget::{Button, CrossAxisAlignment, Flex, Label, LineBreaking, List, Scroll},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LensExt, LifeCycle, LifeCycleCtx,
    PaintCtx, Point, RenderContext, Selector, Size, UpdateCtx, Widget, WidgetExt, WidgetPod,
};

pub const LOAD: Selector = Selector::new("news.load");
pub const MARK_READ: Selector = Selector::new("news.mark-read");

pub fn widget() -> impl Widget<AppState> {
    let feed = Async::new(utils::spinner_widget, || {
        Flex::column().cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(Label::dynamic(|feed: &NewsFeed, _| format!("{} lanzamientos · {} artistas seguidos{}", feed.releases.len(), feed.followed_count,
                if feed.failed_count > 0 { format!(" · No se pudieron consultar {} artistas", feed.failed_count) } else { String::new() }))
                .with_line_break_mode(LineBreaking::WordWrap).expand_width())
            .with_spacer(12.0)
            .with_child(Label::dynamic(|feed: &NewsFeed, _| feed.notice.clone()).with_text_color(theme::PLACEHOLDER_COLOR).with_line_break_mode(LineBreaking::WordWrap).expand_width())
            .with_spacer(16.0)
            .with_child(druid::widget::Either::new(|feed: &NewsFeed, _| feed.releases.is_empty(),
                Label::dynamic(|feed: &NewsFeed, _| if feed.failed_count > 0 {
                    "No se pudieron cargar las novedades de todos tus artistas. Vuelve a intentarlo m\u{00e1}s tarde.".into()
                } else {
                    "Tus artistas no tienen lanzamientos publicados en los \u{00fa}ltimos 14 d\u{00ed}as.".into()
                })
                    .with_line_break_mode(LineBreaking::WordWrap).expand_width().boxed(),
                ArtistCards::default().lens(Map::new(|feed: &NewsFeed| group_by_artist(&feed.releases), |_, _| {})).boxed()))
    }, utils::error_widget).lens(AppState::news.then(crate::data::news::NewsState::feed));
    Scroll::new(Flex::column().cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Novedades de tus artistas").with_font(theme::UI_FONT_MEDIUM).with_text_size(24.0))
        .with_spacer(12.0)
        .with_child(Label::new("Álbumes y sencillos publicados en los últimos 14 días por los artistas que sigues en Spotify.")
            .with_line_break_mode(LineBreaking::WordWrap).expand_width())
        .with_spacer(8.0)
        .with_child(Label::new("Esta vista muestra lanzamientos; los anuncios y mensajes de Spotify se consultan en la app oficial.")
            .with_text_color(theme::PLACEHOLDER_COLOR).with_text_size(theme::TEXT_SIZE_SMALL)
            .with_line_break_mode(LineBreaking::WordWrap).expand_width())
        .with_spacer(16.0)
        .with_child(Flex::row()
            .with_child(Button::new("Actualizar").on_click(|ctx, _: &mut AppState, _| ctx.submit_command(LOAD)).disabled_if(|state: &AppState, _| state.news.feed.is_deferred(&())))
            .with_spacer(12.0).with_child(Button::new("Marcar como leídas").on_click(|ctx, _: &mut AppState, _| ctx.submit_command(MARK_READ))))
        .with_spacer(24.0).with_child(feed).padding(24.0).expand_width()).vertical()
}

fn artist_card() -> impl Widget<ArtistReleases> {
    let heading = Label::dynamic(|artist: &ArtistReleases, _| artist.name.clone())
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_size(18.0)
        .with_line_break_mode(LineBreaking::WordWrap)
        .expand_width()
        .link()
        .rounded(6.0)
        .tooltip("Abrir artista")
        .on_left_click(|ctx, _, artist, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(ArtistLink {
                id: artist.id.clone().into(),
                name: artist.name.clone().into(),
            })))
        });
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(heading)
        .with_spacer(6.0)
        .with_child(
            Label::dynamic(|artist: &ArtistReleases, _| {
                let unread = artist
                    .releases
                    .iter()
                    .filter(|release| release.unread)
                    .count();
                format!(
                    "{} {}{}",
                    artist.releases.len(),
                    if artist.releases.len() == 1 {
                        "lanzamiento"
                    } else {
                        "lanzamientos"
                    },
                    if unread > 0 {
                        format!(" · {unread} sin leer")
                    } else {
                        String::new()
                    }
                )
            })
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_line_break_mode(LineBreaking::WordWrap)
            .expand_width(),
        )
        .with_spacer(12.0)
        .with_child(
            List::new(release_row)
                .with_spacing(4.0)
                .lens(ArtistReleases::releases),
        )
        .padding(16.0)
        .expand_width()
        .background(Painter::new(|ctx, _: &ArtistReleases, env| {
            let rect = ctx.size().to_rect().inset(-0.5).to_rounded_rect(12.0);
            ctx.fill(rect, &env.get(theme::GREY_600));
            ctx.stroke(rect, &env.get(theme::GREY_500), 1.0);
        }))
}

// Each card owns its releases. Resize only changes positions and widths, so
// unread state, image loading and click targets survive column changes.
#[derive(Default)]
struct ArtistCards {
    children: Vec<WidgetPod<ArtistReleases, Box<dyn Widget<ArtistReleases>>>>,
}
impl ArtistCards {
    fn reconcile(&mut self, len: usize) -> bool {
        let changed = self.children.len() != len;
        self.children.truncate(len);
        while self.children.len() < len {
            self.children.push(WidgetPod::new(artist_card().boxed()));
        }
        changed
    }
}
impl Widget<Vector<ArtistReleases>> for ArtistCards {
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Vector<ArtistReleases>,
        env: &Env,
    ) {
        for (child, artist) in self.children.iter_mut().zip(data.iter_mut()) {
            child.event(ctx, event, artist, env);
        }
    }
    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Vector<ArtistReleases>,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) && self.reconcile(data.len()) {
            ctx.children_changed();
        }
        for (child, artist) in self.children.iter_mut().zip(data) {
            child.lifecycle(ctx, event, artist, env);
        }
    }
    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _: &Vector<ArtistReleases>,
        data: &Vector<ArtistReleases>,
        env: &Env,
    ) {
        if self.reconcile(data.len()) {
            ctx.children_changed();
        }
        for (child, artist) in self.children.iter_mut().zip(data) {
            if child.is_initialized() {
                child.update(ctx, artist, env);
            }
        }
        ctx.request_layout();
    }
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Vector<ArtistReleases>,
        env: &Env,
    ) -> Size {
        const GAP: f64 = 16.0;
        let width = if bc.max().width.is_finite() {
            bc.max().width
        } else {
            680.0
        };
        let columns = ((width + GAP) / (320.0 + GAP)).floor().max(1.0) as usize;
        let card_width = ((width - GAP * (columns - 1) as f64) / columns as f64).max(0.0);
        let child_bc = BoxConstraints::new(
            Size::new(card_width, 0.0),
            Size::new(card_width, f64::INFINITY),
        );
        let mut y = 0.0;
        for (row_index, row) in self.children.chunks_mut(columns).enumerate() {
            let mut height: f64 = 0.0;
            for (column, child) in row.iter_mut().enumerate() {
                let index = row_index * columns + column;
                let size = child.layout(ctx, &child_bc, &data[index], env);
                child.set_origin(ctx, Point::new(column as f64 * (card_width + GAP), y));
                height = height.max(size.height);
            }
            y += height + GAP;
        }
        bc.constrain(Size::new(width, (y - GAP).max(0.0)))
    }
    fn paint(&mut self, ctx: &mut PaintCtx, data: &Vector<ArtistReleases>, env: &Env) {
        for (child, artist) in self.children.iter_mut().zip(data) {
            child.paint(ctx, artist, env);
        }
    }
}

fn release_row() -> impl Widget<Release> {
    let image = RemoteImage::new(utils::placeholder_widget(), |release: &Release, _| {
        release
            .album
            .image(48.0, 48.0)
            .map(|image| image.url.clone())
    })
    .fix_size(48.0, 48.0)
    .clip(Size::new(48.0, 48.0).to_rounded_rect(6.0));
    Flex::row()
        .with_child(image)
        .with_spacer(12.0)
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(
                    Label::dynamic(|release: &Release, _| {
                        format!(
                            "{}{}",
                            if release.unread { "● " } else { "" },
                            release.album.name
                        )
                    })
                    .with_font(theme::UI_FONT_MEDIUM)
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .expand_width(),
                )
                .with_spacer(6.0)
                .with_child(
                    Label::dynamic(|release: &Release, _| {
                        format!(
                            "{} · {}",
                            match release.album.album_type {
                                crate::data::AlbumType::Single => "Sencillo",
                                crate::data::AlbumType::Album => "Álbum",
                                _ => "Lanzamiento",
                            },
                            release.date
                        )
                    })
                    .with_text_color(theme::PLACEHOLDER_COLOR)
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .expand_width(),
                ),
            1.0,
        )
        .padding(8.0)
        .expand_width()
        .link()
        .rounded(8.0)
        .tooltip("Abrir este lanzamiento")
        .on_left_click(|ctx, _, release, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::AlbumDetail(release.album.link(), None)))
        })
}

pub fn controller(widget: impl Widget<AppState> + 'static) -> impl Widget<AppState> {
    widget
        .on_command_async(
            LOAD,
            |_| WebApi::global().followed_artist_news(),
            |_, state, _| state.news.feed.defer(()),
            |_, state, (_, result)| {
                let result = result.map(|mut feed| {
                    for release in feed.releases.iter_mut() {
                        release.unread = !state
                            .config
                            .seen_releases
                            .iter()
                            .any(|id| id == release.album.id.as_ref());
                    }
                    feed
                });
                state.news.feed.update(((), result));
            },
        )
        .on_command(MARK_READ, |_, _, state| {
            if let Some(feed) = state.news.feed.resolved_mut() {
                for release in feed.releases.iter_mut() {
                    release.unread = false;
                    if !state
                        .config
                        .seen_releases
                        .iter()
                        .any(|id| id == release.album.id.as_ref())
                    {
                        state
                            .config
                            .seen_releases
                            .push_back(release.album.id.to_string());
                    }
                }
                while state.config.seen_releases.len() > 2000 {
                    state.config.seen_releases.pop_front();
                }
                state.config.save();
            }
        })
}
