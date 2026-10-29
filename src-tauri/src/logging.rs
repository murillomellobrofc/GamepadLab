//! Structured logging: JSON lines on disk, readable lines on stderr, and an
//! in-memory ring of recent warnings/errors for the diagnostics report.
//! The level can change at runtime from the settings screen.

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECENT: usize = 80;
const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;

pub struct Logger {
    file: Mutex<Option<File>>,
    recent: Mutex<VecDeque<String>>,
    path: Mutex<Option<PathBuf>>,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| Logger {
        file: Mutex::new(None),
        recent: Mutex::new(VecDeque::new()),
        path: Mutex::new(None),
    })
}

/// Installs the global logger. Safe to call once; later calls are ignored.
pub fn init(default: LevelFilter) {
    if log::set_logger(logger()).is_ok() {
        log::set_max_level(default);
    }
}

/// Starts writing JSON lines into `dir/gamepadlab.log` (rotated at 5 MB).
pub fn attach_file(dir: &Path) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let path = dir.join("gamepadlab.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_FILE_BYTES) {
        let _ = std::fs::rename(&path, dir.join("gamepadlab.1.log"));
    }
    if let Ok(f) = OpenOptions::new().create(true).append(true).open(&path) {
        *logger().file.lock().unwrap() = Some(f);
        *logger().path.lock().unwrap() = Some(path);
    }
}

pub fn set_level(level: &str) -> LevelFilter {
    let filter = match level.to_ascii_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };
    log::set_max_level(filter);
    filter
}

pub fn recent() -> Vec<String> {
    logger().recent.lock().unwrap().iter().cloned().collect()
}

pub fn log_path() -> Option<PathBuf> {
    logger().path.lock().unwrap().clone()
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
            && !(metadata.target().starts_with("tao")
                || metadata.target().starts_with("wry")
                || metadata.target().starts_with("tracing"))
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let msg = record.args().to_string();
        let line = format!(
            "{{\"ts\":{ts},\"level\":\"{}\",\"target\":\"{}\",\"msg\":\"{}\"}}",
            record.level(),
            escape(record.target()),
            escape(&msg)
        );
        if let Some(f) = self.file.lock().unwrap().as_mut() {
            let _ = writeln!(f, "{line}");
        }
        eprintln!("[{:5}] {}: {}", record.level(), record.target(), msg);
        if record.level() <= Level::Warn {
            let mut q = self.recent.lock().unwrap();
            if q.len() == MAX_RECENT {
                q.pop_front();
            }
            q.push_back(format!(
                "{ts} {} {}: {msg}",
                record.level(),
                record.target()
            ));
        }
    }

    fn flush(&self) {
        if let Some(f) = self.file.lock().unwrap().as_mut() {
            let _ = f.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_json() {
        assert_eq!(escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
        assert_eq!(escape("\u{1}"), "\\u0001");
    }

    #[test]
    fn parses_levels() {
        assert_eq!(set_level("debug"), LevelFilter::Debug);
        assert_eq!(set_level("nonsense"), LevelFilter::Info);
    }
}
