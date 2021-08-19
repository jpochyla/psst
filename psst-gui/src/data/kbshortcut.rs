use druid::Data;
use druid_shell::{Code, KbKey, KeyEvent};
use serde::{Deserialize, Serialize};

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

pub trait ToKbShortcut {
    fn to_kbshortcut(&self) -> Result<KbShortcut, ()>;
}

impl ToKbShortcut for String {
    fn to_kbshortcut(&self) -> Result<KbShortcut, ()> {
        if let Ok(code) = kb_code_from_str(&self) {
            Ok(KbShortcut::Code(code))
        } else if self.len() == 1 {
            Ok(KbShortcut::Key(KbKey::Character(self.clone())))
        } else {
            Err(())
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
