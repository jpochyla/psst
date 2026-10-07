use crate::data::Config;
use log::{Level, Log, Metadata, Record};
use parking_lot::Mutex;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
};

const LIMIT: u64 = 2 * 1024 * 1024;
static FILE_LOCK: Mutex<()> = Mutex::new(());
static REDACT: OnceLock<Vec<regex::Regex>> = OnceLock::new();

pub fn directory() -> Option<PathBuf> {
    Config::config_dir().map(|p| p.join("logs"))
}
fn sanitize(message: &str) -> String {
    let patterns = REDACT.get_or_init(|| vec![
        regex::Regex::new(r"(?i)bearer\s+[^\s,;]+" ).unwrap(),
        regex::Regex::new(r#"(?i)(?:access_token|refresh_token|auth_data|password|client_secret|api_key|api_secret|session_key|authorization)["']?\s*[=:]\s*(?:"[^"]*"|'[^']*'|\[[^\]]*\]|[^\s,;}]+)"#).unwrap(),
        regex::Regex::new(r"https?://[^\s]+" ).unwrap(),
    ]);
    patterns.iter().fold(message.to_owned(), |text, pattern| {
        pattern.replace_all(&text, "[REDACTED]").into_owned()
    })
}
struct Diagnostics {
    console: env_logger::Logger,
}
impl Log for Diagnostics {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info || self.console.enabled(metadata)
    }
    fn log(&self, record: &Record) {
        self.console.log(record);
        if record.level() > Level::Info {
            return;
        }
        let message = sanitize(&record.args().to_string());
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let line = format!(
            "{timestamp} {} [{}] {}\n",
            record.level(),
            record.target(),
            message
        );
        let _guard = FILE_LOCK.lock();
        let result = (|| -> std::io::Result<()> {
            let dir = directory().ok_or_else(|| std::io::Error::other("No log directory"))?;
            fs::create_dir_all(&dir)?;
            let path = dir.join("xpotify.log");
            if fs::metadata(&path).is_ok_and(|m| m.len() >= LIMIT) {
                let oldest = dir.join("xpotify.2.log");
                if oldest.exists() {
                    fs::remove_file(&oldest)?;
                }
                let previous = dir.join("xpotify.1.log");
                if previous.exists() {
                    fs::rename(&previous, oldest)?;
                }
                fs::rename(&path, previous)?;
            }
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            // Bound even a single unusually large server response.
            let end = line
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|i| *i <= 32 * 1024)
                .last()
                .unwrap_or(0);
            if line.len() > 32 * 1024 {
                writeln!(file, "{} [truncated]", &line[..end])?;
            } else {
                file.write_all(line.as_bytes())?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("Cannot persist diagnostics: {error}");
        }
    }
    fn flush(&self) {
        self.console.flush();
    }
}
pub fn init() {
    let console = env_logger::Builder::from_env(
        env_logger::Env::new()
            .filter_or("PSST_LOG", "info")
            .write_style("PSST_LOG_STYLE"),
    )
    .build();
    let level = console.filter().max(log::LevelFilter::Info);
    log::set_boxed_logger(Box::new(Diagnostics { console })).expect("Install logger");
    log::set_max_level(level);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("Application panic: {info}");
        previous(info);
    }));
    log::info!(
        "Xpotify {} ({}) started on {}",
        env!("CARGO_PKG_VERSION"),
        psst_core::GIT_VERSION,
        std::env::consts::OS
    );
}
pub fn preview() -> String {
    let _guard = FILE_LOCK.lock();
    let result = (|| -> std::io::Result<String> {
        let mut file = fs::File::open(directory().unwrap_or_default().join("xpotify.log"))?;
        let start = file.metadata()?.len().saturating_sub(64 * 1024);
        file.seek(SeekFrom::Start(start))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        Ok(if start > 0 {
            text.split_once('\n')
                .map_or(String::new(), |(_, tail)| tail.to_owned())
        } else {
            text
        })
    })();
    result.unwrap_or_else(|e| format!("No se pudieron leer los logs: {e}"))
}
pub fn export(path: &Path) -> std::io::Result<()> {
    let _guard = FILE_LOCK.lock();
    let dir = directory().ok_or_else(|| std::io::Error::other("No log directory"))?;
    fs::create_dir_all(&dir)?;
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Invalid destination"))?;
    if fs::canonicalize(parent)? == fs::canonicalize(&dir)? {
        return Err(std::io::Error::other(
            "Elige una carpeta distinta a la de logs",
        ));
    }
    let mut output = fs::File::create(path)?;
    writeln!(
        output,
        "Xpotify {} | {} | {}\n",
        env!("CARGO_PKG_VERSION"),
        psst_core::GIT_VERSION,
        std::env::consts::OS
    )?;
    for name in ["xpotify.2.log", "xpotify.1.log", "xpotify.log"] {
        match fs::read(dir.join(name)) {
            Ok(bytes) => output.write_all(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    output.flush()
}
#[cfg(test)]
mod tests {
    #[test]
    fn removes_credentials_and_signed_urls() {
        let result = super::sanitize(
            r#"Bearer secret access_token=abc refresh_token=def https://cdn.example/song?token=ghi password=xyz "client_secret":"private""#,
        );
        for secret in ["secret", "abc", "def", "ghi", "xyz", "private"] {
            assert!(!result.contains(secret));
        }
    }
}
