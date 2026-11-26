use crate::{
    data::{AppState, Config, PlaylistLink, Promise, WithCtx},
    widget::{MyWidgetExt, ThemeScope},
};
use druid::{
    widget::{Button, Flex, Label, List, Scroll, TextBox},
    LensExt, Menu, MenuItem, Selector, Widget, WidgetExt, WindowDesc,
};

const MANAGE: Selector = Selector::new("app.folders.manage");
const SELECT: Selector<Option<String>> = Selector::new("app.folders.select");
const SAVE: Selector = Selector::new("app.folders.save");
const EDIT: Selector<String> = Selector::new("app.folders.edit");
const DELETE: Selector<String> = Selector::new("app.folders.delete");
pub const ASSIGN_WINDOW: Selector<PlaylistLink> = Selector::new("app.folders.assign-window");
const ASSIGN: Selector<(String, Option<String>)> = Selector::new("app.folders.assign");

pub fn toolbar() -> impl Widget<AppState> {
    Button::dynamic(|data: &AppState, _| {
        data.selected_folder
            .as_ref()
            .map(|name| format!("Carpeta: {name}"))
            .unwrap_or_else(|| "Todas las playlists ▾".into())
    })
    .on_click(|ctx, data, _| {
        let mut menu: Menu<AppState> =
            Menu::empty().entry(MenuItem::new("Todas las playlists").command(SELECT.with(None)));
        for name in &data.config.playlist_folders {
            menu = menu.entry(MenuItem::new(name.clone()).command(SELECT.with(Some(name.clone()))));
        }
        menu = menu
            .separator()
            .entry(MenuItem::new("Gestionar carpetas locales…").command(MANAGE));
        ctx.show_context_menu(
            menu,
            ctx.window_origin() + druid::Vec2::new(0.0, ctx.size().height),
        );
    })
    .expand_width()
    .padding((10.0, 4.0))
}

pub fn filtered_playlists(
    data: &AppState,
) -> WithCtx<Promise<druid::im::Vector<crate::data::Playlist>>> {
    let mut promise = data.library.playlists.clone();
    if let (Some(folder), Promise::Resolved { val, .. }) = (&data.selected_folder, &mut promise) {
        *val = val
            .iter()
            .filter(|playlist| {
                data.config
                    .playlist_folder_assignments
                    .get(playlist.id.as_ref())
                    == Some(folder)
            })
            .cloned()
            .collect();
    }
    WithCtx {
        ctx: data.common_ctx.clone(),
        data: promise,
    }
}

fn manager() -> impl Widget<AppState> {
    let list = List::new(|| {
        Label::raw()
            .padding((8.0, 10.0))
            .context_menu(|name: &String| {
                Menu::empty()
                    .entry(MenuItem::new("Renombrar").command(EDIT.with(name.clone())))
                    .entry(MenuItem::new("Eliminar carpeta").command(DELETE.with(name.clone())))
            })
    })
    .lens(AppState::config.then(Config::playlist_folders));
    Flex::column()
        .with_child(Label::new("Carpetas locales de Xpotify").padding(10.0))
        .with_child(Label::new("Organizan esta biblioteca en tu equipo.").padding(6.0))
        .with_child(
            TextBox::new()
                .with_placeholder("Nombre de la carpeta")
                .lens(AppState::folder_name)
                .expand_width()
                .padding(8.0),
        )
        .with_child(
            Button::dynamic(|data: &AppState, _| {
                if data.editing_folder.is_some() {
                    "Guardar nombre".to_owned()
                } else {
                    "Crear carpeta".to_owned()
                }
            })
            .on_click(|ctx, _, _| ctx.submit_command(SAVE))
            .padding(8.0),
        )
        .with_flex_child(Scroll::new(list).vertical(), 1.0)
        .padding(12.0)
}

fn assignments(link: PlaylistLink) -> impl Widget<AppState> {
    let id = link.id.to_string();
    let id_none = id.clone();
    let list = List::new(move || {
        let id = id.clone();
        Button::dynamic(|name: &String, _| name.clone())
            .on_click(move |ctx, name: &mut String, _| {
                ctx.submit_command(ASSIGN.with((id.clone(), Some(name.clone()))));
                ctx.submit_command(druid::commands::CLOSE_WINDOW.to(ctx.window_id()));
            })
            .expand_width()
            .padding(6.0)
    })
    .lens(AppState::config.then(Config::playlist_folders));
    Flex::column()
        .with_child(Label::new(format!("Organizar: {}", link.name)).padding(10.0))
        .with_child(
            Button::new("Sin carpeta")
                .on_click(move |ctx, _, _| {
                    ctx.submit_command(ASSIGN.with((id_none.clone(), None)));
                    ctx.submit_command(druid::commands::CLOSE_WINDOW.to(ctx.window_id()));
                })
                .padding(6.0),
        )
        .with_flex_child(Scroll::new(list).vertical(), 1.0)
        .with_child(
            Button::new("Gestionar carpetas")
                .on_click(|ctx, _, _| ctx.submit_command(MANAGE))
                .padding(8.0),
        )
        .padding(12.0)
}

