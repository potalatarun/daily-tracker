//! Tasks and the day view, including automatic carry-over.
//!
//! Carry-over is a *query*, not a copy (DECISIONS.md D5): a task keeps its
//! `original_date` forever, and Today simply also selects unfinished tasks from
//! earlier days. Every function that depends on "today" takes it as a parameter
//! so the rules can be tested with fixed dates.

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::services::workspaces;
use crate::time::{fmt_date, now_ms, parse_date};

pub const MAX_TITLE_LEN: usize = 200;
pub const MAX_DESCRIPTION_LEN: usize = 5000;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub description: String,
    pub original_date: String,
    pub status: String,
    pub completed_at: Option<i64>,
    pub completed_on: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// How a task appears on the day being viewed.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Display {
    /// Pending, belongs to this day (today or a future day).
    Planned,
    /// Pending from an earlier day, shown in Today.
    Carried,
    /// Done; shown normally on the day it belongs to, or in Today if finished today.
    Done,
    /// Past day, still pending: it now lives in Today (greyed "moved to today").
    Moved,
    /// Past day, finished on a later day (greyed "completed on ...").
    CompletedLater,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    #[serde(flatten)]
    pub task: Task,
    pub display: Display,
    /// Days between original date and today (pending tasks shown in Today).
    pub carry_days: Option<i64>,
    /// Days between original date and completion, if completed after its day.
    pub done_after_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayView {
    pub date: String,
    pub today: String,
    pub tasks: Vec<TaskView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    pub date: String,
    pub pending: i64,
    pub done: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonthSummary {
    pub days: Vec<DaySummary>,
    /// Unfinished tasks from before today (they all show in Today).
    pub carried_today: i64,
}

const COLS: &str = "id, workspace_id, title, description, original_date, status,
                    completed_at, completed_on, created_at, updated_at";

fn from_row(r: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: r.get("id")?,
        workspace_id: r.get("workspace_id")?,
        title: r.get("title")?,
        description: r.get("description")?,
        original_date: r.get("original_date")?,
        status: r.get("status")?,
        completed_at: r.get("completed_at")?,
        completed_on: r.get("completed_on")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Task> {
    conn.query_row(
        &format!("SELECT {COLS} FROM tasks WHERE id = ?1 AND deleted_at IS NULL"),
        [id],
        from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound("task"))
}

pub fn create(
    conn: &Connection,
    workspace_id: &str,
    date: &str,
    title: &str,
    description: &str,
) -> AppResult<Task> {
    workspaces::get(conn, workspace_id)?;
    let date = fmt_date(parse_date(date)?);
    let title = validate_title(title)?;
    let description = validate_description(description)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    conn.execute(
        "INSERT INTO tasks (id, workspace_id, title, description, original_date,
                            status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)",
        params![id, workspace_id, title, description, date, now],
    )?;
    get(conn, &id)
}

/// Edits title, description and/or date. Changing the date is an explicit
/// reschedule by the user, so the carry count restarts from the new date.
pub fn update(
    conn: &Connection,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    date: Option<&str>,
) -> AppResult<Task> {
    let cur = get(conn, id)?;
    let title = title.map(validate_title).transpose()?.unwrap_or(cur.title);
    let description = description
        .map(validate_description)
        .transpose()?
        .unwrap_or(cur.description);
    let date = match date {
        Some(d) => fmt_date(parse_date(d)?),
        None => cur.original_date,
    };
    conn.execute(
        "UPDATE tasks SET title = ?2, description = ?3, original_date = ?4, updated_at = ?5
         WHERE id = ?1",
        params![id, title, description, date, now_ms()],
    )?;
    get(conn, id)
}

/// Marks a task done (on `today`) and stops its timer if one is running.
pub fn complete(conn: &mut Connection, id: &str, today: NaiveDate) -> AppResult<Task> {
    let cur = get(conn, id)?;
    if cur.status == "done" {
        return Ok(cur);
    }
    let now = now_ms();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE time_sessions SET ended_at = ?2 WHERE task_id = ?1 AND ended_at IS NULL",
        params![id, now],
    )?;
    tx.execute(
        "UPDATE tasks SET status = 'done', completed_at = ?2, completed_on = ?3, updated_at = ?2
         WHERE id = ?1",
        params![id, now, fmt_date(today)],
    )?;
    tx.commit()?;
    get(conn, id)
}

