use crate::{
    cmd,
    data::{Navigation, State},
};
use druid::{commands, platform_menus, LocalizedString, MenuDesc, MenuItem, SysMods};

#[allow(unused_mut)]
pub fn make_menu() -> MenuDesc<State> {
    let mut menu = MenuDesc::empty();
    #[cfg(target_os = "macos")]
    {
        menu = menu.append(make_mac_app_menu());
    }
    menu.append(make_edit_menu()).append(make_view_menu())
}

fn make_mac_app_menu() -> MenuDesc<State> {
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

fn make_edit_menu() -> MenuDesc<State> {
    MenuDesc::new(LocalizedString::new("common-menu-edit-menu").with_placeholder("Edit"))
        .append(platform_menus::common::cut())
        .append(platform_menus::common::copy())
        .append(platform_menus::common::paste())
}

fn make_view_menu() -> MenuDesc<State> {
    MenuDesc::new(LocalizedString::new("menu-view-menu").with_placeholder("View"))
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-home").with_placeholder("Home"),
                cmd::NAVIGATE_TO.with(Navigation::Home),
            )
            .hotkey(SysMods::Cmd, "1"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-library").with_placeholder("Library"),
                cmd::NAVIGATE_TO.with(Navigation::Library),
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
