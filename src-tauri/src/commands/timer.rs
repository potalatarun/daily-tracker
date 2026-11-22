use tauri::State;

use crate::db::Db;
use crate::error::AppResult;
use crate::services::timer::{self, ActiveTimer, Session, StartResult};
use crate::time::now_ms;

#[tauri::command]
pub fn start_timer(db: State<'_, Db>, task_id: String) -> AppResult<StartResult> {
    timer::start(&mut db.conn(), &task_id, now_ms())
}

#[tauri::command]
pub fn pause_timer(db: State<'_, Db>, task_id: String) -> AppResult<()> {
    timer::pause(&db.conn(), &task_id, now_ms())
}

#[tauri::command]
pub fn get_active_timer(db: State<'_, Db>) -> AppResult<Option<ActiveTimer>> {
    timer::active(&db.conn())
}

#[tauri::command]
pub fn list_sessions(db: State<'_, Db>, task_id: String) -> AppResult<Vec<Session>> {
    timer::list(&db.conn(), &task_id)
}

#[tauri::command]
pub fn add_session(
    db: State<'_, Db>,
    task_id: String,
    started_at: i64,
    ended_at: i64,
) -> AppResult<Session> {
    timer::add(&db.conn(), &task_id, started_at, ended_at, now_ms())
}

#[tauri::command]
pub fn update_session(
    db: State<'_, Db>,
    id: String,
    started_at: i64,
    ended_at: Option<i64>,
) -> AppResult<Session> {
    timer::update(&db.conn(), &id, started_at, ended_at, now_ms())
}

#[tauri::command]
pub fn delete_session(db: State<'_, Db>, id: String) -> AppResult<()> {
    timer::delete(&db.conn(), &id)
}
