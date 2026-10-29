//! Saving and reading files the user picked through the native dialogs
//! (reports, exported profiles).

use crate::error::{CmdError, CmdResult};
use base64::Engine;
use std::path::PathBuf;

const MAX_READ: u64 = 2 * 1024 * 1024;

fn checked(path: &str) -> CmdResult<PathBuf> {
    let p = PathBuf::from(path);
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(CmdError::invalid("choose a file location first"));
    }
    Ok(p)
}

#[tauri::command]
pub fn save_text_file(path: String, contents: String) -> CmdResult<()> {
    std::fs::write(checked(&path)?, contents).map_err(CmdError::io)
}

#[tauri::command]
pub fn save_binary_file(path: String, base64: String) -> CmdResult<()> {
    let data = base64::engine::general_purpose::STANDARD
        .decode(base64.trim())
        .map_err(|e| CmdError::invalid(format!("bad file data: {e}")))?;
    std::fs::write(checked(&path)?, data).map_err(CmdError::io)
}

#[tauri::command]
pub fn read_text_file(path: String) -> CmdResult<String> {
    let p = checked(&path)?;
    let meta = std::fs::metadata(&p).map_err(CmdError::io)?;
    if meta.len() > MAX_READ {
        return Err(CmdError::invalid("file too large"));
    }
    std::fs::read_to_string(p).map_err(CmdError::io)
}
