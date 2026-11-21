//! `#[tauri::command]` wrappers. Each one is a thin adapter over `oxbox-core`:
//! validate input size, move blocking work off the async runtime, map errors.
//!
//! Adding a tool? Add its command here, register it in `lib.rs` (`generate_handler!`)
//! and add a typed wrapper in `src/lib/api.ts`.

use std::path::PathBuf;

use oxbox_core::platform::Os;
use oxbox_core::tools::{checksum, text_transform};
use oxbox_core::{CoreError, CoreResult};
use serde::Serialize;

use crate::error::ToolError;

/// Upper bound for text sent over IPC to text tools.
const MAX_TEXT_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: &'static str,
    os: Os,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        os: Os::current(),
    }
}

#[tauri::command]
pub fn transform_text(text: String, case: text_transform::TextCase) -> Result<String, ToolError> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(CoreError::InvalidInput("text is larger than 5 MB".into()).into());
    }
    Ok(text_transform::transform(&text, case))
}

#[tauri::command]
pub async fn checksum_file(
    path: PathBuf,
    algorithm: checksum::Algorithm,
) -> Result<String, ToolError> {
    run_blocking(move || checksum::hash_file(&path, algorithm)).await
}

/// Run CPU- or disk-heavy work on Tauri's blocking pool so the UI stays responsive.
pub(crate) async fn run_blocking<T, F>(work: F) -> Result<T, ToolError>
where
    F: FnOnce() -> CoreResult<T> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| ToolError::internal(e.to_string()))?
        .map_err(ToolError::from)
}
