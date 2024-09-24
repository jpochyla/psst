use crate::{
    cmd,
    data::{AppState, ArtistLink, Nav},
    ui::theme,
    webapi::{CreditArtist, RoleCredit},
};
use druid::{
    widget::{Controller, CrossAxisAlignment, Flex, Label, LineBreaking, List, Painter, Scroll},
    Cursor, Env, Event, EventCtx, LensExt, RenderContext, Target, Widget, WidgetExt, WindowDesc,
};

pub fn credits_window(track_title: &str) -> WindowDesc<AppState> {
    WindowDesc::new(credits_widget())
        .title(format!("Credits: {}", track_title))
        .window_size((350.0, 500.0))
        .resizable(false)
}

fn credits_widget() -> impl Widget<AppState> {
    Scroll::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(
                Flex::column().with_child(
                    Label::new(|data: &AppState, _: &_| {
                        data.credits
                            .as_ref()
                            .map_or("Credits".to_string(), |c| c.track_title.clone())
                    })
                    .with_font(theme::UI_FONT_MEDIUM)
                    .with_text_size(theme::TEXT_SIZE_LARGE)
                    .padding(theme::grid(2.0))
                    .expand_width(),
                ),
            )
            .with_child(
                List::new(|| role_credit_widget()).lens(AppState::credits.map(
                    |credits| {
                        credits
                            .as_ref()
                            .map(|c| c.role_credits.clone())
                            .unwrap_or_default()
                    },
                    |_, _| {},
                )),
            )
            .with_child(
                Label::new(|data: &AppState, _: &_| {
                    data.credits.as_ref().map_or("".to_string(), |c| {
                        format!("Source: {}", c.source_names.join(", "))
                    })
                })
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .with_text_color(theme::PLACEHOLDER_COLOR)
                .with_line_break_mode(LineBreaking::WordWrap)
                .padding(theme::grid(2.0)),
            )
            .padding(theme::grid(2.0)),
    )
    .vertical()
    .expand()
}

fn role_credit_widget() -> impl Widget<RoleCredit> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::new(|role: &RoleCredit, _: &_| capitalize_first(&role.role_title))
                .with_text_size(theme::TEXT_SIZE_NORMAL)
                .padding((theme::grid(2.0), theme::grid(1.0))),
        )
        .with_child(
            List::new(|| credit_artist_widget())
                .lens(RoleCredit::artists)
                .padding((theme::grid(2.0), 0.0, theme::grid(2.0), 0.0)),
        )
}

fn credit_artist_widget() -> impl Widget<CreditArtist> {
    let painter = Painter::new(|ctx, data: &CreditArtist, env| {
        let bounds = ctx.size().to_rect();

        if ctx.is_hot() && data.uri.is_some() {
            ctx.fill(bounds, &env.get(theme::LINK_HOT_COLOR));
        } else if data.uri.is_some() {
            ctx.fill(bounds, &env.get(theme::LINK_COLD_COLOR));
        }

        if ctx.is_active() && data.uri.is_some() {
            ctx.fill(bounds, &env.get(theme::LINK_ACTIVE_COLOR));
        }
    });

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(
                    Label::new(|artist: &CreditArtist, _: &_| proper_case(&artist.name))
                        .with_font(theme::UI_FONT_MEDIUM),
                )
                .with_child(
                    Label::new(|artist: &CreditArtist, _: &_| {
                        capitalize_first(&artist.subroles.join(", "))
                    })
                    .with_text_size(theme::TEXT_SIZE_SMALL)
                    .with_text_color(theme::PLACEHOLDER_COLOR),
                )
                .padding(theme::grid(1.0))
                .expand_width()
                .background(painter)
                .rounded(theme::BUTTON_BORDER_RADIUS)
                .on_click(|ctx: &mut EventCtx, data: &mut CreditArtist, _: &Env| {
                    if let Some(uri) = &data.uri {
                        let artist_id = uri.split(':').last().unwrap_or("").to_string();
                        let artist_link = ArtistLink {
                            id: artist_id.into(),
                            name: data.name.clone().into(),
                        };
                        ctx.submit_command(
                            cmd::NAVIGATE
                                .with(Nav::ArtistDetail(artist_link))
                                .to(Target::Global),
                        );
                    }
                })
                .disabled_if(|artist: &CreditArtist, _| artist.uri.is_none())
                .controller(CursorController),
        )
        .padding((0.0, theme::grid(0.5)))
}

struct CursorController;

impl<W: Widget<CreditArtist>> Controller<CreditArtist, W> for CursorController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut CreditArtist,
        env: &Env,
    ) {
        match event {
            Event::MouseMove(_) => {
                if data.uri.is_some() {
                    ctx.set_cursor(&Cursor::Pointer);
                } else {
                    ctx.clear_cursor();
                }
            }
            _ => {}
        }
        child.event(ctx, event, data, env)
    }
}

fn proper_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => {
                    f.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
