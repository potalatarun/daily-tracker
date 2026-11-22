//! Daily Tracker core. See docs/ARCHITECTURE.md for the overall design.

mod commands;
mod db;
mod error;
mod scheduler;
mod services;
mod time;
mod tray;
mod windows;

use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_notification::NotificationExt;

pub const DB_FILE: &str = "daily-tracker.sqlite";

pub fn run() {
    let app = tauri::Builder::default()
        // Must be registered first: a second launch focuses the running app
        // instead of starting a second scheduler (which would double-fire reminders).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_main(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .manage(scheduler::Alerts::default())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db = db::Db::open(&dir.join(DB_FILE))?;
            app.manage(db);

            let handle = app.handle().clone();

            // Clicking a reminder notification opens the reminder popup.
            let on_click = handle.clone();
            let _ = app.notification().on_action(move |action| {
                if action.action_id() == "tap" {
                    eprintln!("[notification] clicked: opening reminder popup");
                    windows::show_reminder_popup(&on_click, true);
                }
            });

            if let Err(e) = tray::create(&handle) {
                // No tray host (e.g. some Linux desktops): the app still works;
                // relaunching it shows the window again via single-instance.
                eprintln!("[tray] not available: {e}");
            }

            scheduler::start(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    // Closing the main window keeps the app running in the tray,
                    // so reminders keep working. Quit from the tray menu.
                    windows::MAIN => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    // Closing the reminder popup means "I've seen these".
                    windows::REMINDER => {
                        let app = window.app_handle();
                        app.state::<scheduler::Alerts>().lock().clear();
                        let _ = app.emit("alerts-changed", ());
                    }
                    _ => {}
                }
            }
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
            commands::timer::start_timer,
            commands::timer::pause_timer,
            commands::timer::get_active_timer,
            commands::timer::list_sessions,
            commands::timer::add_session,
            commands::timer::update_session,
            commands::timer::delete_session,
            commands::reminders::set_task_reminders,
            commands::reminders::get_alerts,
            commands::reminders::alert_done,
            commands::reminders::alert_snooze,
            commands::reminders::alert_dismiss,
            commands::reminders::dismiss_all_alerts,
            commands::reminders::show_alerts,
            commands::reminders::show_main_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Daily Tracker");

    app.run(|_app, _event| {
        // macOS: clicking the Dock icon while the window is hidden shows it again.
        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen { .. } = _event {
            windows::show_main(_app);
        }
        #[cfg(not(target_os = "macos"))]
        let _: &RunEvent = &_event;
    });
}
