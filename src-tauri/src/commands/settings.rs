use tauri::State;

use crate::db::Db;
use crate::error::AppResult;
use crate::services::settings;

#[tauri::command]
pub fn get_setting(db: State<'_, Db>, key: String) -> AppResult<Option<String>> {
    settings::get(&db.conn(), &key)
}

#[tauri::command]
pub fn set_setting(db: State<'_, Db>, key: String, value: String) -> AppResult<()> {
    settings::set(&db.conn(), &key, &value)
}
