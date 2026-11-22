use tauri::State;

use crate::db::Db;
use crate::error::AppResult;
use crate::services::tasks::{self, DayView, MonthSummary, Task};
use crate::time::{fmt_date, today};

/// The core's local date. The UI asks for this instead of reading its own clock,
/// so there is exactly one notion of "today" (ARCHITECTURE.md §2).
#[tauri::command]
pub fn get_today() -> String {
    fmt_date(today())
}

#[tauri::command]
pub fn get_day(db: State<'_, Db>, workspace_id: String, date: String) -> AppResult<DayView> {
    tasks::day_view(&db.conn(), &workspace_id, &date, today())
}

#[tauri::command]
pub fn get_month_summary(
    db: State<'_, Db>,
    workspace_id: String,
    from: String,
    to: String,
) -> AppResult<MonthSummary> {
    tasks::month_summary(&db.conn(), &workspace_id, &from, &to, today())
}

#[tauri::command]
pub fn create_task(
    db: State<'_, Db>,
    workspace_id: String,
    date: String,
    title: String,
    description: String,
) -> AppResult<Task> {
    tasks::create(&db.conn(), &workspace_id, &date, &title, &description)
}

#[tauri::command]
pub fn update_task(
    db: State<'_, Db>,
    id: String,
    title: Option<String>,
    description: Option<String>,
    date: Option<String>,
) -> AppResult<Task> {
    tasks::update(
        &db.conn(),
        &id,
        title.as_deref(),
        description.as_deref(),
        date.as_deref(),
    )
}

#[tauri::command]
pub fn complete_task(db: State<'_, Db>, id: String) -> AppResult<Task> {
    tasks::complete(&mut db.conn(), &id, today())
}

#[tauri::command]
pub fn reopen_task(db: State<'_, Db>, id: String) -> AppResult<Task> {
    tasks::reopen(&db.conn(), &id)
}

#[tauri::command]
pub fn delete_task(db: State<'_, Db>, id: String) -> AppResult<()> {
    tasks::delete(&mut db.conn(), &id)
}

#[tauri::command]
pub fn restore_task(db: State<'_, Db>, id: String) -> AppResult<Task> {
    tasks::restore(&db.conn(), &id)
}
