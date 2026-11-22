//! Time tracking with pause/resume (DECISIONS.md D8, D9).
//!
//! Each Start→Pause span is one `time_sessions` row; a running session has
//! `ended_at = NULL`. Totals are computed from timestamps, so nothing has to
//! tick in the background and closing the app or sleeping can't lose time.
//! Only one timer runs at a time across all workspaces.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::services::tasks;

/// Allow a little clock skew between the UI and the core when validating
/// "not in the future".
const FUTURE_TOLERANCE_MS: i64 = 60_000;
/// A single session longer than this is almost certainly a typo.
const MAX_SESSION_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub task_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

/// The running timer, with enough context to show it anywhere in the app.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTimer {
    pub session_id: String,
    pub task_id: String,
    pub task_title: String,
    /// The task's own date, so the UI can jump to where it is shown.
    pub task_date: String,
    pub workspace_id: String,
    pub started_at: i64,
    /// Time tracked on this task before the current session.
    pub previous_ms: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartResult {
    pub active: ActiveTimer,
    /// Task that was auto-paused to start this one, if any.
    pub paused_task_title: Option<String>,
}

fn from_row(r: &Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: r.get("id")?,
        task_id: r.get("task_id")?,
        started_at: r.get("started_at")?,
        ended_at: r.get("ended_at")?,
    })
}

pub fn list(conn: &Connection, task_id: &str) -> AppResult<Vec<Session>> {
    tasks::get(conn, task_id)?;
    let mut stmt = conn.prepare(
        "SELECT id, task_id, started_at, ended_at FROM time_sessions
         WHERE task_id = ?1 ORDER BY started_at",
    )?;
    let rows = stmt
        .query_map([task_id], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Sum of finished sessions for a task (the running one is added live by the UI).
pub fn closed_total_ms(conn: &Connection, task_id: &str) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(ended_at - started_at), 0) FROM time_sessions
         WHERE task_id = ?1 AND ended_at IS NOT NULL",
        [task_id],
        |r| r.get(0),
    )?)
}

pub fn active(conn: &Connection) -> AppResult<Option<ActiveTimer>> {
    let row = conn
        .query_row(
            "SELECT s.id, s.task_id, s.started_at, t.title, t.workspace_id, t.original_date
             FROM time_sessions s
             JOIN tasks t ON t.id = s.task_id
             JOIN workspaces w ON w.id = t.workspace_id
             WHERE s.ended_at IS NULL AND t.deleted_at IS NULL AND w.deleted_at IS NULL
             ORDER BY s.started_at DESC LIMIT 1",
            [],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some((session_id, task_id, started_at, task_title, workspace_id, task_date)) => {
            let previous_ms = closed_total_ms(conn, &task_id)?;
            Ok(Some(ActiveTimer {
                session_id,
                task_id,
                task_title,
                task_date,
                workspace_id,
                started_at,
                previous_ms,
            }))
        }
    }
}

/// Starts (or resumes) the timer on a task, pausing any other running timer
/// in the same transaction so two can never run at once.
pub fn start(conn: &mut Connection, task_id: &str, now: i64) -> AppResult<StartResult> {
    let task = tasks::get(conn, task_id)?;
    if task.status == "done" {
        return Err(AppError::Validation(
            "This task is done. Mark it not done to track more time.".into(),
        ));
    }
    if let Some(a) = active(conn)? {
        if a.task_id == task_id {
            return Ok(StartResult {
                active: a,
                paused_task_title: None,
            });
        }
    }

    let tx = conn.transaction()?;
    let paused_task_title: Option<String> = tx
        .query_row(
            "SELECT t.title FROM time_sessions s JOIN tasks t ON t.id = s.task_id
             WHERE s.ended_at IS NULL LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    // max() guards against a clock that moved backwards since the other timer started.
    tx.execute(
        "UPDATE time_sessions SET ended_at = MAX(started_at, ?1) WHERE ended_at IS NULL",
        [now],
    )?;
    tx.execute(
        "INSERT INTO time_sessions (id, task_id, started_at) VALUES (?1, ?2, ?3)",
        params![uuid::Uuid::new_v4().to_string(), task_id, now],
    )?;
    tx.commit()?;

    let active = active(conn)?.ok_or(AppError::NotFound("timer"))?;
    Ok(StartResult {
        active,
        paused_task_title,
    })
}

/// Pauses the task's running session. Pausing a task that isn't running is a no-op.
pub fn pause(conn: &Connection, task_id: &str, now: i64) -> AppResult<()> {
    tasks::get(conn, task_id)?;
    conn.execute(
        "UPDATE time_sessions SET ended_at = MAX(started_at, ?2)
         WHERE task_id = ?1 AND ended_at IS NULL",
        params![task_id, now],
    )?;
    Ok(())
}

/// Adds a finished session by hand (e.g. "I forgot to start the timer").
pub fn add(
    conn: &Connection,
    task_id: &str,
    started_at: i64,
    ended_at: i64,
    now: i64,
) -> AppResult<Session> {
    tasks::get(conn, task_id)?;
    validate(conn, task_id, None, started_at, Some(ended_at), now)?;
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO time_sessions (id, task_id, started_at, ended_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, task_id, started_at, ended_at],
    )?;
    get(conn, &id)
}

