//! Error type returned by every command. Serialised as `{ code, message }` so
//! the UI can pick a localised, actionable explanation by code.

use gamepadlab_input::InputError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CmdError {
    pub code: &'static str,
    pub message: String,
}

#[allow(dead_code)] // the full set is used as more commands land
impl CmdError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn not_found(what: impl Into<String>) -> Self {
        Self::new("notFound", what)
    }
    pub fn invalid(what: impl Into<String>) -> Self {
        Self::new("invalidInput", what)
    }
    pub fn io(e: impl std::fmt::Display) -> Self {
        Self::new("io", e.to_string())
    }
    pub fn internal(e: impl std::fmt::Display) -> Self {
        Self::new("internal", e.to_string())
    }
}

impl From<InputError> for CmdError {
    fn from(e: InputError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl std::fmt::Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

pub type CmdResult<T> = Result<T, CmdError>;
