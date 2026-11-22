use tauri::State;

use crate::db::Db;
use crate::error::AppResult;
use crate::services::workspaces::{self, Workspace};

#[tauri::command]
pub fn list_workspaces(db: State<'_, Db>) -> AppResult<Vec<Workspace>> {
    workspaces::list(&db.conn())
}

#[tauri::command]
pub fn create_workspace(db: State<'_, Db>, name: String, color: String) -> AppResult<Workspace> {
    workspaces::create(&db.conn(), &name, &color)
}

#[tauri::command]
pub fn update_workspace(
    db: State<'_, Db>,
    id: String,
    name: Option<String>,
    color: Option<String>,
) -> AppResult<Workspace> {
    workspaces::update(&db.conn(), &id, name.as_deref(), color.as_deref())
}

#[tauri::command]
pub fn reorder_workspaces(db: State<'_, Db>, ids: Vec<String>) -> AppResult<Vec<Workspace>> {
    workspaces::reorder(&mut db.conn(), &ids)
}

#[tauri::command]
pub fn delete_workspace(db: State<'_, Db>, id: String) -> AppResult<()> {
    workspaces::delete(&mut db.conn(), &id)
}
