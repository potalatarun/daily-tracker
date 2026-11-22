//! Export / import of all data as one JSON file, and "delete my data".
//!
//! The export is a plain, readable JSON document so people can keep it as a
//! backup or move to a new computer. Import *replaces* everything in a single
//! transaction: if anything in the file is invalid, nothing changes.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db;
use crate::error::{AppError, AppResult};
use crate::time::now_ms;

pub const FORMAT: &str = "daily-tracker-export";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Export {
    pub format: String,
    pub format_version: u32,
    pub app_version: String,
    pub exported_at: i64,
    pub workspaces: Vec<WorkspaceRow>,
    pub tasks: Vec<TaskRow>,
    pub time_sessions: Vec<SessionRow>,
    pub reminders: Vec<ReminderRow>,
    pub notes: Vec<NoteRow>,
    pub settings: Vec<SettingRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRow {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: String,
    pub task_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReminderRow {
    pub id: String,
    pub task_id: String,
    pub time: String,
    pub enabled: bool,
    pub last_fired_on: Option<String>,
    pub snoozed_until: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteRow {
    pub id: String,
    pub workspace_id: String,
    pub label: String,
    pub value: String,
    pub note: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub workspaces: usize,
    pub tasks: usize,
    pub notes: usize,
}

/// Everything that is not deleted.
pub fn export(conn: &Connection, app_version: &str) -> AppResult<Export> {
    fn rows<T>(
        conn: &Connection,
        sql: &str,
        f: impl FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
    ) -> AppResult<Vec<T>> {
        let mut stmt = conn.prepare(sql)?;
        let v = stmt.query_map([], f)?.collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    }
    // Active tasks = not deleted and in an active workspace.
    const ACTIVE_TASKS: &str = "SELECT t.id FROM tasks t JOIN workspaces w ON w.id = t.workspace_id
                                WHERE t.deleted_at IS NULL AND w.deleted_at IS NULL";

    Ok(Export {
        format: FORMAT.into(),
        format_version: FORMAT_VERSION,
        app_version: app_version.into(),
        exported_at: now_ms(),
        workspaces: rows(
            conn,
            "SELECT id, name, color, sort_order, created_at, updated_at FROM workspaces
             WHERE deleted_at IS NULL ORDER BY sort_order",
            |r| {
                Ok(WorkspaceRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                    sort_order: r.get(3)?,
                    created_at: r.get(4)?,
                    updated_at: r.get(5)?,
                })
            },
        )?,
        tasks: rows(
            conn,
            &format!(
                "SELECT id, workspace_id, title, description, original_date, status,
                        completed_at, completed_on, created_at, updated_at
                 FROM tasks WHERE id IN ({ACTIVE_TASKS}) ORDER BY original_date, created_at"
            ),
            |r| {
                Ok(TaskRow {
                    id: r.get(0)?,
                    workspace_id: r.get(1)?,
                    title: r.get(2)?,
                    description: r.get(3)?,
                    original_date: r.get(4)?,
                    status: r.get(5)?,
                    completed_at: r.get(6)?,
                    completed_on: r.get(7)?,
                    created_at: r.get(8)?,
                    updated_at: r.get(9)?,
                })
            },
        )?,
        time_sessions: rows(
            conn,
            &format!(
                "SELECT id, task_id, started_at, ended_at FROM time_sessions
                 WHERE task_id IN ({ACTIVE_TASKS}) ORDER BY started_at"
            ),
            |r| {
                Ok(SessionRow {
                    id: r.get(0)?,
                    task_id: r.get(1)?,
                    started_at: r.get(2)?,
                    ended_at: r.get(3)?,
                })
            },
        )?,
        reminders: rows(
            conn,
            &format!(
                "SELECT id, task_id, time, enabled, last_fired_on, snoozed_until FROM reminders
                 WHERE task_id IN ({ACTIVE_TASKS}) ORDER BY task_id, time"
            ),
            |r| {
                Ok(ReminderRow {
                    id: r.get(0)?,
                    task_id: r.get(1)?,
                    time: r.get(2)?,
                    enabled: r.get::<_, i64>(3)? != 0,
                    last_fired_on: r.get(4)?,
                    snoozed_until: r.get(5)?,
                })
            },
        )?,
        notes: rows(
            conn,
            "SELECT n.id, n.workspace_id, n.label, n.value, n.note, n.sort_order, n.created_at, n.updated_at
             FROM notes n JOIN workspaces w ON w.id = n.workspace_id
             WHERE n.deleted_at IS NULL AND w.deleted_at IS NULL ORDER BY n.workspace_id, n.sort_order",
            |r| {
                Ok(NoteRow {
                    id: r.get(0)?,
                    workspace_id: r.get(1)?,
                    label: r.get(2)?,
                    value: r.get(3)?,
                    note: r.get(4)?,
                    sort_order: r.get(5)?,
                    created_at: r.get(6)?,
                    updated_at: r.get(7)?,
                })
            },
        )?,
        settings: rows(conn, "SELECT key, value FROM settings ORDER BY key", |r| {
            Ok(SettingRow {
                key: r.get(0)?,
                value: r.get(1)?,
            })
        })?,
    })
}

/// Parses and checks an export file before anything is changed.
pub fn parse(json: &str) -> AppResult<Export> {
    let data: Export = serde_json::from_str(json).map_err(|e| {
        AppError::Validation(format!(
            "This doesn't look like a Daily Tracker export ({e})"
        ))
    })?;
    if data.format != FORMAT {
        return Err(AppError::Validation(
            "This file isn't a Daily Tracker export".into(),
        ));
    }
    if data.format_version > FORMAT_VERSION {
        return Err(AppError::Validation(
            "This export was made by a newer version of Daily Tracker. Update the app first."
                .into(),
        ));
    }
    Ok(data)
}

/// Replaces all data with `data`. One transaction: any error (bad reference,
/// invalid value) rolls everything back and the current data stays untouched.
pub fn import(conn: &mut Connection, data: &Export) -> AppResult<ImportSummary> {
    let tx = conn.transaction()?;
    wipe_tables(&tx)?;
    for w in &data.workspaces {
        tx.execute(
            "INSERT INTO workspaces (id, name, color, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                w.id,
                w.name,
                w.color,
                w.sort_order,
                w.created_at,
                w.updated_at
            ],
        )?;
    }
    for t in &data.tasks {
        tx.execute(
            "INSERT INTO tasks (id, workspace_id, title, description, original_date, status,
                                completed_at, completed_on, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                t.id,
                t.workspace_id,
                t.title,
                t.description,
                t.original_date,
                t.status,
                t.completed_at,
                t.completed_on,
                t.created_at,
                t.updated_at
            ],
        )?;
    }
    for s in &data.time_sessions {
        tx.execute(
            "INSERT INTO time_sessions (id, task_id, started_at, ended_at) VALUES (?1, ?2, ?3, ?4)",
            params![s.id, s.task_id, s.started_at, s.ended_at],
        )?;
    }
    for r in &data.reminders {
        tx.execute(
            "INSERT INTO reminders (id, task_id, time, enabled, last_fired_on, snoozed_until)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                r.id,
                r.task_id,
                r.time,
                r.enabled as i64,
                r.last_fired_on,
                r.snoozed_until
            ],
        )?;
    }
    for n in &data.notes {
        tx.execute(
            "INSERT INTO notes (id, workspace_id, label, value, note, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                n.id,
                n.workspace_id,
                n.label,
                n.value,
                n.note,
                n.sort_order,
                n.created_at,
                n.updated_at
            ],
        )?;
    }
    for s in &data.settings {
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params![s.key, s.value],
        )?;
    }
    // Foreign keys are checked per statement, but be explicit about it.
    let broken: i64 = tx.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
        r.get(0)
    })?;
    if broken > 0 {
        return Err(AppError::Validation(
            "The file has rows that point to missing items".into(),
        ));
    }
    tx.commit()?;
    Ok(ImportSummary {
        workspaces: data.workspaces.len(),
        tasks: data.tasks.len(),
        notes: data.notes.len(),
    })
}

