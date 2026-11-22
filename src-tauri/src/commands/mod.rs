//! Thin `#[tauri::command]` handlers the UI calls via `invoke()`.
//! They only unpack arguments, lock the DB and call a service.

pub mod settings;
pub mod tasks;
pub mod timer;
pub mod workspaces;
