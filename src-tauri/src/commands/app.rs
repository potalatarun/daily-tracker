//! App-level commands: settings that touch the OS (start at login, data
//! folder), test notification, export / import, and deleting all data.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::scheduler::Alerts;
use crate::services::backup::{self, ImportSummary};
use crate::windows;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub start_at_login: bool,
}

fn data_dir(db: &Db) -> AppResult<std::path::PathBuf> {
    db.data_dir()
        .map(|p| p.to_path_buf())
        .ok_or(AppError::NotFound("data folder"))
}

#[tauri::command]
pub fn get_app_info(app: AppHandle, db: State<'_, Db>) -> AppResult<AppInfo> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: data_dir(&db)?.to_string_lossy().into_owned(),
        start_at_login: app.autolaunch().is_enabled().unwrap_or(false),
    })
}

#[tauri::command]
pub fn set_start_at_login(app: AppHandle, enabled: bool) -> AppResult<bool> {
    let al = app.autolaunch();
    let res = if enabled { al.enable() } else { al.disable() };
    res.map_err(|e| AppError::Validation(format!("Couldn't change start at login: {e}")))?;
    Ok(al.is_enabled().unwrap_or(enabled))
}

#[tauri::command]
pub fn open_data_folder(app: AppHandle, db: State<'_, Db>) -> AppResult<()> {
    let dir = data_dir(&db)?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Validation(format!("Couldn't open the folder: {e}")))
}

#[tauri::command]
pub fn send_test_notification(app: AppHandle) -> AppResult<()> {
    app.notification()
        .builder()
        .title("Daily Tracker")
        .body("Notifications are working. Reminders will look like this.")
        .show()
        .map_err(|e| AppError::Validation(format!("Couldn't send a notification: {e}")))
}

/// Asks where to save and writes the export. Returns the saved path, or None
/// if the user cancelled. Async so the blocking dialog runs off the main thread.
#[tauri::command]
pub async fn export_data(app: AppHandle) -> AppResult<Option<String>> {
    let date = chrono::Local::now().format("%Y-%m-%d");
    let Some(path) = app
        .dialog()
        .file()
        .set_title("Export Daily Tracker data")
        .set_file_name(format!("daily-tracker-{date}.json"))
        .add_filter("Daily Tracker export", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|e| AppError::Validation(format!("Can't save there: {e}")))?;
    let export = {
        let db = app.state::<Db>();
        let conn = db.conn();
        backup::export(&conn, &app.package_info().version.to_string())?
    };
    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| AppError::Validation(format!("Couldn't create the export: {e}")))?;
    std::fs::write(&path, json)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Asks for an export file and replaces all data with it. A snapshot of the
/// current database is saved to `backups/` first. None if cancelled.
#[tauri::command]
pub async fn import_data(app: AppHandle) -> AppResult<Option<ImportSummary>> {
    let Some(path) = app
        .dialog()
        .file()
        .set_title("Import Daily Tracker data")
        .add_filter("Daily Tracker export", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|e| AppError::Validation(format!("Can't open that file: {e}")))?;
    let text = std::fs::read_to_string(&path)?;
    let data = backup::parse(&text)?;

    let summary = {
        let db = app.state::<Db>();
        let mut conn = db.conn();
        if let Some(dir) = db.data_dir() {
            backup::snapshot(&conn, &dir.join("backups"), "before-import")?;
        }
        backup::import(&mut conn, &data)?
    };
    reset_ui(&app);
    Ok(Some(summary))
}

/// Deletes every task, workspace, note and setting, plus the automatic backups.
#[tauri::command]
pub fn delete_all_data(app: AppHandle, db: State<'_, Db>) -> AppResult<()> {
    backup::wipe(&mut db.conn())?;
    if let Some(dir) = db.data_dir() {
        let backups = dir.join("backups");
        if backups.exists() {
            std::fs::remove_dir_all(backups)?;
        }
    }
    let _ = app.autolaunch().disable();
    reset_ui(&app);
    Ok(())
}

/// After data was replaced: drop pending alerts and tell every window to reload.
fn reset_ui(app: &AppHandle) {
    app.state::<Alerts>().lock().clear();
    windows::close_reminder_popup(app);
    let _ = app.emit("data-replaced", ());
}
