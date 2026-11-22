//! Key/value settings (default snooze, last selected workspace, ...).

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};

/// Keys the UI is allowed to read and write. Keeps typos from silently
/// creating new settings.
pub const ALLOWED_KEYS: &[&str] = &[
    "last_workspace_id",
    "remember_panel_open",
    "default_snooze_minutes",
];

pub fn get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    check_key(key)?;
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    check_key(key)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn check_key(key: &str) -> AppResult<()> {
    if ALLOWED_KEYS.contains(&key) {
        Ok(())
    } else {
        Err(AppError::Validation(format!("Unknown setting \"{key}\"")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn set_get_overwrite() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        assert_eq!(get(&c, "last_workspace_id").unwrap(), None);
        set(&c, "last_workspace_id", "a").unwrap();
        set(&c, "last_workspace_id", "b").unwrap();
        assert_eq!(get(&c, "last_workspace_id").unwrap().as_deref(), Some("b"));
    }

    #[test]
    fn unknown_key_rejected() {
        let db = Db::open_in_memory().unwrap();
        assert!(set(&db.conn(), "typo", "x").is_err());
    }
}
