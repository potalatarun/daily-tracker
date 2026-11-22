//! Showing and hiding the app's windows.

use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder};

pub const MAIN: &str = "main";
pub const REMINDER: &str = "reminder";

const POPUP_W: f64 = 380.0;
const POPUP_H: f64 = 460.0;
const POPUP_MARGIN: f64 = 24.0;

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Opens the always-on-top reminder popup near the top-right of the screen.
/// `focus` is true when the user asked for it (clicked a notification).
pub fn show_reminder_popup(app: &AppHandle, focus: bool) {
    if let Some(w) = app.get_webview_window(REMINDER) {
        let _ = w.show();
        let _ = w.set_always_on_top(true);
        if focus {
            let _ = w.set_focus();
        }
        return;
    }

    let mut builder =
        WebviewWindowBuilder::new(app, REMINDER, WebviewUrl::App("popup.html".into()))
            .title("Daily Tracker · Reminder")
            .inner_size(POPUP_W, POPUP_H)
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            .always_on_top(true)
            .focused(focus)
            .disable_drag_drop_handler();

    if let Ok(Some(m)) = app.primary_monitor() {
        let scale = m.scale_factor();
        let size = m.size().to_logical::<f64>(scale);
        let pos = m.position().to_logical::<f64>(scale);
        builder = builder.position(
            pos.x + size.width - POPUP_W - POPUP_MARGIN,
            pos.y + POPUP_MARGIN * 2.0,
        );
    }

    match builder.build() {
        Ok(w) => {
            // Keep it on screen even if the monitor query was off.
            if let Ok(p) = w.outer_position() {
                if p.x < 0 || p.y < 0 {
                    let _ = w.set_position(LogicalPosition::new(POPUP_MARGIN, POPUP_MARGIN));
                }
            }
        }
        Err(e) => eprintln!("[windows] could not open reminder popup: {e}"),
    }
}

pub fn close_reminder_popup(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(REMINDER) {
        let _ = w.destroy();
    }
}