/// Corrects a session's times. A running session keeps running unless an end is given.
pub fn update(
    conn: &Connection,
    id: &str,
    started_at: i64,
    ended_at: Option<i64>,
    now: i64,
) -> AppResult<Session> {
    let cur = get(conn, id)?;
    if cur.ended_at.is_some() && ended_at.is_none() {
        return Err(AppError::Validation(
            "A finished session needs an end time".into(),
        ));
    }
    validate(conn, &cur.task_id, Some(id), started_at, ended_at, now)?;
    conn.execute(
        "UPDATE time_sessions SET started_at = ?2, ended_at = ?3 WHERE id = ?1",
        params![id, started_at, ended_at],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    get(conn, id)?;
    conn.execute("DELETE FROM time_sessions WHERE id = ?1", [id])?;
    Ok(())
}

fn get(conn: &Connection, id: &str) -> AppResult<Session> {
    conn.query_row(
        "SELECT id, task_id, started_at, ended_at FROM time_sessions WHERE id = ?1",
        [id],
        from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound("time session"))
}

fn validate(
    conn: &Connection,
    task_id: &str,
    except_id: Option<&str>,
    started_at: i64,
    ended_at: Option<i64>,
    now: i64,
) -> AppResult<()> {
    if started_at > now + FUTURE_TOLERANCE_MS {
        return Err(AppError::Validation(
            "Start time can't be in the future".into(),
        ));
    }
    if let Some(end) = ended_at {
        if end <= started_at {
            return Err(AppError::Validation(
                "End time must be after the start time".into(),
            ));
        }
        if end > now + FUTURE_TOLERANCE_MS {
            return Err(AppError::Validation(
                "End time can't be in the future".into(),
            ));
        }
        if end - started_at > MAX_SESSION_MS {
            return Err(AppError::Validation(
                "A single session can't be longer than 24 hours".into(),
            ));
        }
    }
    // Sessions of the same task must not overlap. A running session extends to `now`.
    let end = ended_at.unwrap_or(now);
    let overlaps: i64 = conn.query_row(
        "SELECT COUNT(*) FROM time_sessions
         WHERE task_id = ?1 AND id != COALESCE(?2, '')
           AND started_at < ?4 AND COALESCE(ended_at, ?5) > ?3",
        params![task_id, except_id, started_at, end, now],
        |r| r.get(0),
    )?;
    if overlaps > 0 {
        return Err(AppError::Validation(
            "This overlaps another time entry for the same task".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::services::workspaces;
    use crate::time::parse_date;

    const MIN: i64 = 60_000;
    const HOUR: i64 = 60 * MIN;
    const T0: i64 = 1_795_000_000_000;

    fn setup() -> (Db, String, String, String) {
        let db = Db::open_in_memory().unwrap();
        let (a, b, ws) = {
            let c = db.conn();
            let ws = workspaces::create(&c, "Work", "#000000").unwrap().id;
            let a = tasks::create(&c, &ws, "2026-11-22", "A", "").unwrap().id;
            let b = tasks::create(&c, &ws, "2026-11-22", "B", "").unwrap().id;
            (a, b, ws)
        };
        (db, a, b, ws)
    }

    #[test]
    fn pause_resume_sums_worked_time_not_elapsed() {
        let (db, a, _, _) = setup();
        let mut c = db.conn();
        // Monday: 1h. Tuesday: 2h. About 26h elapsed in between.
        start(&mut c, &a, T0).unwrap();
        pause(&c, &a, T0 + HOUR).unwrap();
        start(&mut c, &a, T0 + 24 * HOUR).unwrap();
        pause(&c, &a, T0 + 26 * HOUR).unwrap();
        assert_eq!(closed_total_ms(&c, &a).unwrap(), 3 * HOUR);
        assert_eq!(list(&c, &a).unwrap().len(), 2);
        assert!(active(&c).unwrap().is_none());
    }

    #[test]
    fn starting_another_task_auto_pauses_the_first() {
        let (db, a, b, _) = setup();
        let mut c = db.conn();
        start(&mut c, &a, T0).unwrap();
        let r = start(&mut c, &b, T0 + 10 * MIN).unwrap();
        assert_eq!(r.paused_task_title.as_deref(), Some("A"));
        assert_eq!(r.active.task_id, b);
        assert_eq!(closed_total_ms(&c, &a).unwrap(), 10 * MIN);
        let running: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM time_sessions WHERE ended_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(running, 1);
    }

    #[test]
    fn only_one_timer_across_workspaces() {
        let (db, a, _, _) = setup();
        let mut c = db.conn();
        let ws2 = workspaces::create(&c, "Personal", "#000000").unwrap().id;
        let p = tasks::create(&c, &ws2, "2026-11-22", "P", "").unwrap().id;
        start(&mut c, &a, T0).unwrap();
        let r = start(&mut c, &p, T0 + MIN).unwrap();
        assert_eq!(r.paused_task_title.as_deref(), Some("A"));
        assert_eq!(active(&c).unwrap().unwrap().workspace_id, ws2);
    }

    #[test]
    fn starting_running_task_again_is_a_no_op() {
        let (db, a, _, _) = setup();
        let mut c = db.conn();
        start(&mut c, &a, T0).unwrap();
        let r = start(&mut c, &a, T0 + MIN).unwrap();
        assert_eq!(r.active.started_at, T0);
        assert_eq!(list(&c, &a).unwrap().len(), 1);
    }

    #[test]
    fn active_reports_previous_time() {
        let (db, a, _, _) = setup();
        let mut c = db.conn();
        start(&mut c, &a, T0).unwrap();
        pause(&c, &a, T0 + 30 * MIN).unwrap();
        start(&mut c, &a, T0 + HOUR).unwrap();
        let act = active(&c).unwrap().unwrap();
        assert_eq!(act.previous_ms, 30 * MIN);
        assert_eq!(act.task_title, "A");
    }

    #[test]
    fn cannot_start_done_task() {
        let (db, a, _, _) = setup();
        let mut c = db.conn();
        tasks::complete(&mut c, &a, parse_date("2026-11-22").unwrap()).unwrap();
        assert!(matches!(
            start(&mut c, &a, T0),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn completing_task_stops_its_timer() {
        let (db, a, _, _) = setup();
        let mut c = db.conn();
        start(&mut c, &a, T0).unwrap();
        tasks::complete(&mut c, &a, parse_date("2026-11-22").unwrap()).unwrap();
        assert!(active(&c).unwrap().is_none());
    }

    #[test]
    fn deleted_task_timer_is_not_active() {
        let (db, a, _, _) = setup();
        let mut c = db.conn();
        start(&mut c, &a, T0).unwrap();
        tasks::delete(&mut c, &a).unwrap();
        assert!(active(&c).unwrap().is_none());
    }

    #[test]
    fn manual_entries_are_validated() {
        let (db, a, _, _) = setup();
        let c = db.conn();
        let now = T0 + 10 * HOUR;
        add(&c, &a, T0, T0 + HOUR, now).unwrap();
        // end before start
        assert!(add(&c, &a, T0 + 3 * HOUR, T0 + 2 * HOUR, now).is_err());
        // in the future
        assert!(add(&c, &a, now + HOUR, now + 2 * HOUR, now).is_err());
        // overlaps the first entry
        assert!(add(&c, &a, T0 + 30 * MIN, T0 + 2 * HOUR, now).is_err());
        // touching edges is fine
        add(&c, &a, T0 + HOUR, T0 + 2 * HOUR, now).unwrap();
        // longer than a day
        assert!(add(&c, &a, T0 - 30 * HOUR, T0 - HOUR, now).is_err());
        assert_eq!(closed_total_ms(&c, &a).unwrap(), 2 * HOUR);
    }

    #[test]
    fn edit_and_delete_entries() {
        let (db, a, b, _) = setup();
        let c = db.conn();
        let now = T0 + 10 * HOUR;
        let s = add(&c, &a, T0, T0 + HOUR, now).unwrap();
        // Sessions of a different task may overlap (only one *runs* at a time).
        add(&c, &b, T0, T0 + HOUR, now).unwrap();
        let u = update(&c, &s.id, T0, Some(T0 + 2 * HOUR), now).unwrap();
        assert_eq!(u.ended_at, Some(T0 + 2 * HOUR));
        assert!(update(&c, &s.id, T0, None, now).is_err());
        delete(&c, &s.id).unwrap();
        assert_eq!(closed_total_ms(&c, &a).unwrap(), 0);
    }

    #[test]
    fn manual_entry_cannot_overlap_running_session() {
        let (db, a, _, _) = setup();
        let mut c = db.conn();
        start(&mut c, &a, T0).unwrap();
        assert!(add(&c, &a, T0 + MIN, T0 + 2 * MIN, T0 + HOUR).is_err());
        assert!(add(&c, &a, T0 - HOUR, T0, T0 + HOUR).is_ok());
    }
}
