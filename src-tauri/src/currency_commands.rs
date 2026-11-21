//! Optional currency-network boundary. All parsing/conversion stays in core.
use crate::error::ToolError;
use oxbox_core::{
    CoreError,
    tools::currency::{self, RateBook},
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
// The repository requires a public issue before runtime network access is enabled.
pub const NETWORK_ISSUE: Option<&str> = Some("https://github.com/ninad-k/oxbox/issues/1");
static CACHE_LOCK: Mutex<()> = Mutex::new(());
fn cache_lock() -> Result<std::sync::MutexGuard<'static, ()>, ToolError> {
    CACHE_LOCK
        .lock()
        .map_err(|_| ToolError::internal("Currency cache is unavailable. Restart Oxbox.".into()))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyStatus {
    network_enabled: bool,
    network_issue: Option<&'static str>,
    book: Option<RateBook>,
}
fn store(
    app: &AppHandle,
) -> Result<std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>, ToolError> {
    app.store_builder("currency.json")
        .disable_auto_save()
        .build()
        .map_err(|e| ToolError::internal(e.to_string()))
}
fn cached(app: &AppHandle) -> Result<Option<RateBook>, ToolError> {
    let _guard = cache_lock()?;
    Ok(store(app)?
        .get("currencyRates")
        .and_then(|v| serde_json::from_value(v).ok()))
}
fn save(app: &AppHandle, book: &RateBook) -> Result<(), ToolError> {
    write_key(
        app,
        "currencyRates",
        serde_json::to_value(book).map_err(|e| ToolError::internal(e.to_string()))?,
    )
}
fn write_key(app: &AppHandle, key: &str, value: serde_json::Value) -> Result<(), ToolError> {
    let _guard = cache_lock()?;
    let store = store(app)?;
    let previous = store.get(key);
    store.set(key, value);
    if let Err(error) = store.save() {
        if let Some(previous) = previous {
            store.set(key, previous);
        } else {
            store.delete(key);
        }
        return Err(ToolError::internal(error.to_string()));
    }
    Ok(())
}
#[tauri::command]
pub fn currency_status(app: AppHandle) -> Result<CurrencyStatus, ToolError> {
    let _guard = cache_lock()?;
    let store = store(&app)?;
    Ok(CurrencyStatus {
        network_enabled: NETWORK_ISSUE.is_some()
            && store
                .get("currencyNetwork")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        network_issue: NETWORK_ISSUE,
        book: store
            .get("currencyRates")
            .and_then(|value| serde_json::from_value(value).ok()),
    })
}
#[tauri::command]
pub fn currency_set_network(app: AppHandle, enabled: bool) -> Result<(), ToolError> {
    if enabled && NETWORK_ISSUE.is_none() {
        return Err(CoreError::InvalidInput("Network access awaits the repository's required public issue; file import is available.".into()).into());
    }
    write_key(&app, "currencyNetwork", serde_json::json!(enabled))
}
#[tauri::command]
pub async fn currency_import(app: AppHandle, path: PathBuf) -> Result<RateBook, ToolError> {
    let book = super::commands::run_blocking(move || currency::import(&path, chrono_now())).await?;
    save(&app, &book)?;
    Ok(book)
}
fn chrono_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |v| v.as_secs())
}
#[tauri::command]
pub async fn currency_refresh(app: AppHandle, force: bool) -> Result<RateBook, ToolError> {
    let status = currency_status(app.clone())?;
    if !status.network_enabled {
        return Err(CoreError::InvalidInput("ECB network access is off.".into()).into());
    }
    if !force
        && let Some(book) = status.book
        && !currency::needs_refresh(book.fetched_at, chrono_now())
    {
        return Ok(book);
    }
    let book = super::commands::run_blocking(|| currency::download(true)).await?;
    save(&app, &book)?;
    Ok(book)
}
#[tauri::command]
pub fn currency_convert(
    app: AppHandle,
    amount: f64,
    from: String,
    to: String,
) -> Result<f64, ToolError> {
    let book = cached(&app)?
        .ok_or_else(|| CoreError::InvalidInput("import or refresh a rate table first".into()))?;
    Ok(currency::convert(amount, &from, &to, &book)?)
}
