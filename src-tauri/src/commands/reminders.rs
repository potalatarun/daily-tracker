use tauri::{AppHandle, Emitter, State};

use crate::db::Db;
use crate::error::AppResult;
use crate::scheduler::{Alert, Alerts};
use crate::services::reminders::{self, Reminder};
use crate::services::tasks;
use crate::time::{now_hhmm, now_ms, today};
use crate::windows;

#[tauri::command]
pub fn set_task_reminders(
    db: State<'_, Db>,
    task_id: String,
    times: Vec<String>,
) -> AppResult<Vec<Reminder>> {
    reminders::set_for_task(&mut db.conn(), &task_id, &times, today(), &now_hhmm())
}

/// Alerts waiting in the popup, minus any whose task was finished or deleted
/// in the meantime.
#[tauri::command]
pub fn get_alerts(db: State<'_, Db>, alerts: State<'_, Alerts>) -> AppResult<Vec<Alert>> {
    let conn = db.conn();
    let mut list = alerts.lock();
    list.retain(|a| {
        tasks::get(&conn, &a.task_id)
            .map(|t| t.status == "pending")
            .unwrap_or(false)
    });
    Ok(list.clone())
}

#[tauri::command]
pub fn alert_done(
    app: AppHandle,
    db: State<'_, Db>,
    alerts: State<'_, Alerts>,
    task_id: String,
) -> AppResult<()> {
    tasks::complete(&mut db.conn(), &task_id, today())?;
    alerts.lock().retain(|a| a.task_id != task_id);
    after_change(&app, &alerts);
    Ok(())
}

#[tauri::command]
pub fn alert_snooze(
    app: AppHandle,
    db: State<'_, Db>,
    alerts: State<'_, Alerts>,
    reminder_id: String,
    until: i64,
) -> AppResult<()> {
    reminders::snooze(&db.conn(), &reminder_id, until, now_ms())?;
    alerts.lock().retain(|a| a.reminder_id != reminder_id);
    after_change(&app, &alerts);
    Ok(())
}

#[tauri::command]
pub fn alert_dismiss(
    app: AppHandle,
    alerts: State<'_, Alerts>,
    reminder_id: String,
) -> AppResult<()> {
    alerts.lock().retain(|a| a.reminder_id != reminder_id);
    after_change(&app, &alerts);
    Ok(())
}

#[tauri::command]
pub fn dismiss_all_alerts(app: AppHandle, alerts: State<'_, Alerts>) -> AppResult<()> {
    alerts.lock().clear();
    after_change(&app, &alerts);
    Ok(())
}

#[tauri::command]
pub fn show_alerts(app: AppHandle) {
    windows::show_reminder_popup(&app, true);
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    windows::show_main(&app);
}

fn after_change(app: &AppHandle, alerts: &Alerts) {
    let _ = app.emit("alerts-changed", ());
    let _ = app.emit("data-changed", ());
    if alerts.lock().is_empty() {
        windows::close_reminder_popup(app);
    }
}
