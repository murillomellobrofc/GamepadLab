use crate::error::{CmdError, CmdResult};
use crate::history::{NewSession, Session};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

fn db(state: &AppState) -> CmdResult<&crate::history::History> {
    state
        .history
        .as_ref()
        .ok_or_else(|| CmdError::new("io", "the history database could not be opened"))
}

#[tauri::command]
pub fn history_add(state: St, session: NewSession) -> CmdResult<i64> {
    db(&state)?.add(&session).map_err(CmdError::invalid)
}

#[tauri::command]
pub fn history_list(state: St, limit: u32) -> CmdResult<Vec<Session>> {
    db(&state)?.list(limit).map_err(CmdError::io)
}

#[tauri::command]
pub fn history_delete(state: St, id: i64) -> CmdResult<()> {
    if db(&state)?.delete(id).map_err(CmdError::io)? {
        Ok(())
    } else {
        Err(CmdError::not_found(format!("session {id}")))
    }
}

#[tauri::command]
pub fn history_clear(state: St) -> CmdResult<()> {
    db(&state)?.clear().map_err(CmdError::io)
}
