//! Daily Tracker core. See docs/ARCHITECTURE.md for the overall design.

mod commands;
mod db;
mod error;
mod services;
mod time;

use tauri::Manager;

pub const DB_FILE: &str = "daily-tracker.sqlite";

pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch focuses the running app
        // instead of starting a second scheduler (which would double-fire reminders).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db = db::Db::open(&dir.join(DB_FILE))?;
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::workspaces::list_workspaces,
            commands::workspaces::create_workspace,
            commands::workspaces::update_workspace,
            commands::workspaces::reorder_workspaces,
            commands::workspaces::delete_workspace,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::tasks::get_today,
            commands::tasks::get_day,
            commands::tasks::get_month_summary,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::complete_task,
            commands::tasks::reopen_task,
            commands::tasks::delete_task,
            commands::tasks::restore_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Daily Tracker");
}