/// Undoes completion. If the task's day is past, it carries over again.
pub fn reopen(conn: &Connection, id: &str) -> AppResult<Task> {
    get(conn, id)?;
    conn.execute(
        "UPDATE tasks SET status = 'pending', completed_at = NULL, completed_on = NULL,
                          updated_at = ?2
         WHERE id = ?1",
        params![id, now_ms()],
    )?;
    get(conn, id)
}

/// Soft delete (so it can be undone) and stop any running timer.
pub fn delete(conn: &mut Connection, id: &str) -> AppResult<()> {
    get(conn, id)?;
    let now = now_ms();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE time_sessions SET ended_at = ?2 WHERE task_id = ?1 AND ended_at IS NULL",
        params![id, now],
    )?;
    tx.execute(
        "UPDATE tasks SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )?;
    tx.commit()?;
    Ok(())
}

/// Undo for `delete`. Fails if the workspace itself was deleted meanwhile.
pub fn restore(conn: &Connection, id: &str) -> AppResult<Task> {
    let ws: Option<String> = conn
        .query_row("SELECT workspace_id FROM tasks WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    let ws = ws.ok_or(AppError::NotFound("task"))?;
    workspaces::get(conn, &ws)?;
    conn.execute(
        "UPDATE tasks SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    get(conn, id)
}

/// Everything shown for `date` in a workspace. See ARCHITECTURE.md §5.1.
pub fn day_view(
    conn: &Connection,
    workspace_id: &str,
    date: &str,
    today: NaiveDate,
) -> AppResult<DayView> {
    workspaces::get(conn, workspace_id)?;
    let day = parse_date(date)?;
    let day_s = fmt_date(day);
    let today_s = fmt_date(today);

    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM tasks
         WHERE workspace_id = ?1 AND deleted_at IS NULL
           AND (original_date = ?2
                OR (?2 = ?3 AND original_date < ?3
                    AND (status = 'pending' OR completed_on = ?3)))"
    ))?;
    let tasks = stmt
        .query_map(params![workspace_id, day_s, today_s], from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut views: Vec<TaskView> = tasks.into_iter().map(|t| classify(t, day, today)).collect();
    views.sort_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then_with(|| a.task.original_date.cmp(&b.task.original_date))
            .then_with(|| {
                a.task
                    .completed_at
                    .unwrap_or(0)
                    .cmp(&b.task.completed_at.unwrap_or(0))
            })
            .then_with(|| a.task.created_at.cmp(&b.task.created_at))
    });

    Ok(DayView {
        date: day_s,
        today: today_s,
        tasks: views,
    })
}

fn classify(task: Task, day: NaiveDate, today: NaiveDate) -> TaskView {
    let original = parse_date(&task.original_date).unwrap_or(day);
    let completed_on = task
        .completed_on
        .as_deref()
        .and_then(|d| parse_date(d).ok());
    let pending = task.status == "pending";

    let (display, carry_days) = if pending {
        if day == today && original < today {
            (Display::Carried, Some((today - original).num_days()))
        } else if day < today {
            (Display::Moved, Some((today - original).num_days()))
        } else {
            (Display::Planned, None)
        }
    } else {
        match completed_on {
            Some(c) if original == day && c > day => (Display::CompletedLater, None),
            _ => (Display::Done, None),
        }
    };
    let done_after_days = completed_on
        .map(|c| (c - original).num_days())
        .filter(|d| *d > 0);

    TaskView {
        task,
        display,
        carry_days,
        done_after_days,
    }
}

/// Order within a day: carried (oldest first), planned, done, then greyed history.
fn rank(v: &TaskView) -> u8 {
    match v.display {
        Display::Carried => 0,
        Display::Planned => 1,
        Display::Done => 2,
        Display::Moved => 3,
        Display::CompletedLater => 4,
    }
}

