//! Workspaces: user-defined groups such as "Work" and "Personal".

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::time::now_ms;

pub const MAX_NAME_LEN: usize = 40;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

fn from_row(r: &Row) -> rusqlite::Result<Workspace> {
    Ok(Workspace {
        id: r.get("id")?,
        name: r.get("name")?,
        color: r.get("color")?,
        sort_order: r.get("sort_order")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

const COLS: &str = "id, name, color, sort_order, created_at, updated_at";

pub fn list(conn: &Connection) -> AppResult<Vec<Workspace>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM workspaces WHERE deleted_at IS NULL ORDER BY sort_order, created_at"
    ))?;
    let rows = stmt
        .query_map([], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Workspace> {
    conn.query_row(
        &format!("SELECT {COLS} FROM workspaces WHERE id = ?1 AND deleted_at IS NULL"),
        [id],
        from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound("workspace"))
}

pub fn create(conn: &Connection, name: &str, color: &str) -> AppResult<Workspace> {
    let name = validate_name(name)?;
    let color = validate_color(color)?;
    ensure_unique_name(conn, &name, None)?;

    let next_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM workspaces WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    conn.execute(
        "INSERT INTO workspaces (id, name, color, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, name, color, next_order, now],
    )?;
    get(conn, &id)
}

pub fn update(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    color: Option<&str>,
) -> AppResult<Workspace> {
    let current = get(conn, id)?;
    let name = match name {
        Some(n) => {
            let n = validate_name(n)?;
            ensure_unique_name(conn, &n, Some(id))?;
            n
        }
        None => current.name,
    };
    let color = match color {
        Some(c) => validate_color(c)?,
        None => current.color,
    };
    conn.execute(
        "UPDATE workspaces SET name = ?2, color = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, name, color, now_ms()],
    )?;
    get(conn, id)
}

/// Sets the order to exactly `ids`, which must contain every active workspace once.
pub fn reorder(conn: &mut Connection, ids: &[String]) -> AppResult<Vec<Workspace>> {
    let mut current: Vec<String> = list(conn)?.into_iter().map(|w| w.id).collect();
    let mut given = ids.to_vec();
    current.sort();
    given.sort();
    if current != given {
        return Err(AppError::Validation(
            "Reorder must include every workspace exactly once".into(),
        ));
    }
    let now = now_ms();
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE workspaces SET sort_order = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, i as i64, now],
        )?;
    }
    tx.commit()?;
    list(conn)
}

/// Soft-deletes the workspace together with its tasks and notes, and stops any
/// timer running on one of its tasks. One transaction, so it is all or nothing.
pub fn delete(conn: &mut Connection, id: &str) -> AppResult<()> {
    get(conn, id)?;
    let now = now_ms();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE time_sessions SET ended_at = ?2
         WHERE ended_at IS NULL
           AND task_id IN (SELECT id FROM tasks WHERE workspace_id = ?1)",
        params![id, now],
    )?;
    tx.execute(
        "UPDATE tasks SET deleted_at = ?2, updated_at = ?2
         WHERE workspace_id = ?1 AND deleted_at IS NULL",
        params![id, now],
    )?;
    tx.execute(
        "UPDATE notes SET deleted_at = ?2, updated_at = ?2
         WHERE workspace_id = ?1 AND deleted_at IS NULL",
        params![id, now],
    )?;
    tx.execute(
        "UPDATE workspaces SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )?;
    tx.commit()?;
    Ok(())
}

fn validate_name(name: &str) -> AppResult<String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(AppError::Validation("Workspace name can't be empty".into()));
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(AppError::Validation(format!(
            "Workspace name must be at most {MAX_NAME_LEN} characters"
        )));
    }
    Ok(name)
}

fn validate_color(color: &str) -> AppResult<String> {
    let c = color.trim();
    let ok = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
    if !ok {
        return Err(AppError::Validation("Color must look like #RRGGBB".into()));
    }
    Ok(c.to_ascii_uppercase())
}

