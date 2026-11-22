//! Tray / menu-bar icon. Closing the main window only hides it, so the app (and
//! the reminder scheduler) keeps running here until "Quit" is chosen.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

use crate::windows;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Daily Tracker", true, None::<&str>)?;
    let reminders = MenuItem::with_id(app, "reminders", "Show reminders", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Daily Tracker", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &reminders, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Daily Tracker")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => windows::show_main(app),
            "reminders" => windows::show_reminder_popup(app, true),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
