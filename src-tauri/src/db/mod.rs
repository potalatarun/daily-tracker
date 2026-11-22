//! SQLite access: opening the database, pragmas and migrations.
//!
//! One connection is shared by the whole process behind a mutex. The app is
//! single-user and every query is tiny, so a pool would add complexity
//! without any measurable benefit.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::AppResult;

/// Ordered list of migrations. Index + 1 is the schema version.
/// Never edit a shipped migration; add a new file instead.
const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_init.sql")];

pub struct Db {
    conn: Mutex<Connection>,
    /// The database file (None for in-memory test databases).
    path: Option<PathBuf>,
}

impl Db {
    /// Opens (or creates) the database file, applies pragmas and runs migrations.
    /// If migrations are pending on an existing database, a backup copy is
    /// written to `<dir>/backups/` first.
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut conn = Connection::open(path)?;
        configure(&conn)?;
        let backup_dir = path.parent().map(|d| d.join("backups"));
        migrate(&mut conn, backup_dir.as_deref())?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: Some(path.to_path_buf()),
        })
    }

    /// Folder holding the database file and its `backups/` folder.
    pub fn data_dir(&self) -> Option<&Path> {
        self.path.as_deref().and_then(Path::parent)
    }

    /// In-memory database for tests.
    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let mut conn = Connection::open_in_memory()?;
        configure(&conn)?;
        migrate(&mut conn, None)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: None,
        })
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        // A poisoned lock only means a previous command panicked mid-query;
        // SQLite transactions keep the data consistent, so keep going.
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn configure(conn: &Connection) -> rusqlite::Result<()> {
    // WAL: crash-safe and lets reads proceed during writes.
    // NORMAL sync is safe with WAL and much faster than FULL.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    Ok(())
}

pub fn schema_version(conn: &Connection) -> rusqlite::Result<usize> {
    conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
        .map(|v| v as usize)
}

fn migrate(conn: &mut Connection, backup_dir: Option<&Path>) -> AppResult<()> {
    let current = schema_version(conn)?;
    let target = MIGRATIONS.len();
    if current >= target {
        return Ok(());
    }

    // Back up an existing database before changing its schema.
    if current > 0 {
        if let Some(dir) = backup_dir {
            backup(conn, dir, current)?;
        }
    }

    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

/// Writes a consistent snapshot using `VACUUM INTO` (safe while in WAL mode).
fn backup(conn: &Connection, dir: &Path, version: usize) -> AppResult<PathBuf> {
    snapshot(conn, dir, &format!("before-v{}", version + 1))
}

/// Writes a consistent copy of the database to `dir/<reason>-<timestamp>.sqlite`.
pub fn snapshot(conn: &Connection, dir: &Path, reason: &str) -> AppResult<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let file = dir.join(format!("{reason}-{}.sqlite", crate::time::now_ms()));
    conn.execute("VACUUM INTO ?1", [file.to_string_lossy()])?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_bring_schema_to_latest() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(schema_version(&db.conn()).unwrap(), MIGRATIONS.len());
    }

    #[test]
    fn reopening_is_idempotent_and_keeps_data() {
        let dir = std::env::temp_dir().join(format!("dt-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("data.sqlite");
        {
            let db = Db::open(&path).unwrap();
            db.conn()
                .execute("INSERT INTO settings(key, value) VALUES ('k', 'v')", [])
                .unwrap();
        }
        let db = Db::open(&path).unwrap();
        let v: String = db
            .conn()
            .query_row("SELECT value FROM settings WHERE key = 'k'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(v, "v");
        drop(db);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn backup_writes_snapshot() {
        let dir = std::env::temp_dir().join(format!("dt-test-{}", uuid::Uuid::new_v4()));
        let db = Db::open(&dir.join("data.sqlite")).unwrap();
        let file = backup(&db.conn(), &dir.join("backups"), 1).unwrap();
        assert!(file.exists());
        let copy = Connection::open(&file).unwrap();
        assert_eq!(schema_version(&copy).unwrap(), MIGRATIONS.len());
        drop(copy);
        drop(db);
        std::fs::remove_dir_all(dir).ok();
    }
}
