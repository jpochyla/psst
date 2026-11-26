use super::theme;
use crate::{
    data::{AppState, Playlist, PlaylistLink, Promise, TrackId},
    widget::{MyWidgetExt, ThemeScope},
};
use druid::{
    im::Vector,
    widget::{Button, Flex, Label, LineBreaking, List, Scroll, TextBox},
    Selector, WidgetExt, WindowDesc,
};

pub const OPEN: Selector<TrackId> = Selector::new("app.playlist-picker.open");
pub const SELECT: Selector<PlaylistLink> = Selector::new("app.playlist-picker.select");

pub fn parse_target(value: &str) -> Option<PlaylistLink> {
    let value = value.trim();
    let id = if let Some(id) = value.strip_prefix("spotify:playlist:") {
        id.to_owned()
    } else {
        let url = url::Url::parse(value).ok()?;
        if url.scheme() != "https" || url.host_str()? != "open.spotify.com" {
            return None;
        }
        let parts: Vec<_> = url.path_segments()?.collect();
        if parts.len() != 2 || parts[0] != "playlist" {
            return None;
        }
        parts[1].to_owned()
    };
    if id.len() != 22 || !id.bytes().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(PlaylistLink {
        id: id.into(),
        name: "Playlist".into(),
    })
}

fn targets(data: &AppState) -> Vector<Playlist> {
    let filter = data.playlist_picker_filter.trim().to_lowercase();
    data.library
        .writable_playlists()
        .into_iter()
        .filter(|p| p.name.to_lowercase().contains(&filter))
        .cloned()
        .collect()
}

pub fn status(data: &AppState) -> String {
    if !data.playlist_picker_status.is_empty() {
        return data.playlist_picker_status.clone();
    }
    if let Some(promise_error) = [
        match &data.library.playlists {
            Promise::Rejected { err, .. } => Some(err),
            _ => None,
        },
        match &data.library.user_profile {
            Promise::Rejected { err, .. } => Some(err),
            _ => None,
        },
    ]
    .into_iter()
    .flatten()
    .next()
    {
        return format!("No se pudo cargar tu biblioteca: {promise_error}");
    }
    if data.library.playlists.is_deferred(&()) || data.library.user_profile.is_deferred(&()) {
        "Cargando tus playlists...".into()
    } else if !data.library.user_profile.is_resolved() {
        "Falta cargar tu perfil para comprobar que puedes editar las playlists.".into()
    } else if data.library.writable_playlists().is_empty() {
        "No hay playlists propias o colaborativas disponibles.".into()
    } else {
        "Elige una playlist propia o colaborativa.".into()
    }
}

pub fn window() -> WindowDesc<AppState> {
    let list = List::new(|| {
        Flex::row()
            .with_child(super::playlist::picker_cover().fix_size(44.0, 44.0))
            .with_spacer(12.0)
            .with_flex_child(
                Label::raw()
                    .with_line_break_mode(LineBreaking::Clip)
                    .lens(Playlist::name)
                    .expand_width(),
                1.0,
            )
            .padding(10.0)
            .expand_width()
            .link()
            .rounded(6.0)
            .on_left_click(|ctx, _, playlist: &mut Playlist, _| {
                ctx.submit_command(SELECT.with(playlist.link()))
            })
    })
    .lens(druid::lens::Map::new(targets, |_, _| {}));
    let content = Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(
            Label::new("Añadir a una playlist")
                .with_font(theme::UI_FONT_MEDIUM)
                .with_text_size(22.0),
        )
        .with_spacer(12.0)
        .with_child(
            Label::dynamic(|data: &AppState, _| status(data))
                .with_line_break_mode(LineBreaking::WordWrap)
                .expand_width(),
        )
        .with_spacer(12.0)
        .with_child(
            TextBox::new()
                .with_placeholder("Filtrar playlists")
                .expand_width()
                .lens(AppState::playlist_picker_filter),
        )
        .with_spacer(10.0)
        .with_flex_child(Scroll::new(list).vertical().expand_width(), 1.0)
        .with_spacer(12.0)
        .with_child(
            Label::new("También puedes pegar el enlace de una playlist editable.")
                .with_line_break_mode(LineBreaking::WordWrap)
                .expand_width(),
        )
        .with_spacer(8.0)
        .with_child(
            TextBox::new()
                .with_placeholder("https://open.spotify.com/playlist/...")
                .expand_width()
                .lens(AppState::playlist_picker_url),
        )
        .with_spacer(12.0)
        .with_child(
            Flex::row()
                .with_child(
                    Button::new("Añadir por enlace")
                        .on_click(|ctx, data: &mut AppState, _| {
                            if let Some(link) = parse_target(&data.playlist_picker_url) {
                                ctx.submit_command(SELECT.with(link));
                            }
                        })
                        .disabled_if(|data: &AppState, _| {
                            parse_target(&data.playlist_picker_url).is_none()
                        }),
                )
                .with_spacer(12.0)
                .with_child(Button::new("Actualizar biblioteca").on_click(
                    |ctx, data: &mut AppState, _| {
                        if let Some(error) = crate::webapi::WebApi::global().rate_limit_error() {
                            data.playlist_picker_status = error.to_string();
                            data.error_alert(error);
                            return;
                        }
                        data.playlist_picker_status.clear();
                        if !data.library.playlists.is_deferred(&()) {
                            ctx.submit_command(super::playlist::LOAD_LIST);
                        }
                        if !data.library.user_profile.is_deferred(&()) {
                            ctx.submit_command(super::user::LOAD_PROFILE);
                        }
                    },
                )),
        )
        .padding(24.0)
        .expand()
        .background(theme::BACKGROUND_DARK);
    WindowDesc::new(ThemeScope::new(content))
        .title("Xpotify · Añadir a playlist")
        .window_size((580.0, 620.0))
        .with_min_size((480.0, 520.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_real_spotify_playlist_targets() {
        let id = "37i9dQZF1DX4WYpdgoIcn6";
        assert_eq!(
            parse_target(&format!("https://open.spotify.com/playlist/{id}?si=x"))
                .unwrap()
                .id
                .as_ref(),
            id
        );
        assert!(parse_target(&format!("spotify:playlist:{id}")).is_some());
        for value in [
            "https://evil.test/playlist/37i9dQZF1DX4WYpdgoIcn6",
            "https://open.spotify.com/track/37i9dQZF1DX4WYpdgoIcn6",
            "https://open.spotify.com/playlist/../me",
            "spotify:playlist:short",
        ] {
            assert!(parse_target(value).is_none());
        }
    }
    #[test]
    fn missing_profile_is_explained_instead_of_empty_submenu() {
        let data = AppState::default_with_config(Default::default());
        assert!(status(&data).contains("perfil"));
    }
}