/// Deletes every row in every table (the schema stays).
pub fn wipe(conn: &mut Connection) -> AppResult<()> {
    let tx = conn.transaction()?;
    wipe_tables(&tx)?;
    tx.commit()?;
    // Reclaim the space so deleted text doesn't linger in the file.
    conn.execute_batch("VACUUM")?;
    Ok(())
}

fn wipe_tables(conn: &Connection) -> AppResult<()> {
    // Children first because of foreign keys.
    conn.execute_batch(
        "DELETE FROM time_sessions; DELETE FROM reminders; DELETE FROM notes;
         DELETE FROM tasks; DELETE FROM workspaces; DELETE FROM settings;",
    )?;
    Ok(())
}

/// Snapshot of the database before a destructive action.
pub fn snapshot(
    conn: &Connection,
    dir: &std::path::Path,
    reason: &str,
) -> AppResult<std::path::PathBuf> {
    db::snapshot(conn, dir, reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::services::{notes, reminders, tasks, timer, workspaces};
    use crate::time::parse_date;

    fn populated() -> Db {
        let db = Db::open_in_memory().unwrap();
        {
            let mut c = db.conn();
            let ws = workspaces::create(&c, "Work", "#7F77DD").unwrap().id;
            let gone = workspaces::create(&c, "Gone", "#000000").unwrap().id;
            let t = tasks::create(&c, &ws, "2026-11-20", "Fix bug", "details")
                .unwrap()
                .id;
            let t2 = tasks::create(&c, &ws, "2026-11-21", "Deleted task", "")
                .unwrap()
                .id;
            tasks::create(&c, &gone, "2026-11-21", "In deleted ws", "").unwrap();
            timer::add(&c, &t, 1_000, 61_000, 10_000_000).unwrap();
            reminders::set_for_task(
                &mut c,
                &t,
                &["15:00".into()],
                parse_date("2026-11-22").unwrap(),
                "09:00",
            )
            .unwrap();
            notes::create(&c, &ws, "Secrets file", "secrets.env", "").unwrap();
            tasks::delete(&mut c, &t2).unwrap();
            workspaces::delete(&mut c, &gone).unwrap();
            c.execute(
                "INSERT INTO settings (key, value) VALUES ('default_snooze_minutes', '15')",
                [],
            )
            .unwrap();
        }
        db
    }

    #[test]
    fn export_contains_only_live_data() {
        let db = populated();
        let e = export(&db.conn(), "1.0.0").unwrap();
        assert_eq!(e.workspaces.len(), 1);
        assert_eq!(e.tasks.len(), 1);
        assert_eq!(e.tasks[0].title, "Fix bug");
        assert_eq!(e.time_sessions.len(), 1);
        assert_eq!(e.reminders.len(), 1);
        assert_eq!(e.notes.len(), 1);
        assert_eq!(e.settings.len(), 1);
    }

    #[test]
    fn round_trip_into_another_database() {
        let src = populated();
        let json = serde_json::to_string_pretty(&export(&src.conn(), "1.0.0").unwrap()).unwrap();
        let dst = Db::open_in_memory().unwrap();
        workspaces::create(&dst.conn(), "Old data", "#000000").unwrap();
        let parsed = parse(&json).unwrap();
        let summary = import(&mut dst.conn(), &parsed).unwrap();
        assert_eq!(
            summary,
            ImportSummary {
                workspaces: 1,
                tasks: 1,
                notes: 1
            }
        );
        // Old data replaced, new data identical.
        let again = export(&dst.conn(), "1.0.0").unwrap();
        assert_eq!(again.workspaces, parsed.workspaces);
        assert_eq!(again.tasks, parsed.tasks);
        assert_eq!(again.reminders, parsed.reminders);
        assert_eq!(again.time_sessions, parsed.time_sessions);
        assert_eq!(again.notes, parsed.notes);
    }

    #[test]
    fn bad_files_are_rejected_without_changes() {
        let db = populated();
        let before = export(&db.conn(), "1").unwrap();
        assert!(parse("not json").is_err());
        assert!(parse(r#"{"format":"other"}"#).is_err());

        let mut newer = before.clone();
        newer.format_version = 99;
        assert!(parse(&serde_json::to_string(&newer).unwrap()).is_err());

        // A task pointing at a missing workspace: import fails, nothing changes.
        let mut broken = before.clone();
        broken.tasks[0].workspace_id = "missing".into();
        assert!(import(&mut db.conn(), &broken).is_err());
        let after = export(&db.conn(), "1").unwrap();
        assert_eq!(after.tasks, before.tasks);
        assert_eq!(after.workspaces, before.workspaces);
    }

    #[test]
    fn wipe_removes_everything() {
        let db = populated();
        wipe(&mut db.conn()).unwrap();
        let e = export(&db.conn(), "1").unwrap();
        assert!(
            e.workspaces.is_empty()
                && e.tasks.is_empty()
                && e.notes.is_empty()
                && e.settings.is_empty()
        );
        let raw: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(raw, 0);
    }
}
