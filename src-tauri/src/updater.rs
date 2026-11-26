//! Automatic updates from GitHub Releases (DECISIONS.md D16).
//!
//! The app checks `latest.json` on the latest GitHub release shortly after
//! start and every 6 hours. If a newer version exists, the main window shows a
//! bar; "Update & restart" downloads it, verifies its signature against the
//! public key built into the app, installs it and restarts. Data is untouched.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::{AppError, AppResult};

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(10);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// Placeholder in tauri.conf.json until the maintainer adds the real key.
const KEY_PLACEHOLDER: &str = "REPLACE_WITH_UPDATER_PUBLIC_KEY";

#[derive(Default)]
pub struct PendingUpdate(pub Mutex<Option<Update>>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
}

/// Updates need a real signing key in the config; without one, stay quiet.
pub fn configured(app: &AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.is_empty() && k != KEY_PLACEHOLDER)
}

/// Background checks. Skipped in development builds and when not configured.
pub fn start(app: AppHandle) {
    if cfg!(debug_assertions) || !configured(&app) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio_sleep(FIRST_CHECK_DELAY).await;
        loop {
            if let Err(e) = check(&app).await {
                eprintln!("[updater] check failed: {e}");
            }
            tokio_sleep(CHECK_EVERY).await;
        }
    });
}

async fn tokio_sleep(d: Duration) {
    // tauri's async runtime is tokio; avoid a direct tokio dependency by
    // sleeping on a blocking-pool thread.
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

/// Checks once. Emits `update-available` and remembers the update if found.
pub async fn check(app: &AppHandle) -> AppResult<Option<UpdateInfo>> {
    if !configured(app) {
        return Err(AppError::Validation(
            "Updates aren't set up for this build".into(),
        ));
    }
    let updater = app
        .updater()
        .map_err(|e| AppError::Validation(format!("Updater unavailable: {e}")))?;
    let found = updater
        .check()
        .await
        .map_err(|e| AppError::Validation(format!("Couldn't check for updates: {e}")))?;
    let Some(update) = found else {
        return Ok(None);
    };
    let info = UpdateInfo {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone(),
    };
    *app.state::<PendingUpdate>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(update);
    let _ = app.emit("update-available", info.clone());
    Ok(Some(info))
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> AppResult<Option<UpdateInfo>> {
    check(&app).await
}

/// Downloads, verifies and installs the pending update, then restarts.
#[tauri::command]
pub async fn install_update(app: AppHandle, pending: State<'_, PendingUpdate>) -> AppResult<()> {
    let update = pending.0.lock().unwrap_or_else(|e| e.into_inner()).take();
    let Some(update) = update else {
        return Err(AppError::NotFound("update"));
    };
    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| AppError::Validation(format!("Update failed: {e}")))?;
    app.restart();
}
