//! Reminder times on tasks and the rules for when they are due.
//!
//! A reminder is a local wall-clock time ("15:00") on a task. It fires once per
//! day while its task is pending and visible in Today, so carried tasks repeat
//! their reminders every day (ARCHITECTURE.md §5.3). `last_fired_on` records
//! the local date it last fired; `snoozed_until` overrides the daily time.

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::services::tasks;
use crate::time::fmt_date;

pub const MAX_PER_TASK: usize = 10;
/// Longest allowed snooze.
const MAX_SNOOZE_MS: i64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub task_id: String,
    /// Local "HH:MM".
    pub time: String,
    pub enabled: bool,
    pub last_fired_on: Option<String>,
    pub snoozed_until: Option<i64>,
}

/// A reminder that should fire now, with what the notification and popup show.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DueReminder {
    pub reminder_id: String,
    pub task_id: String,
    pub task_title: String,
    pub workspace_id: String,
    pub workspace_name: String,
    pub time: String,
    /// True when it fires because a snooze ended (not its daily time).
    pub from_snooze: bool,
}

fn from_row(r: &Row) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: r.get("id")?,
        task_id: r.get("task_id")?,
        time: r.get("time")?,
        enabled: r.get::<_, i64>("enabled")? != 0,
        last_fired_on: r.get("last_fired_on")?,
        snoozed_until: r.get("snoozed_until")?,
    })
}

