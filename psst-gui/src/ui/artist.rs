use druid::{
    im::Vector,
    kurbo::Circle,
    widget::{CrossAxisAlignment, Flex, Label, LabelText, LineBreaking, List, Scroll},
    BoxConstraints, Data, Env, Event, EventCtx, Insets, LayoutCtx, LensExt, LifeCycle,
    LifeCycleCtx, LocalizedString, Menu, MenuItem, PaintCtx, Point, Selector, Size, UnitPoint,
    UpdateCtx, Widget, WidgetExt, WidgetPod,
};

use crate::{
    cmd,
    data::{
        AppState, Artist, ArtistAlbums, ArtistDetail, ArtistInfo, ArtistLink, ArtistTracks, Cached,
        Ctx, Nav, WithCtx,
    },
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, RemoteImage},
};

use super::{
    album, playable, theme, track,
    utils::{self},
};

pub const LOAD_DETAIL: Selector<ArtistLink> = Selector::new("app.artist.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(async_artist_info().padding((theme::grid(1.0), 0.0)))
        .with_child(async_top_tracks_widget())
        .with_child(async_albums_widget().padding((theme::grid(1.0), 0.0)))
        .with_child(async_related_widget().padding((theme::grid(1.0), 0.0)))
}

fn async_top_tracks_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        top_tracks_widget,
        utils::error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::artist_detail.then(ArtistDetail::top_tracks),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_DETAIL,
        |d| WebApi::global().get_artist_top_tracks(&d.id),
        |_, data, d| data.artist_detail.top_tracks.defer(d),
        |_, data, (d, r)| {
            let r = r.map(|tracks| ArtistTracks {
                id: d.id.clone(),
                name: d.name.clone(),
                tracks,
            });
            data.artist_detail.top_tracks.update((d, r))
        },
    )
}

fn async_albums_widget() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, albums_widget, utils::error_widget)
        .lens(
            Ctx::make(
                AppState::common_ctx,
                AppState::artist_detail.then(ArtistDetail::albums),
            )
            .then(Ctx::in_promise()),
        )
        .on_command_async(
            LOAD_DETAIL,
            |d| WebApi::global().get_artist_albums(&d.id),
            |_, data, d| data.artist_detail.albums.defer(d),
            |_, data, r| data.artist_detail.albums.update(r),
        )
}

fn async_artist_info() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        artist_info_widget,
        || Empty,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::artist_detail.then(ArtistDetail::artist_info),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_DETAIL,
        |d| WebApi::global().get_artist_info(&d.id),
        |_, data, d| data.artist_detail.artist_info.defer(d),
        |_, data, r| data.artist_detail.artist_info.update(r),
    )
}

fn async_related_widget() -> impl Widget<AppState> {
    Async::new(utils::spinner_widget, related_widget, utils::error_widget)
        .lens(AppState::artist_detail.then(ArtistDetail::related_artists))
        .on_command_async(
            LOAD_DETAIL,
            |d| WebApi::global().get_related_artists(&d.id),
            |_, data, d| data.artist_detail.related_artists.defer(d),
            |_, data, r| data.artist_detail.related_artists.update(r),
        )
}

pub fn artist_widget(horizontal: bool) -> impl Widget<Artist> {
    let (mut artist, artist_image) = if horizontal {
        (Flex::column(), cover_widget(theme::grid(16.0)))
    } else {
        (Flex::row(), cover_widget(theme::grid(6.0)))
    };

    artist = if horizontal {
        artist
            .with_child(artist_image)
            .with_default_spacer()
            .with_child(
                Label::raw()
                    .with_font(theme::UI_FONT_MEDIUM)
                    .align_horizontal(UnitPoint::CENTER)
                    .align_vertical(UnitPoint::TOP)
                    .fix_size(theme::grid(16.0), theme::grid(8.0))
                    .lens(Artist::name),
            )
    } else {
        artist
            .with_child(artist_image)
            .with_default_spacer()
            .with_flex_child(
                Label::raw()
                    .with_font(theme::UI_FONT_MEDIUM)
                    .lens(Artist::name),
                1.0,
            )
    };

    artist
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, artist, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(artist.link())));
        })
        .context_menu(|artist| artist_menu(&artist.link()))
}

