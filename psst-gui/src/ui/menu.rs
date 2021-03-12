use crate::{
    cmd,
    data::{Nav, State},
};
use druid::{commands, platform_menus, LocalizedString, MenuDesc, MenuItem, SysMods};

pub fn main_menu() -> MenuDesc<State> {
    #[allow(unused_mut)]
    let mut menu = MenuDesc::empty();
    #[cfg(target_os = "macos")]
    {
        menu = menu.append(mac_app_menu());
    }
    menu.append(edit_menu()).append(view_menu())
}

fn mac_app_menu() -> MenuDesc<State> {
    MenuDesc::new(LocalizedString::new("macos-menu-application-menu"))
        .append(platform_menus::mac::application::preferences())
        .append_separator()
        .append(
            // TODO:
            //  This is just overriding `platform_menus::mac::application::quit()`
            //  because l10n is a bit stupid now.
            MenuItem::new(
                LocalizedString::new("macos-menu-quit").with_placeholder("Quit Psst"),
                commands::QUIT_APP,
            )
            .hotkey(SysMods::Cmd, "q"),
        )
}

fn edit_menu() -> MenuDesc<State> {
    MenuDesc::new(LocalizedString::new("common-menu-edit-menu").with_placeholder("Edit"))
        .append(platform_menus::common::cut())
        .append(platform_menus::common::copy())
        .append(platform_menus::common::paste())
}

fn view_menu() -> MenuDesc<State> {
    MenuDesc::new(LocalizedString::new("menu-view-menu").with_placeholder("View"))
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-home").with_placeholder("Home"),
                cmd::NAVIGATE_TO.with(Nav::Home),
            )
            .hotkey(SysMods::Cmd, "1"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-library").with_placeholder("Library"),
                cmd::NAVIGATE_TO.with(Nav::Library),
            )
            .hotkey(SysMods::Cmd, "2"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-search").with_placeholder("Search..."),
                cmd::SET_FOCUS.to(cmd::WIDGET_SEARCH_INPUT),
            )
            .hotkey(SysMods::Cmd, "l"),
        )
}
