use druid::Data;
use druid_shell::{Code, KbKey, KeyEvent};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Keyboard Shortcut comprised of either a [`KbKey`] or a [`Code`]
pub enum KbShortcut {
    Key(KbKey),
    Code(Code),
}

impl Data for KbShortcut {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl KbShortcut {
    /// Return whether a given [`KeyEvent`] matches this shortcut
    pub fn matches(&self, event: &KeyEvent) -> bool {
        match self {
            KbShortcut::Key(key) => &event.key == key,
            KbShortcut::Code(code) => &event.code == code,
        }
    }
}

/// Given a [`&str`], return the corresponding [`Code`] or an `Err` if there is no corresponding
/// code.
/// TODO: Add more String-to-Code mappings
fn kb_code_from_str(s: &str) -> Result<Code, ()> {
    match s {
        "NumpadAdd" => Ok(Code::NumpadAdd),
        "Minus" => Ok(Code::Minus),
        "Space" => Ok(Code::Space),
        "ArrowRight" => Ok(Code::ArrowRight),
        "ArrowLeft" => Ok(Code::ArrowLeft),
        "ArrowUp" => Ok(Code::ArrowUp),
        "ArrowDown" => Ok(Code::ArrowDown),
        "Backspace" => Ok(Code::Backspace),
        "Enter" => Ok(Code::Enter),
        "Tab" => Ok(Code::Tab),
        _ => Err(()),
    }
}

impl fmt::Display for KbShortcut {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KbShortcut::Key(key) => {
                write!(f, "{}", key.to_string())
            }
            KbShortcut::Code(code) => {
                write!(f, "{}", code.to_string())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseShortcutError;

impl fmt::Display for ParseShortcutError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error parsing shortcut!")
    }
}

impl FromStr for KbShortcut {
    type Err = ParseShortcutError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(code) = kb_code_from_str(s) {
            Ok(KbShortcut::Code(code))
        } else if s.len() == 1 {
            Ok(KbShortcut::Key(KbKey::Character(s.to_string())))
        } else {
            Err(ParseShortcutError)
        }
    }
}

/// Return whether a given [`KeyEvent`] matches the code or character given in `str`
pub fn matches(key_event: &KeyEvent, str: &String) -> bool {
    if let Ok(shortcut) = KbShortcut::from_str(&str) {
        return shortcut.matches(key_event);
    }
    false
}