pub fn link_widget() -> impl Widget<ArtistLink> {
    Label::raw()
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_font(theme::UI_FONT_MEDIUM)
        .link()
        .lens(ArtistLink::name)
        .on_left_click(|ctx, _, link, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::ArtistDetail(link.to_owned())));
        })
        .context_menu(artist_menu)
}

pub fn cover_widget(size: f64) -> impl Widget<Artist> {
    let radius = size / 2.0;
    RemoteImage::new(utils::placeholder_widget(), move |artist: &Artist, _| {
        artist.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
    .clip(Circle::new((radius, radius), radius))
}

fn artist_info_widget() -> impl Widget<WithCtx<ArtistInfo>> {
    let size = theme::grid(15.0);

    let artist_image = RemoteImage::new(
        utils::placeholder_widget(),
        move |artist: &ArtistInfo, _| Some(artist.main_image.clone()),
    )
    .fix_size(size, size)
    .clip(Size::new(size, size).to_rounded_rect(4.0))
    .lens(Ctx::data());

    let biography = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Scroll::new(
                Label::new(|data: &ArtistInfo, _env: &_| data.bio.clone())
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(theme::TEXT_SIZE_NORMAL)
                    .lens(Ctx::data()),
            )
            .vertical()
            .fix_height(size - theme::grid(1.5)),
        );

    let artist_stats = Flex::column()
        .with_child(stat_row("Followers:", |info: &ArtistInfo| {
            format!("{} followers", info.stats.followers)
        }))
        .with_child(stat_row("Monthly Listeners:", |info: &ArtistInfo| {
            format!("{} monthly listeners", info.stats.monthly_listeners)
        }))
        .with_child(stat_row("Ranking:", |info: &ArtistInfo| {
            if !info.stats.world_rank.starts_with("0") {
                format!("#{} in the world", info.stats.world_rank)
            } else {
                "N/A".to_string()
            }
        }));

    Flex::row()
        .with_child(artist_image)
        .with_spacer(theme::grid(1.0))
        .with_flex_child(
            Flex::row().with_flex_child(ArtistInfoLayout::new(biography, artist_stats), 1.0),
            1.0,
        )
        .context_menu(|artist| artist_info_menu(&artist.data))
        .padding((0.0, theme::grid(1.0))) // Keep overall vertical padding
}

struct ArtistInfoLayout<T, B, S> {
    biography: WidgetPod<T, B>,
    stats: WidgetPod<T, S>,
}

impl<T, B, S> ArtistInfoLayout<T, B, S>
where
    T: Data,
    B: Widget<T>,
    S: Widget<T>,
{
    fn new(biography: B, stats: S) -> Self {
        Self {
            biography: WidgetPod::new(biography),
            stats: WidgetPod::new(stats),
        }
    }
}

impl<T: Data, B: Widget<T>, S: Widget<T>> Widget<T> for ArtistInfoLayout<T, B, S> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.biography.event(ctx, event, data, env);
        self.stats.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.biography.lifecycle(ctx, event, data, env);
        self.stats.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.biography.update(ctx, data, env);
        self.stats.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let max = bc.max();
        let wide_layout = max.width > theme::grid(60.0) + theme::GRID * 2.0;
        let padding = theme::grid(1.0); // Padding between bio and stats

        if wide_layout {
            let biography_width = max.width * 0.75 - padding / 2.0;
            let stats_width = max.width * 0.25 - padding / 2.0;

            let biography_bc =
                BoxConstraints::new(Size::ZERO, Size::new(biography_width, max.height));
            let stats_bc = BoxConstraints::new(Size::ZERO, Size::new(stats_width, max.height));

            let biography_size = self.biography.layout(ctx, &biography_bc, data, env);
            let stats_size = self.stats.layout(ctx, &stats_bc, data, env);

            self.biography.set_origin(ctx, Point::ORIGIN);
            self.stats
                .set_origin(ctx, Point::new(biography_width + padding, 0.0));

            Size::new(max.width, biography_size.height.max(stats_size.height))
        } else {
            let biography_size = self.biography.layout(ctx, bc, data, env);
            let stats_bc = BoxConstraints::new(
                Size::ZERO,
                Size::new(max.width, max.height - biography_size.height - padding),
            );
            let stats_size = self.stats.layout(ctx, &stats_bc, data, env);

            self.biography.set_origin(ctx, Point::ORIGIN);
            self.stats
                .set_origin(ctx, Point::new(0.0, biography_size.height + padding));

            Size::new(
                max.width,
                biography_size.height + padding + stats_size.height,
            )
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.biography.paint(ctx, data, env);
        self.stats.paint(ctx, data, env);
    }
}