/// Per-day counts for the mini calendar, `from`..=`to` inclusive.
pub fn month_summary(
    conn: &Connection,
    workspace_id: &str,
    from: &str,
    to: &str,
    today: NaiveDate,
) -> AppResult<MonthSummary> {
    let from = fmt_date(parse_date(from)?);
    let to = fmt_date(parse_date(to)?);
    let mut stmt = conn.prepare(
        "SELECT original_date,
                SUM(status = 'pending'),
                SUM(status = 'done')
         FROM tasks
         WHERE workspace_id = ?1 AND deleted_at IS NULL
           AND original_date BETWEEN ?2 AND ?3
         GROUP BY original_date ORDER BY original_date",
    )?;
    let days = stmt
        .query_map(params![workspace_id, from, to], |r| {
            Ok(DaySummary {
                date: r.get(0)?,
                pending: r.get(1)?,
                done: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let carried_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks
         WHERE workspace_id = ?1 AND deleted_at IS NULL
           AND status = 'pending' AND original_date < ?2",
        params![workspace_id, fmt_date(today)],
        |r| r.get(0),
    )?;
    Ok(MonthSummary {
        days,
        carried_today,
    })
}

fn validate_title(title: &str) -> AppResult<String> {
    let t = title.trim();
    if t.is_empty() {
        return Err(AppError::Validation("Give the task a title".into()));
    }
    if t.chars().count() > MAX_TITLE_LEN {
        return Err(AppError::Validation(format!(
            "Title must be at most {MAX_TITLE_LEN} characters"
        )));
    }
    Ok(t.to_string())
}

fn validate_description(d: &str) -> AppResult<String> {
    let d = d.trim();
    if d.chars().count() > MAX_DESCRIPTION_LEN {
        return Err(AppError::Validation(format!(
            "Description must be at most {MAX_DESCRIPTION_LEN} characters"
        )));
    }
    Ok(d.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn d(s: &str) -> NaiveDate {
        parse_date(s).unwrap()
    }

    fn setup() -> (Db, String) {
        let db = Db::open_in_memory().unwrap();
        let ws = workspaces::create(&db.conn(), "Work", "#000000")
            .unwrap()
            .id;
        (db, ws)
    }

    fn titles(v: &DayView) -> Vec<(&str, Display)> {
        v.tasks
            .iter()
            .map(|t| (t.task.title.as_str(), t.display))
            .collect()
    }

    #[test]
    fn create_validates() {
        let (db, ws) = setup();
        let c = db.conn();
        assert!(create(&c, &ws, "2026-11-22", "  ", "").is_err());
        assert!(create(&c, &ws, "22/11/2026", "Ok", "").is_err());
        assert!(create(&c, "missing", "2026-11-22", "Ok", "").is_err());
        let t = create(&c, &ws, "2026-11-22", "  Write report ", " notes ").unwrap();
        assert_eq!(t.title, "Write report");
        assert_eq!(t.description, "notes");
        assert_eq!(t.status, "pending");
    }

    #[test]
    fn unfinished_tasks_carry_into_today_with_count() {
        let (db, ws) = setup();
        let c = db.conn();
        create(&c, &ws, "2026-11-18", "Old", "").unwrap();
        create(&c, &ws, "2026-11-22", "Today", "").unwrap();
        create(&c, &ws, "2026-11-25", "Future", "").unwrap();

        let v = day_view(&c, &ws, "2026-11-22", d("2026-11-22")).unwrap();
        assert_eq!(
            titles(&v),
            vec![("Old", Display::Carried), ("Today", Display::Planned)]
        );
        assert_eq!(v.tasks[0].carry_days, Some(4));
        assert_eq!(v.tasks[0].task.original_date, "2026-11-18");
    }

    #[test]
    fn carry_count_is_right_after_days_away() {
        let (db, ws) = setup();
        let c = db.conn();
        create(&c, &ws, "2026-11-18", "Old", "").unwrap();
        // App was closed for a week: no job ran, the count is still correct.
        let v = day_view(&c, &ws, "2026-11-25", d("2026-11-25")).unwrap();
        assert_eq!(v.tasks[0].carry_days, Some(7));
    }

    #[test]
    fn past_day_shows_moved_and_completed_later() {
        let (db, ws) = setup();
        let mut c = db.conn();
        let a = create(&c, &ws, "2026-11-18", "Still open", "").unwrap();
        let b = create(&c, &ws, "2026-11-18", "Finished later", "").unwrap();
        let e = create(&c, &ws, "2026-11-18", "Finished same day", "").unwrap();
        complete(&mut c, &b.id, d("2026-11-21")).unwrap();
        complete(&mut c, &e.id, d("2026-11-18")).unwrap();

        let v = day_view(&c, &ws, "2026-11-18", d("2026-11-22")).unwrap();
        let got: Vec<_> = v
            .tasks
            .iter()
            .map(|t| (t.task.id.clone(), t.display))
            .collect();
        assert!(got.contains(&(a.id.clone(), Display::Moved)));
        assert!(got.contains(&(b.id.clone(), Display::CompletedLater)));
        assert!(got.contains(&(e.id.clone(), Display::Done)));
        // Ordering: done before greyed history.
        assert_eq!(v.tasks[0].display, Display::Done);
    }

    #[test]
    fn finished_carried_task_stays_in_today_as_done() {
        let (db, ws) = setup();
        let mut c = db.conn();
        let t = create(&c, &ws, "2026-11-18", "Old", "").unwrap();
        complete(&mut c, &t.id, d("2026-11-22")).unwrap();

        let today = day_view(&c, &ws, "2026-11-22", d("2026-11-22")).unwrap();
        assert_eq!(titles(&today), vec![("Old", Display::Done)]);
        assert_eq!(today.tasks[0].done_after_days, Some(4));

        // Next day it no longer appears in Today.
        let tomorrow = day_view(&c, &ws, "2026-11-23", d("2026-11-23")).unwrap();
        assert!(tomorrow.tasks.is_empty());
    }

    #[test]
    fn reopen_makes_it_carry_again() {
        let (db, ws) = setup();
        let mut c = db.conn();
        let t = create(&c, &ws, "2026-11-20", "Task", "").unwrap();
        complete(&mut c, &t.id, d("2026-11-20")).unwrap();
        let r = reopen(&c, &t.id).unwrap();
        assert_eq!(r.status, "pending");
        assert_eq!(r.completed_on, None);
        let v = day_view(&c, &ws, "2026-11-22", d("2026-11-22")).unwrap();
        assert_eq!(v.tasks[0].display, Display::Carried);
    }

    #[test]
    fn complete_stops_running_timer() {
        let (db, ws) = setup();
        let mut c = db.conn();
        let t = create(&c, &ws, "2026-11-22", "Task", "").unwrap();
        c.execute(
            "INSERT INTO time_sessions (id, task_id, started_at) VALUES ('s', ?1, 1)",
            [&t.id],
        )
        .unwrap();
        complete(&mut c, &t.id, d("2026-11-22")).unwrap();
        let ended: Option<i64> = c
            .query_row(
                "SELECT ended_at FROM time_sessions WHERE id = 's'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ended.is_some());
    }

    #[test]
    fn delete_and_restore() {
        let (db, ws) = setup();
        let mut c = db.conn();
        let t = create(&c, &ws, "2026-11-22", "Task", "").unwrap();
        delete(&mut c, &t.id).unwrap();
        assert!(day_view(&c, &ws, "2026-11-22", d("2026-11-22"))
            .unwrap()
            .tasks
            .is_empty());
        assert!(matches!(get(&c, &t.id), Err(AppError::NotFound(_))));
        restore(&c, &t.id).unwrap();
        assert_eq!(
            day_view(&c, &ws, "2026-11-22", d("2026-11-22"))
                .unwrap()
                .tasks
                .len(),
            1
        );
    }

    #[test]
    fn rescheduling_moves_task_and_resets_count() {
        let (db, ws) = setup();
        let c = db.conn();
        let t = create(&c, &ws, "2026-11-18", "Task", "").unwrap();
        let u = update(&c, &t.id, Some("Renamed"), None, Some("2026-11-24")).unwrap();
        assert_eq!(u.title, "Renamed");
        assert_eq!(u.original_date, "2026-11-24");
        assert!(day_view(&c, &ws, "2026-11-22", d("2026-11-22"))
            .unwrap()
            .tasks
            .is_empty());
    }

    #[test]
    fn workspaces_are_isolated() {
        let (db, ws) = setup();
        let c = db.conn();
        let other = workspaces::create(&c, "Personal", "#000000").unwrap().id;
        create(&c, &ws, "2026-11-22", "Work task", "").unwrap();
        create(&c, &other, "2026-11-20", "Personal task", "").unwrap();
        let v = day_view(&c, &ws, "2026-11-22", d("2026-11-22")).unwrap();
        assert_eq!(titles(&v), vec![("Work task", Display::Planned)]);
    }

    #[test]
    fn month_summary_counts() {
        let (db, ws) = setup();
        let mut c = db.conn();
        create(&c, &ws, "2026-11-03", "A", "").unwrap();
        let b = create(&c, &ws, "2026-11-03", "B", "").unwrap();
        create(&c, &ws, "2026-12-01", "Outside", "").unwrap();
        complete(&mut c, &b.id, d("2026-11-03")).unwrap();
        let m = month_summary(&c, &ws, "2026-11-01", "2026-11-30", d("2026-11-22")).unwrap();
        assert_eq!(
            m.days,
            vec![DaySummary {
                date: "2026-11-03".into(),
                pending: 1,
                done: 1
            }]
        );
        assert_eq!(m.carried_today, 1);
    }
}