fn ensure_unique_name(conn: &Connection, name: &str, except_id: Option<&str>) -> AppResult<()> {
    let clash: Option<String> = conn
        .query_row(
            "SELECT name FROM workspaces
             WHERE deleted_at IS NULL AND name = ?1 COLLATE NOCASE AND id != COALESCE(?2, '')",
            params![name, except_id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(existing) = clash {
        return Err(AppError::Conflict(format!(
            "You already have a workspace called \"{existing}\""
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn db() -> Db {
        Db::open_in_memory().unwrap()
    }

    #[test]
    fn create_and_list_in_order() {
        let db = db();
        let c = db.conn();
        create(&c, "Work", "#534ab7").unwrap();
        create(&c, "  Personal   life ", "#0F6E56").unwrap();
        let all = list(&c).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "Work");
        assert_eq!(all[0].color, "#534AB7");
        assert_eq!(all[1].name, "Personal life");
        assert_eq!(all[1].sort_order, 1);
    }

    #[test]
    fn rejects_bad_input() {
        let db = db();
        let c = db.conn();
        assert!(matches!(
            create(&c, "   ", "#000000"),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            create(&c, &"x".repeat(41), "#000000"),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            create(&c, "Work", "red"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn names_are_unique_case_insensitive() {
        let db = db();
        let c = db.conn();
        let w = create(&c, "Work", "#000000").unwrap();
        assert!(matches!(
            create(&c, "work", "#000000"),
            Err(AppError::Conflict(_))
        ));
        // Renaming to its own name (different case) is fine.
        assert_eq!(update(&c, &w.id, Some("WORK"), None).unwrap().name, "WORK");
    }

    #[test]
    fn update_changes_only_given_fields() {
        let db = db();
        let c = db.conn();
        let w = create(&c, "Work", "#000000").unwrap();
        let u = update(&c, &w.id, None, Some("#ffffff")).unwrap();
        assert_eq!(u.name, "Work");
        assert_eq!(u.color, "#FFFFFF");
    }

    #[test]
    fn reorder_requires_full_set() {
        let db = db();
        let mut c = db.conn();
        let a = create(&c, "A", "#000000").unwrap();
        let b = create(&c, "B", "#000000").unwrap();
        let ordered = reorder(&mut c, &[b.id.clone(), a.id.clone()]).unwrap();
        assert_eq!(ordered[0].name, "B");
        assert!(reorder(&mut c, std::slice::from_ref(&a.id)).is_err());
    }

    #[test]
    fn delete_cascades_and_frees_name() {
        let db = db();
        let mut c = db.conn();
        let w = create(&c, "Work", "#000000").unwrap();
        c.execute(
            "INSERT INTO tasks (id, workspace_id, title, original_date, created_at, updated_at)
             VALUES ('t1', ?1, 'Task', '2026-11-22', 0, 0)",
            [&w.id],
        )
        .unwrap();
        c.execute(
            "INSERT INTO time_sessions (id, task_id, started_at) VALUES ('s1', 't1', 0)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO notes (id, workspace_id, sort_order, created_at, updated_at)
             VALUES ('n1', ?1, 0, 0, 0)",
            [&w.id],
        )
        .unwrap();

        delete(&mut c, &w.id).unwrap();

        assert!(list(&c).unwrap().is_empty());
        let task_deleted: Option<i64> = c
            .query_row("SELECT deleted_at FROM tasks WHERE id='t1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let note_deleted: Option<i64> = c
            .query_row("SELECT deleted_at FROM notes WHERE id='n1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let session_ended: Option<i64> = c
            .query_row(
                "SELECT ended_at FROM time_sessions WHERE id='s1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(task_deleted.is_some() && note_deleted.is_some() && session_ended.is_some());
        // The name can be reused after deletion.
        create(&c, "Work", "#000000").unwrap();
    }

    #[test]
    fn missing_workspace_is_not_found() {
        let db = db();
        let mut c = db.conn();
        assert!(matches!(get(&c, "nope"), Err(AppError::NotFound(_))));
        assert!(matches!(delete(&mut c, "nope"), Err(AppError::NotFound(_))));
    }
}