fn top_tracks_widget() -> impl Widget<WithCtx<ArtistTracks>> {
    playable::list_widget(playable::Display {
        track: track::Display {
            title: true,
            album: true,
            popularity: true,
            cover: true,
            ..track::Display::empty()
        },
    })
}

fn albums_widget() -> impl Widget<WithCtx<ArtistAlbums>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(header_widget("Albums"))
        .with_child(List::new(|| album::album_widget(false)).lens(Ctx::map(ArtistAlbums::albums)))
        .with_child(header_widget("Singles"))
        .with_child(List::new(|| album::album_widget(false)).lens(Ctx::map(ArtistAlbums::singles)))
        .with_child(header_widget("Compilations"))
        .with_child(
            List::new(|| album::album_widget(false)).lens(Ctx::map(ArtistAlbums::compilations)),
        )
        .with_child(header_widget("Appears On"))
        .with_child(
            List::new(|| album::album_widget(false)).lens(Ctx::map(ArtistAlbums::appears_on)),
        )
}

fn related_widget() -> impl Widget<Cached<Vector<Artist>>> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(header_widget("Related Artists"))
        .with_child(List::new(|| artist_widget(false)))
        .lens(Cached::data)
}

fn header_widget<T: Data>(text: impl Into<LabelText<T>>) -> impl Widget<T> {
    Label::new(text)
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .padding(Insets::new(0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
}

fn artist_info_menu(artist: &ArtistInfo) -> Menu<AppState> {
    let mut menu = Menu::empty();

    for artist_link in &artist.artist_links {
        let platform = if artist_link.contains("wikipedia.org") {
            "Wikipedia"
        } else {
            artist_link
                .strip_prefix("https://")
                .unwrap_or(artist_link)
                .split('.')
                .next()
                .unwrap_or("Unknown")
        };

        let title = LocalizedString::new("menu-item-go-to-social").with_placeholder(format!(
            "Go to their {}",
            platform
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .collect::<String>()
                + &platform[1..]
        ));

        menu =
            menu.entry(MenuItem::new(title).command(cmd::GO_TO_URL.with(artist_link.to_owned())));
    }

    menu
}

fn artist_menu(artist: &ArtistLink) -> Menu<AppState> {
    let mut menu = Menu::empty();

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Artist"),
        )
        .command(cmd::COPY.with(artist.url())),
    );

    menu
}

fn stat_row(
    label: &'static str,
    value_func: impl Fn(&ArtistInfo) -> String + 'static,
) -> impl Widget<WithCtx<ArtistInfo>> {
    Flex::row()
        .with_child(
            Label::new(label)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .with_text_color(theme::PLACEHOLDER_COLOR),
        )
        .with_spacer(theme::grid(0.5))
        .with_child(
            Label::new(move |ctx: &WithCtx<ArtistInfo>, _env: &_| value_func(&ctx.data))
                .with_text_size(theme::TEXT_SIZE_SMALL),
        )
        .align_left()
}
