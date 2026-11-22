-- 0001: initial schema. See docs/ARCHITECTURE.md §4 for conventions:
--   * ids are UUID text
--   * instants are UTC milliseconds (INTEGER)
--   * calendar values are local 'YYYY-MM-DD' / 'HH:MM' (TEXT)
--   * mutable rows carry updated_at and deleted_at (soft delete)

CREATE TABLE workspaces (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL,
    sort_order  INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    deleted_at  INTEGER
);

CREATE TABLE tasks (
    id             TEXT PRIMARY KEY NOT NULL,
    workspace_id   TEXT NOT NULL REFERENCES workspaces(id),
    title          TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    original_date  TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'done')),
    completed_at   INTEGER,
    completed_on   TEXT,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL,
    deleted_at     INTEGER
);
CREATE INDEX idx_tasks_day       ON tasks(workspace_id, original_date, status);
CREATE INDEX idx_tasks_completed ON tasks(workspace_id, completed_on);

CREATE TABLE time_sessions (
    id          TEXT PRIMARY KEY NOT NULL,
    task_id     TEXT NOT NULL REFERENCES tasks(id),
    started_at  INTEGER NOT NULL,
    ended_at    INTEGER
);
CREATE INDEX idx_sessions_task ON time_sessions(task_id);

CREATE TABLE reminders (
    id             TEXT PRIMARY KEY NOT NULL,
    task_id        TEXT NOT NULL REFERENCES tasks(id),
    time           TEXT NOT NULL,
    enabled        INTEGER NOT NULL DEFAULT 1,
    last_fired_on  TEXT,
    snoozed_until  INTEGER
);
CREATE INDEX idx_reminders_task ON reminders(task_id);

CREATE TABLE notes (
    id            TEXT PRIMARY KEY NOT NULL,
    workspace_id  TEXT NOT NULL REFERENCES workspaces(id),
    label         TEXT NOT NULL DEFAULT '',
    value         TEXT NOT NULL DEFAULT '',
    note          TEXT NOT NULL DEFAULT '',
    sort_order    INTEGER NOT NULL,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    deleted_at    INTEGER
);
CREATE INDEX idx_notes_workspace ON notes(workspace_id);

CREATE TABLE settings (
    key    TEXT PRIMARY KEY NOT NULL,
    value  TEXT NOT NULL
);
