use crate::data::Theme;

pub fn resolve(selected: Theme) -> Theme {
    match selected {
        Theme::System => {
            if system_is_dark() {
                Theme::Dark
            } else {
                Theme::Light
            }
        }
        explicit => explicit,
    }
}

#[cfg(windows)]
fn system_is_dark() -> bool {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
        .encode_utf16()
        .collect();
    let name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();
    let mut value = 1_u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // The fixed registry path is read-only; the DWORD buffer matches the requested type.
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut size,
        )
    };
    result == 0 && value == 0
}

#[cfg(not(windows))]
fn system_is_dark() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_theme_overrides_system_preference() {
        assert_eq!(resolve(Theme::Light), Theme::Light);
        assert_eq!(resolve(Theme::Dark), Theme::Dark);
        assert_eq!(Theme::default(), Theme::System);
        assert_ne!(resolve(Theme::System), Theme::System);
    }
}
