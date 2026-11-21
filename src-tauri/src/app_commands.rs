//! App preferences and small, offline Phase 1 tool adapters.
use crate::error::ToolError;
use oxbox_core::{
    CoreError,
    tools::{qr, unit_convert, world_time},
};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_store::StoreExt;

#[derive(Debug, Clone, Serialize)]
pub struct ShortcutStatus {
    pub shortcut: String,
    pub error: Option<String>,
}
struct ShortcutRegistration {
    status: ShortcutStatus,
    active: bool,
}
pub struct ShortcutState(Mutex<ShortcutRegistration>);
pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let store = app
        .store_builder("settings.json")
        .disable_auto_save()
        .build();
    let storage_error = store
        .as_ref()
        .err()
        .map(|e| format!("Could not load shortcut preferences: {e}"));
    let shortcut = store
        .ok()
        .and_then(|s| s.get("shortcut"))
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "CommandOrControl+Alt+Space".into());
    let result = shortcut
        .parse::<Shortcut>()
        .map_err(|e| e.to_string())
        .and_then(|s| app.global_shortcut().register(s).map_err(|e| e.to_string()));
    let active = result.is_ok();
    app.manage(ShortcutState(Mutex::new(ShortcutRegistration {
        status: ShortcutStatus {
            shortcut,
            error: result.err().or(storage_error),
        },
        active,
    })));
    Ok(())
}
#[tauri::command]
pub fn shortcut_status(state: State<'_, ShortcutState>) -> Result<ShortcutStatus, ToolError> {
    state
        .0
        .lock()
        .map(|s| s.status.clone())
        .map_err(|_| ToolError::internal("Shortcut state unavailable".into()))
}
#[tauri::command]
pub fn configure_shortcut(
    app: AppHandle,
    shortcut: String,
    state: State<'_, ShortcutState>,
) -> Result<String, ToolError> {
    let shortcut = shortcut.trim().to_string();
    if shortcut.len() > 100 {
        return Err(CoreError::InvalidInput("shortcut is too long".into()).into());
    }
    let new = shortcut
        .parse::<Shortcut>()
        .map_err(|_| CoreError::InvalidInput("invalid shortcut; use modifiers and a key".into()))?;
    let mut current = state
        .0
        .lock()
        .map_err(|_| ToolError::internal("Shortcut state unavailable".into()))?;
    let old = current
        .status
        .shortcut
        .parse::<Shortcut>()
        .ok()
        .filter(|_| current.active);
    let same_active = Some(new) == old;
    if same_active && current.status.error.is_none() {
        return Ok(current.status.shortcut.clone());
    }
    let store = app
        .store("settings.json")
        .map_err(|e| ToolError::internal(e.to_string()))?;
    if !same_active {
        app.global_shortcut().register(new).map_err(|e| {
            CoreError::InvalidInput(format!("shortcut could not be registered: {e}"))
        })?;
        if let Some(old) = old
            && let Err(error) = app.global_shortcut().unregister(old)
        {
            let rollback = app.global_shortcut().unregister(new);
            if let Err(rollback) = rollback {
                current.status.error = Some(format!(
                    "Could not restore shortcut registrations: {rollback}. Restart Oxbox."
                ));
            }
            return Err(ToolError::internal(error.to_string()));
        }
    }
    let previous = store.get("shortcut");
    store.set("shortcut", serde_json::json!(shortcut));
    if let Err(e) = store.save() {
        let mut recovery_errors = Vec::new();
        if !same_active {
            if let Err(error) = app.global_shortcut().unregister(new) {
                recovery_errors.push(error.to_string());
            }
            current.active = false;
            if let Some(old) = old {
                match app.global_shortcut().register(old) {
                    Ok(()) => current.active = true,
                    Err(error) => recovery_errors.push(error.to_string()),
                }
            }
        }
        if let Some(previous) = previous {
            store.set("shortcut", previous);
        } else {
            store.delete("shortcut");
        }
        if !recovery_errors.is_empty() {
            current.status.error = Some(format!(
                "Shortcut recovery failed: {}. Restart Oxbox.",
                recovery_errors.join("; ")
            ));
            return Err(ToolError::internal(format!(
                "{e}. {}",
                current
                    .status
                    .error
                    .as_deref()
                    .unwrap_or("Shortcut recovery failed.")
            )));
        }
        return Err(ToolError::internal(e.to_string()));
    }
    *current = ShortcutRegistration {
        status: ShortcutStatus {
            shortcut: shortcut.clone(),
            error: None,
        },
        active: true,
    };
    Ok(shortcut)
}
#[tauri::command]
pub fn conversion_units() -> &'static [unit_convert::Unit] {
    unit_convert::units()
}
#[tauri::command]
pub fn convert_unit(value: f64, from: String, to: String) -> Result<f64, ToolError> {
    Ok(unit_convert::convert(value, &from, &to)?)
}
#[tauri::command]
pub fn time_zones() -> Vec<&'static str> {
    world_time::zones()
}
#[tauri::command]
pub fn world_clocks(zones: Vec<String>) -> Result<Vec<world_time::Clock>, ToolError> {
    if zones.len() > 200 {
        return Err(CoreError::InvalidInput("at most 200 clocks are allowed".into()).into());
    }
    zones
        .iter()
        .map(|z| world_time::now(z).map_err(Into::into))
        .collect()
}
#[tauri::command]
pub async fn generate_code(
    text: String,
    kind: qr::CodeKind,
    correction: String,
    size: u32,
) -> Result<String, ToolError> {
    super::commands::run_blocking(move || Ok(qr::generate(&text, kind, &correction, size)?.svg))
        .await
}
#[tauri::command]
pub async fn save_code(
    text: String,
    kind: qr::CodeKind,
    correction: String,
    size: u32,
    path: PathBuf,
    svg: bool,
) -> Result<(), ToolError> {
    super::commands::run_blocking(move || {
        qr::save(&qr::generate(&text, kind, &correction, size)?, &path, svg)
    })
    .await
}
#[tauri::command]
pub async fn copy_code(
    app: AppHandle,
    text: String,
    kind: qr::CodeKind,
    correction: String,
    size: u32,
) -> Result<(), ToolError> {
    super::commands::run_blocking(move || {
        app.state::<crate::media_commands::MediaState>()
            .clipboard
            .copy_image(&qr::decode_image(&qr::generate(
                &text,
                kind,
                &correction,
                size,
            )?)?)
    })
    .await
}
