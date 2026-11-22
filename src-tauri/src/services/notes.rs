//! The Remember panel: a permanent, per-workspace table of label / value / note
//! rows for things you don't want to forget (file names, paths, decisions).
//! Not tied to dates. Plain text: for *where* a secret is, never the secret.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::services::workspaces;
use crate::time::now_ms;

pub const MAX_LABEL: usize = 200;
pub const MAX_VALUE: usize = 2000;
pub const MAX_NOTE: usize = 2000;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub workspace_id: String,
    pub label: String,
    pub value: String,
    pub note: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

const COLS: &str = "id, workspace_id, label, value, note, sort_order, created_at, updated_at";

fn from_row(r: &Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get("id")?,
        workspace_id: r.get("workspace_id")?,
        label: r.get("label")?,
        value: r.get("value")?,
        note: r.get("note")?,
        sort_order: r.get("sort_order")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

pub fn list(conn: &Connection, workspace_id: &str) -> AppResult<Vec<Note>> {
    workspaces::get(conn, workspace_id)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM notes WHERE workspace_id = ?1 AND deleted_at IS NULL
         ORDER BY sort_order, created_at"
    ))?;
    let rows = stmt
        .query_map([workspace_id], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Note> {
    conn.query_row(
        &format!("SELECT {COLS} FROM notes WHERE id = ?1 AND deleted_at IS NULL"),
        [id],
        from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound("row"))
}

pub fn create(
    conn: &Connection,
    workspace_id: &str,
    label: &str,
    value: &str,
    note: &str,
) -> AppResult<Note> {
    workspaces::get(conn, workspace_id)?;
    let (label, value, note) = validate(label, value, note)?;
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM notes
         WHERE workspace_id = ?1 AND deleted_at IS NULL",
        [workspace_id],
        |r| r.get(0),
    )?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    conn.execute(
        "INSERT INTO notes (id, workspace_id, label, value, note, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![id, workspace_id, label, value, note, next, now],
    )?;
    get(conn, &id)
}