pub fn list_for_task(conn: &Connection, task_id: &str) -> AppResult<Vec<Reminder>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, time, enabled, last_fired_on, snoozed_until
         FROM reminders WHERE task_id = ?1 ORDER BY time",
    )?;
    let rows = stmt
        .query_map([task_id], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Replaces a task's reminder times with `times`.
///
/// Times that already exist keep their state (fired today, snoozed). A newly
/// added time that is already past today counts as fired today, so adding
/// "09:00" at 17:00 doesn't trigger an instant notification; it fires tomorrow
/// if the task is still open.
pub fn set_for_task(
    conn: &mut Connection,
    task_id: &str,
    times: &[String],
    today: NaiveDate,
    now_hhmm: &str,
) -> AppResult<Vec<Reminder>> {
    let task = tasks::get(conn, task_id)?;
    let mut wanted: Vec<String> = Vec::new();
    for t in times {
        let t = normalize_time(t)?;
        if !wanted.contains(&t) {
            wanted.push(t);
        }
    }
    if wanted.len() > MAX_PER_TASK {
        return Err(AppError::Validation(format!(
            "A task can have at most {MAX_PER_TASK} reminders"
        )));
    }

    let existing = list_for_task(conn, task_id)?;
    let today_s = fmt_date(today);
    // Only tasks that show in Today (their day is today or earlier) can fire today.
    let task_active_today = task.original_date <= today_s;

    let tx = conn.transaction()?;
    for r in &existing {
        if !wanted.contains(&r.time) {
            tx.execute("DELETE FROM reminders WHERE id = ?1", [&r.id])?;
        }
    }
    for t in &wanted {
        if existing.iter().any(|r| &r.time == t) {
            continue;
        }
        let already_past = task_active_today && t.as_str() <= now_hhmm;
        tx.execute(
            "INSERT INTO reminders (id, task_id, time, enabled, last_fired_on)
             VALUES (?1, ?2, ?3, 1, ?4)",
            params![
                uuid::Uuid::new_v4().to_string(),
                task_id,
                t,
                already_past.then(|| today_s.clone())
            ],
        )?;
    }
    tx.commit()?;
    list_for_task(conn, task_id)
}

/// Reminders that should fire at `now`.
pub fn due(
    conn: &Connection,
    today: NaiveDate,
    now_hhmm: &str,
    now_ms: i64,
) -> AppResult<Vec<DueReminder>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.task_id, t.title, t.workspace_id, w.name, r.time,
                (r.snoozed_until IS NOT NULL) AS from_snooze
         FROM reminders r
         JOIN tasks t ON t.id = r.task_id
         JOIN workspaces w ON w.id = t.workspace_id
         WHERE r.enabled = 1
           AND t.status = 'pending' AND t.deleted_at IS NULL AND w.deleted_at IS NULL
           AND t.original_date <= ?1
           AND (
                 (r.snoozed_until IS NOT NULL AND r.snoozed_until <= ?3)
              OR (r.snoozed_until IS NULL AND r.time <= ?2
                  AND (r.last_fired_on IS NULL OR r.last_fired_on < ?1))
           )
         ORDER BY r.time, t.title",
    )?;
    let rows = stmt
        .query_map(params![fmt_date(today), now_hhmm, now_ms], |r| {
            Ok(DueReminder {
                reminder_id: r.get(0)?,
                task_id: r.get(1)?,
                task_title: r.get(2)?,
                workspace_id: r.get(3)?,
                workspace_name: r.get(4)?,
                time: r.get(5)?,
                from_snooze: r.get::<_, i64>(6)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Records that these reminders fired today. Written *before* notifying, so a
/// crash right after can never cause a duplicate on restart.
pub fn mark_fired(conn: &mut Connection, ids: &[String], today: NaiveDate) -> AppResult<()> {
    let tx = conn.transaction()?;
    for id in ids {
        tx.execute(
            "UPDATE reminders SET last_fired_on = ?2, snoozed_until = NULL WHERE id = ?1",
            params![id, fmt_date(today)],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn snooze(conn: &Connection, id: &str, until_ms: i64, now_ms: i64) -> AppResult<Reminder> {
    get(conn, id)?;
    if until_ms <= now_ms {
        return Err(AppError::Validation(
            "Snooze until a time in the future".into(),
        ));
    }
    if until_ms - now_ms > MAX_SNOOZE_MS {
        return Err(AppError::Validation("Snooze can be at most 7 days".into()));
    }
    conn.execute(
        "UPDATE reminders SET snoozed_until = ?2 WHERE id = ?1",
        params![id, until_ms],
    )?;
    get(conn, id)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Reminder> {
    conn.query_row(
        "SELECT id, task_id, time, enabled, last_fired_on, snoozed_until
         FROM reminders WHERE id = ?1",
        [id],
        from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound("reminder"))
}

/// Accepts "9:05", "09:05", "0905" or "9.05"; returns zero-padded "HH:MM".
pub fn normalize_time(s: &str) -> AppResult<String> {
    let t = s.trim().replace('.', ":");
    let (h, m) = match t.split_once(':') {
        Some((h, m)) => (h.to_string(), m.to_string()),
        None if t.len() == 3 || t.len() == 4 => {
            let (h, m) = t.split_at(t.len() - 2);
            (h.to_string(), m.to_string())
        }
        None => (String::new(), String::new()),
    };
    let bad = || AppError::Validation(format!("\"{s}\" is not a valid time (like 09:30)"));
    if m.len() != 2 || h.is_empty() || h.len() > 2 {
        return Err(bad());
    }
    let h: u32 = h.parse().map_err(|_| bad())?;
    let m: u32 = m.parse().map_err(|_| bad())?;
    if h > 23 || m > 59 {
        return Err(bad());
    }
    Ok(format!("{h:02}:{m:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::services::workspaces;
    use crate::time::parse_date;

    fn d(s: &str) -> NaiveDate {
        parse_date(s).unwrap()
    }

    fn setup() -> (Db, String, String) {
        let db = Db::open_in_memory().unwrap();
        let (ws, task) = {
            let c = db.conn();
            let ws = workspaces::create(&c, "Work", "#000000").unwrap().id;
            let task = tasks::create(&c, &ws, "2026-11-22", "Fix bug", "")
                .unwrap()
                .id;
            (ws, task)
        };
        (db, ws, task)
    }

    fn times(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn due_ids(c: &Connection, today: &str, hhmm: &str, now: i64) -> Vec<String> {
        due(c, d(today), hhmm, now)
            .unwrap()
            .into_iter()
            .map(|r| r.time)
            .collect()
    }

    #[test]
    fn normalizes_times() {
        assert_eq!(normalize_time("9:05").unwrap(), "09:05");
        assert_eq!(normalize_time("0905").unwrap(), "09:05");
        assert_eq!(normalize_time("9.05").unwrap(), "09:05");
        assert_eq!(normalize_time(" 15:00 ").unwrap(), "15:00");
        for bad in ["24:00", "9:60", "abc", "", "9", "123:00"] {
            assert!(normalize_time(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn fires_once_per_day_at_its_time() {
        let (db, _, task) = setup();
        let mut c = db.conn();
        set_for_task(&mut c, &task, &times(&["15:00"]), d("2026-11-22"), "09:00").unwrap();
        assert!(due_ids(&c, "2026-11-22", "14:59", 0).is_empty());
        assert_eq!(due_ids(&c, "2026-11-22", "15:00", 0), vec!["15:00"]);
        let ids: Vec<String> = due(&c, d("2026-11-22"), "15:00", 0)
            .unwrap()
            .into_iter()
            .map(|r| r.reminder_id)
            .collect();
        mark_fired(&mut c, &ids, d("2026-11-22")).unwrap();
        assert!(due_ids(&c, "2026-11-22", "23:00", 0).is_empty());
    }

    #[test]
    fn carried_task_repeats_reminder_next_day() {
        let (db, _, task) = setup();
        let mut c = db.conn();
        let r = set_for_task(&mut c, &task, &times(&["15:00"]), d("2026-11-22"), "09:00").unwrap();
        mark_fired(&mut c, &[r[0].id.clone()], d("2026-11-22")).unwrap();
        // Still pending the next day: fires again at the same time.
        assert_eq!(due_ids(&c, "2026-11-23", "15:00", 0), vec!["15:00"]);
    }

    #[test]
    fn done_deleted_or_future_tasks_dont_fire() {
        let (db, ws, task) = setup();
        let mut c = db.conn();
        set_for_task(&mut c, &task, &times(&["10:00"]), d("2026-11-22"), "09:00").unwrap();
        let future = tasks::create(&c, &ws, "2026-11-25", "Later", "")
            .unwrap()
            .id;
        set_for_task(
            &mut c,
            &future,
            &times(&["10:00"]),
            d("2026-11-22"),
            "09:00",
        )
        .unwrap();
        assert_eq!(due_ids(&c, "2026-11-22", "10:00", 0).len(), 1); // only today's task

        tasks::complete(&mut c, &task, d("2026-11-22")).unwrap();
        assert!(due_ids(&c, "2026-11-22", "10:00", 0).is_empty());
        tasks::reopen(&c, &task).unwrap();
        tasks::delete(&mut c, &task).unwrap();
        assert!(due_ids(&c, "2026-11-22", "10:00", 0).is_empty());
        // The future task fires on its own day.
        assert_eq!(due_ids(&c, "2026-11-25", "10:00", 0).len(), 1);
    }

    #[test]
    fn adding_a_past_time_does_not_fire_instantly() {
        let (db, _, task) = setup();
        let mut c = db.conn();
        set_for_task(
            &mut c,
            &task,
            &times(&["09:00", "18:00"]),
            d("2026-11-22"),
            "17:00",
        )
        .unwrap();
        assert_eq!(due_ids(&c, "2026-11-22", "17:00", 0), Vec::<String>::new());
        assert_eq!(due_ids(&c, "2026-11-22", "18:00", 0), vec!["18:00"]);
        // Tomorrow both fire as normal.
        assert_eq!(
            due_ids(&c, "2026-11-23", "18:00", 0),
            vec!["09:00", "18:00"]
        );
    }

    #[test]
    fn set_keeps_state_of_unchanged_times_and_dedups() {
        let (db, _, task) = setup();
        let mut c = db.conn();
        let r = set_for_task(&mut c, &task, &times(&["15:00"]), d("2026-11-22"), "09:00").unwrap();
        mark_fired(&mut c, &[r[0].id.clone()], d("2026-11-22")).unwrap();
        let r2 = set_for_task(
            &mut c,
            &task,
            &times(&["15:00", "3:00 ", "16:00", "16:00"]),
            d("2026-11-22"),
            "09:00",
        )
        .unwrap();
        assert_eq!(r2.len(), 3);
        let kept = r2.iter().find(|x| x.time == "15:00").unwrap();
        assert_eq!(kept.id, r[0].id);
        assert_eq!(kept.last_fired_on.as_deref(), Some("2026-11-22"));
        let r3 = set_for_task(&mut c, &task, &[], d("2026-11-22"), "09:00").unwrap();
        assert!(r3.is_empty());
    }

    #[test]
    fn limit_and_validation() {
        let (db, _, task) = setup();
        let mut c = db.conn();
        let many: Vec<String> = (0..11).map(|i| format!("{:02}:00", i)).collect();
        assert!(set_for_task(&mut c, &task, &many, d("2026-11-22"), "00:00").is_err());
        assert!(set_for_task(&mut c, &task, &times(&["25:00"]), d("2026-11-22"), "00:00").is_err());
    }

    #[test]
    fn snooze_overrides_daily_time() {
        let (db, _, task) = setup();
        let mut c = db.conn();
        let r = set_for_task(&mut c, &task, &times(&["15:00"]), d("2026-11-22"), "09:00").unwrap();
        let id = r[0].id.clone();
        mark_fired(&mut c, std::slice::from_ref(&id), d("2026-11-22")).unwrap();
        let now = 1_000_000;
        snooze(&c, &id, now + 600_000, now).unwrap();
        assert!(due(&c, d("2026-11-22"), "15:05", now + 300_000)
            .unwrap()
            .is_empty());
        let fired = due(&c, d("2026-11-22"), "15:10", now + 600_000).unwrap();
        assert_eq!(fired.len(), 1);
        assert!(fired[0].from_snooze);
        mark_fired(&mut c, std::slice::from_ref(&id), d("2026-11-22")).unwrap();
        assert!(get(&c, &id).unwrap().snoozed_until.is_none());
        assert!(due(&c, d("2026-11-22"), "15:11", now + 700_000)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn snooze_validation() {
        let (db, _, task) = setup();
        let mut c = db.conn();
        let r = set_for_task(&mut c, &task, &times(&["15:00"]), d("2026-11-22"), "09:00").unwrap();
        assert!(snooze(&c, &r[0].id, 100, 200).is_err());
        assert!(snooze(&c, &r[0].id, 8 * 24 * 3_600_000, 0).is_err());
    }
}