fn save_folder(config: &mut Config, old: Option<&str>, name: &str) -> Result<(), &'static str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Usa un nombre de 1 a 80 caracteres.");
    }
    if config
        .playlist_folders
        .iter()
        .any(|existing| existing == name && Some(existing.as_str()) != old)
    {
        return Err("Ya existe una carpeta con ese nombre.");
    }
    if let Some(old) = old {
        let index = config
            .playlist_folders
            .iter()
            .position(|existing| existing == old)
            .ok_or("La carpeta ya no existe.")?;
        config.playlist_folders.set(index, name.to_owned());
        config.playlist_folder_assignments = config
            .playlist_folder_assignments
            .iter()
            .map(|(id, folder)| {
                (
                    id.clone(),
                    if folder == old {
                        name.to_owned()
                    } else {
                        folder.clone()
                    },
                )
            })
            .collect();
    } else {
        config.playlist_folders.push_back(name.to_owned());
    }
    Ok(())
}

pub fn controller<W: Widget<AppState> + 'static>(child: W) -> impl Widget<AppState> {
    child
        .on_command(MANAGE, |ctx, _, data| {
            data.folder_name.clear();
            data.editing_folder = None;
            ctx.new_window(
                WindowDesc::new(ThemeScope::new(manager()))
                    .title("Carpetas de Xpotify")
                    .window_size((420.0, 480.0)),
            );
        })
        .on_command(SELECT, |_, folder, data| {
            data.selected_folder = folder.clone();
        })
        .on_command(SAVE, |_, _, data| {
            match save_folder(
                &mut data.config,
                data.editing_folder.as_deref(),
                &data.folder_name,
            ) {
                Ok(()) => {
                    if data.selected_folder == data.editing_folder && data.editing_folder.is_some()
                    {
                        data.selected_folder = Some(data.folder_name.trim().to_owned());
                    }
                    data.folder_name.clear();
                    data.editing_folder = None;
                    data.config.save();
                }
                Err(error) => data.error_alert(error),
            }
        })
        .on_command(EDIT, |_, name, data| {
            data.folder_name = name.clone();
            data.editing_folder = Some(name.clone());
        })
        .on_command(DELETE, |_, name, data| {
            data.config.playlist_folders.retain(|folder| folder != name);
            data.config
                .playlist_folder_assignments
                .retain(|_, folder| folder != name);
            if data.selected_folder.as_ref() == Some(name) {
                data.selected_folder = None;
            }
            if data.editing_folder.as_ref() == Some(name) {
                data.editing_folder = None;
                data.folder_name.clear();
            }
            data.config.save();
        })
        .on_command(ASSIGN_WINDOW, |ctx, link, _| {
            ctx.new_window(
                WindowDesc::new(ThemeScope::new(assignments(link.clone())))
                    .title("Organizar playlist")
                    .window_size((420.0, 480.0)),
            );
        })
        .on_command(ASSIGN, |_, (id, folder), data| {
            if let Some(folder) = folder {
                if data.config.playlist_folders.contains(folder) {
                    data.config
                        .playlist_folder_assignments
                        .insert(id.clone(), folder.clone());
                }
            } else {
                data.config.playlist_folder_assignments.remove(id);
            }
            data.config.save();
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rename_preserves_membership_and_rejects_duplicate_names() {
        let mut config = Config::default();
        save_folder(&mut config, None, " Rock ").unwrap();
        config
            .playlist_folder_assignments
            .insert("playlist-a".into(), "Rock".into());
        save_folder(&mut config, Some("Rock"), "Guitarras").unwrap();
        assert_eq!(
            config
                .playlist_folder_assignments
                .get("playlist-a")
                .unwrap(),
            "Guitarras"
        );
        assert!(save_folder(&mut config, None, "Guitarras").is_err());
        assert!(save_folder(&mut config, None, "   ").is_err());
        assert!(save_folder(&mut config, Some("missing"), "Otro").is_err());
        let roundtrip: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(
            roundtrip.playlist_folder_assignments,
            config.playlist_folder_assignments
        );
    }
}