/// Updates the given fields. `updated_at` only changes if something actually changed.
pub fn update(
    conn: &Connection,
    id: &str,
    label: Option<&str>,
    value: Option<&str>,
    note: Option<&str>,
) -> AppResult<Note> {
    let cur = get(conn, id)?;
    let (label, value, note) = validate(
        label.unwrap_or(&cur.label),
        value.unwrap_or(&cur.value),
        note.unwrap_or(&cur.note),
    )?;
    if label == cur.label && value == cur.value && note == cur.note {
        return Ok(cur);
    }
    conn.execute(
        "UPDATE notes SET label = ?2, value = ?3, note = ?4, updated_at = ?5 WHERE id = ?1",
        params![id, label, value, note, now_ms()],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    get(conn, id)?;
    let now = now_ms();
    conn.execute(
        "UPDATE notes SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )?;
    Ok(())
}

/// Undo for `delete`. Fails if the workspace was deleted meanwhile.
pub fn restore(conn: &Connection, id: &str) -> AppResult<Note> {
    let ws: Option<String> = conn
        .query_row("SELECT workspace_id FROM notes WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    workspaces::get(conn, &ws.ok_or(AppError::NotFound("row"))?)?;
    conn.execute(
        "UPDATE notes SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    get(conn, id)
}

/// Sets the order to exactly `ids` (every row of the workspace once).
pub fn reorder(conn: &mut Connection, workspace_id: &str, ids: &[String]) -> AppResult<Vec<Note>> {
    let mut current: Vec<String> = list(conn, workspace_id)?
        .into_iter()
        .map(|n| n.id)
        .collect();
    let mut given = ids.to_vec();
    current.sort();
    given.sort();
    if current != given {
        return Err(AppError::Validation(
            "Reorder must include every row exactly once".into(),
        ));
    }
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE notes SET sort_order = ?2 WHERE id = ?1",
            params![id, i as i64],
        )?;
    }
    tx.commit()?;
    list(conn, workspace_id)
}

fn validate(label: &str, value: &str, note: &str) -> AppResult<(String, String, String)> {
    let label = label.trim().to_string();
    // Values are kept exactly (minus surrounding whitespace): they may be paths or commands.
    let value = value.trim().to_string();
    let note = note.trim().to_string();
    if label.is_empty() && value.is_empty() {
        return Err(AppError::Validation("Add a label or a value".into()));
    }
    for (field, text, max) in [
        ("Label", &label, MAX_LABEL),
        ("Value", &value, MAX_VALUE),
        ("Note", &note, MAX_NOTE),
    ] {
        if text.chars().count() > max {
            return Err(AppError::Validation(format!(
                "{field} must be at most {max} characters"
            )));
        }
    }
    Ok((label, value, note))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn setup() -> (Db, String) {
        let db = Db::open_in_memory().unwrap();
        let ws = workspaces::create(&db.conn(), "Work", "#000000")
            .unwrap()
            .id;
        (db, ws)
    }

    #[test]
    fn create_list_in_order() {
        let (db, ws) = setup();
        let c = db.conn();
        create(&c, &ws, " Secrets file ", "secrets.env", "temp name").unwrap();
        create(&c, &ws, "Main script", "abcd.py", "").unwrap();
        let all = list(&c, &ws).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].label, "Secrets file");
        assert_eq!(all[1].sort_order, 1);
    }

    #[test]
    fn needs_label_or_value_and_respects_limits() {
        let (db, ws) = setup();
        let c = db.conn();
        assert!(create(&c, &ws, "  ", "", "just a note").is_err());
        assert!(create(&c, &ws, "", "value only", "").is_ok());
        assert!(create(&c, &ws, &"x".repeat(201), "", "").is_err());
    }

    #[test]
    fn update_changes_fields_and_timestamp_only_when_changed() {
        let (db, ws) = setup();
        let c = db.conn();
        let n = create(&c, &ws, "Secrets file", "secrets", "").unwrap();
        let same = update(&c, &n.id, Some("Secrets file"), None, None).unwrap();
        assert_eq!(same.updated_at, n.updated_at);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let u = update(&c, &n.id, None, Some("secrets.env"), Some("renamed")).unwrap();
        assert_eq!(u.value, "secrets.env");
        assert_eq!(u.note, "renamed");
        assert!(u.updated_at > n.updated_at);
        assert!(update(&c, &n.id, Some(""), Some(""), None).is_err());
    }

    #[test]
    fn workspaces_are_separate() {
        let (db, ws) = setup();
        let c = db.conn();
        let other = workspaces::create(&c, "Personal", "#000000").unwrap().id;
        create(&c, &ws, "Work thing", "", "").unwrap();
        create(&c, &other, "Personal thing", "", "").unwrap();
        assert_eq!(list(&c, &ws).unwrap()[0].label, "Work thing");
        assert_eq!(list(&c, &other).unwrap()[0].label, "Personal thing");
    }

    #[test]
    fn delete_restore_and_reorder() {
        let (db, ws) = setup();
        let mut c = db.conn();
        let a = create(&c, &ws, "A", "", "").unwrap();
        let b = create(&c, &ws, "B", "", "").unwrap();
        delete(&c, &a.id).unwrap();
        assert_eq!(list(&c, &ws).unwrap().len(), 1);
        restore(&c, &a.id).unwrap();
        let r = reorder(&mut c, &ws, &[b.id.clone(), a.id.clone()]).unwrap();
        assert_eq!(r[0].label, "B");
        assert!(reorder(&mut c, &ws, std::slice::from_ref(&a.id)).is_err());
    }

    #[test]
    fn rows_go_away_with_their_workspace() {
        let (db, ws) = setup();
        let mut c = db.conn();
        let n = create(&c, &ws, "A", "", "").unwrap();
        workspaces::delete(&mut c, &ws).unwrap();
        assert!(get(&c, &n.id).is_err());
        assert!(restore(&c, &n.id).is_err());
    }
}
