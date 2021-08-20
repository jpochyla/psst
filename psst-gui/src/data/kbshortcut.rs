use druid::{Data, Lens};
use druid_shell::{Code, KbKey, KeyEvent};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    pub fn matches(&self, event: &KeyEvent) -> bool {
        match self {
            KbShortcut::Key(key) => &event.key == key,
            KbShortcut::Code(code) => &event.code == code,
        }
    }
}

fn kb_code_from_str(s: &str) -> Result<Code, ()> {
    match s {
        "NumpadAdd" => Ok(Code::NumpadAdd),
        "Minus" => Ok(Code::Minus),
        " " | "Space" => Ok(Code::Space),
        "ArrowRight" => Ok(Code::ArrowRight),
        "ArrowLeft" => Ok(Code::ArrowLeft),
        "ArrowUp" => Ok(Code::ArrowUp),
        "ArrowDown" => Ok(Code::ArrowDown),
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
struct ParseShortcutError;

// Generation of an error is completely separate from how it is displayed.
// There's no need to be concerned about cluttering complex logic with the display style.
//
// Note that we don't store any extra info about the errors. This means we can't state
// which string failed to parse without modifying our types to carry that information.
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

struct ShortcutLens;

impl Lens<KbShortcut, String> for ShortcutLens {
    fn with<V, F: FnOnce(&String) -> V>(&self, data: &KbShortcut, f: F) -> V {
        f(&data.to_string())
    }

    fn with_mut<V, F: FnOnce(&mut String) -> V>(&self, data: &mut KbShortcut, f: F) -> V {
        let mut shortcut_as_string = &data.to_string();
        f(&mut shortcut_as_string)
    }
}

pub fn matches(key: KeyEvent, str: String) -> bool {
    // Make KbShortcut from str, match key.code and key.key to the result and return if it matched
    // on error return false?
}
