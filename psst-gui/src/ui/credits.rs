//! Track credits window and related widgets.
//!
//! This module handles displaying detailed credit information for tracks,
//! including artists, roles, and sources.

use std::sync::Arc;

use crate::{
    cmd,
    data::{AppState, ArtistLink, Nav},
    ui::theme,
    ui::utils,
};
use druid::{
    widget::{Controller, CrossAxisAlignment, Flex, Label, List, Maybe, Painter, Scroll},
    Cursor, Data, Env, Event, EventCtx, Lens, RenderContext, Target, UpdateCtx, Widget, WidgetExt,
};
use serde::Deserialize;

#[derive(Debug, Clone, Data, Lens, Deserialize)]
pub struct TrackCredits {
    #[serde(rename = "trackUri")]
    pub track_uri: String,
    #[serde(rename = "trackTitle")]
    pub track_title: String,
    #[serde(rename = "roleCredits")]
    pub role_credits: Arc<Vec<RoleCredit>>,
    #[serde(rename = "extendedCredits")]
    pub extended_credits: Arc<Vec<String>>,
    #[serde(rename = "sourceNames")]
    pub source_names: Arc<Vec<String>>,
}

#[derive(Debug, Clone, Data, Lens, Deserialize)]
pub struct RoleCredit {
    #[serde(rename = "roleTitle")]
    pub role_title: String,
    pub artists: Arc<Vec<CreditArtist>>,
}

#[derive(Debug, Clone, Data, Lens, Deserialize)]
pub struct CreditArtist {
    pub uri: Option<String>,
    pub name: String,
    #[serde(rename = "imageUri")]
    pub image_uri: Option<String>,
    #[serde(rename = "externalUrl")]
    pub external_url: Option<String>,
    #[serde(rename = "creatorUri")]
    pub creator_uri: Option<String>,
    #[serde(default)]
    pub subroles: Arc<Vec<String>>,
    #[serde(default)]
    pub weight: f64,
}

pub fn credits_widget() -> impl Widget<AppState> {
    Scroll::new(
        Maybe::new(
            || {
                Flex::column()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(
                        Label::new(|data: &TrackCredits, _: &_| data.track_title.clone())
                            .with_font(theme::UI_FONT_MEDIUM)
                            .with_text_size(theme::TEXT_SIZE_LARGE)
                            .padding(theme::grid(2.0))
                            .expand_width(),
                    )
                    .with_child(List::new(role_credit_widget).lens(TrackCredits::role_credits))
                    .with_child(
                        Label::new(|data: &TrackCredits, _: &_| {
                            format!("Source: {}", data.source_names.join(", "))
                        })
                        .with_text_size(theme::TEXT_SIZE_SMALL)
                        .with_text_color(theme::PLACEHOLDER_COLOR)
                        .padding(theme::grid(2.0)),
                    )
                    .padding(theme::grid(2.0))
            },
            || utils::spinner_widget(),
        )
        .lens(AppState::credits)
        .controller(CreditsController),
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
            List::new(credit_artist_widget)
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
        if let Event::MouseMove(_) = event {
            if data.uri.is_some() {
                ctx.set_cursor(&Cursor::Pointer);
            } else {
                ctx.clear_cursor();
            }
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

/// Controller that handles updating the credits view when data changes
pub struct CreditsController;

impl<W: Widget<AppState>> Controller<AppState, W> for CreditsController {
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &Env,
    ) {
        if !old_data.credits.same(&data.credits) {
            ctx.request_layout();
            ctx.request_paint();
        }
        child.update(ctx, old_data, data, env)
    }
}
