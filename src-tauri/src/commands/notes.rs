use tauri::State;

use crate::db::Db;
use crate::error::AppResult;
use crate::services::notes::{self, Note};

#[tauri::command]
pub fn list_notes(db: State<'_, Db>, workspace_id: String) -> AppResult<Vec<Note>> {
    notes::list(&db.conn(), &workspace_id)
}

#[tauri::command]
pub fn create_note(
    db: State<'_, Db>,
    workspace_id: String,
    label: String,
    value: String,
    note: String,
) -> AppResult<Note> {
    notes::create(&db.conn(), &workspace_id, &label, &value, &note)
}

#[tauri::command]
pub fn update_note(
    db: State<'_, Db>,
    id: String,
    label: Option<String>,
    value: Option<String>,
    note: Option<String>,
) -> AppResult<Note> {
    notes::update(
        &db.conn(),
        &id,
        label.as_deref(),
        value.as_deref(),
        note.as_deref(),
    )
}

#[tauri::command]
pub fn delete_note(db: State<'_, Db>, id: String) -> AppResult<()> {
    notes::delete(&db.conn(), &id)
}

#[tauri::command]
pub fn restore_note(db: State<'_, Db>, id: String) -> AppResult<Note> {
    notes::restore(&db.conn(), &id)
}

#[tauri::command]
pub fn reorder_notes(
    db: State<'_, Db>,
    workspace_id: String,
    ids: Vec<String>,
) -> AppResult<Vec<Note>> {
    notes::reorder(&mut db.conn(), &workspace_id, &ids)
}
