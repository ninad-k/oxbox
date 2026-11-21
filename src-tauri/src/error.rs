use oxbox_core::CoreError;
use serde::Serialize;

/// The error shape every command returns to the UI: `{ code, message }`.
/// Mirrored by `ToolError` in `src/lib/api.ts`.
#[derive(Debug, Serialize)]
pub struct ToolError {
    code: &'static str,
    message: String,
}

impl ToolError {
    pub fn internal(message: String) -> Self {
        Self {
            code: "internal",
            message,
        }
    }
}

impl From<CoreError> for ToolError {
    fn from(err: CoreError) -> Self {
        Self {
            code: err.code(),
            message: err.to_string(),
        }
    }
}
