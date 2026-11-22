//! The reminder scheduler (ARCHITECTURE.md §5.3, §5.4).
//!
//! A background thread wakes on every :00 and :30 second mark, asks the
//! database which reminders are due, records them as fired *before* notifying
//! (so a crash can't cause duplicates), then shows OS notifications and queues
//! the reminders as "alerts" for the reminder popup.
//!
//! On startup, and when a tick arrives much later than expected (the laptop was
//! asleep), everything due is treated as *missed* and summarised in a single
//! notification instead of a burst.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::db::Db;
use crate::error::AppResult;
use crate::services::reminders::{self, DueReminder};
use crate::time::{fmt_date, now_hhmm, now_ms, today};
use crate::windows;

const TICK_SECS: u64 = 30;
/// A tick later than this after the previous one means we were asleep.
const WAKE_GAP_MS: i64 = 2 * 60 * 1000;

/// A fired reminder waiting for the user in the popup.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub reminder_id: String,
    pub task_id: String,
    pub task_title: String,
    pub workspace_name: String,
    pub time: String,
    pub missed: bool,
    pub from_snooze: bool,
    pub fired_at: i64,
}

/// Alerts shown in the popup. In memory only: after a restart, anything still
/// due is picked up again by the missed-reminder check.
#[derive(Default)]
pub struct Alerts(pub Mutex<Vec<Alert>>);

impl Alerts {
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Alert>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("reminder-scheduler".into())
        .spawn(move || run(app))
        .expect("failed to start reminder scheduler");
}

fn run(app: AppHandle) {
    let mut last_tick: Option<i64> = None;
    let mut last_day = today();
    loop {
        let now = now_ms();
        let missed = match last_tick {
            None => true,
            Some(prev) => now - prev > WAKE_GAP_MS,
        };
        last_tick = Some(now);

        let day = today();
        if day != last_day {
            last_day = day;
            let _ = app.emit("day-changed", fmt_date(day));
        }

        if let Err(e) = tick(&app, missed) {
            eprintln!("[scheduler] tick failed: {e}");
        }

        // Sleep until the next :00 / :30 boundary so reminders land on time.
        let secs_into = (now_ms() / 1000) as u64 % TICK_SECS;
        std::thread::sleep(Duration::from_secs((TICK_SECS - secs_into).max(1)));
    }
}

fn tick(app: &AppHandle, missed: bool) -> AppResult<()> {
    let db = app.state::<Db>();
    let due = {
        let mut conn = db.conn();
        let due = reminders::due(&conn, today(), &now_hhmm(), now_ms())?;
        if due.is_empty() {
            return Ok(());
        }
        let ids: Vec<String> = due.iter().map(|d| d.reminder_id.clone()).collect();
        reminders::mark_fired(&mut conn, &ids, today())?;
        due
    };
    fire(app, &due, missed);
    Ok(())
}

/// Queues alerts, shows notifications, and tells every window to refresh.
pub fn fire(app: &AppHandle, due: &[DueReminder], missed: bool) {
    let now = now_ms();
    {
        let alerts = app.state::<Alerts>();
        let mut list = alerts.lock();
        for d in due {
            list.retain(|a| a.reminder_id != d.reminder_id);
            list.push(Alert {
                reminder_id: d.reminder_id.clone(),
                task_id: d.task_id.clone(),
                task_title: d.task_title.clone(),
                workspace_name: d.workspace_name.clone(),
                time: d.time.clone(),
                missed,
                from_snooze: d.from_snooze,
                fired_at: now,
            });
        }
    }

    notify(app, due, missed);
    let _ = app.emit("alerts-changed", ());
    let _ = app.emit("data-changed", ());

    // Always show the popup too (on top, without stealing keyboard focus).
    // Notifications can fail silently (permission off, Focus/Do Not Disturb, no
    // notification service, unsigned dev builds on Windows) and the plugin
    // can't report that, so the popup is what guarantees a reminder is seen.
    windows::show_reminder_popup(app, false);
}

fn notify(app: &AppHandle, due: &[DueReminder], missed: bool) {
    let (title, body) = match (missed, due) {
        (_, [one]) => (
            one.task_title.clone(),
            format!(
                "{} · {} · {}",
                if missed {
                    "Missed reminder"
                } else if one.from_snooze {
                    "Snoozed reminder"
                } else {
                    "Reminder"
                },
                one.time,
                one.workspace_name
            ),
        ),
        (true, many) => (
            format!("You missed {} reminders", many.len()),
            summary(many),
        ),
        (false, many) => (format!("{} reminders", many.len()), summary(many)),
    };
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        eprintln!("[scheduler] notification failed: {e}");
    }
}

fn summary(due: &[DueReminder]) -> String {
    let mut lines: Vec<String> = due
        .iter()
        .take(4)
        .map(|d| format!("{} · {}", d.time, d.task_title))
        .collect();
    if due.len() > 4 {
        lines.push(format!("and {} more", due.len() - 4));
    }
    lines.join("\n")
}
