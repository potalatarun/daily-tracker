# Daily Tracker — Product Spec (v1)

What the app does, from the user's point of view. For how it's built, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Main window

```
┌────────────┬──────────────────────────────┬─────────────────┐
│ Workspaces │  ◀  Sun, Nov 22  ▶   [+ Add] │  Remember       │
│ • Work     │                              │  [search…]      │
│ • Personal │  ☐ Fix login bug             │  Label │ Value  │
│ + New      │    Nov 18 · carried 4 days   │  ──────┼─────── │
│            │    ⏰ 3:00 PM   ⏱ 1h 20m      │  ...   │ ...    │
│ Calendar   │  ☑ Write report   ✓ 45m      │  + Row          │
└────────────┴──────────────────────────────┴─────────────────┘
```

## Workspaces
- Users create their own (Work and Personal suggested on first run).
- Rename, recolor, reorder, delete. Delete asks for confirmation and removes the workspace's tasks and notes.

## Tasks
- **Add:** select date → title → description (optional) → reminders (optional, multiple).
- Edit, delete, mark done.

## Automatic carry-over
- Unfinished tasks from earlier days appear in **Today** automatically, with no prompt.
- The original date is always kept and shown: *"Nov 18 · carried 4 days"*.
- On past days the task appears greyed: *"→ moved to today"* or *"completed Nov 22"*.
- When completed: *"done after 4 days"*.

## Time tracking (optional)
- ▶ Start · ⏸ Pause · ▶ Resume · ✓ Done.
- Total shows worked time across sessions (1 h Monday + 2 h Tuesday = 3 h).
- Time log per task: see every session, add time by hand, fix or delete entries.
- The running timer shows in the sidebar from any workspace, with a pause button.
- Only one timer runs at a time; starting another pauses the current one.
- Keeps counting if the window is closed or the laptop sleeps.

## Reminders
- Multiple per task, at chosen times.
- Repeat at the same times every day a task is carried; editable any time.
- When a reminder fires: OS notification **and** a small popup in the corner with **Done**, **Snooze** (5 / 15 / 30 / 60 min, your default, or custom: "45", "1h30", "16:30"), **Dismiss**. Clicking the notification brings the popup forward.
- Reminder times accept "15:00", "9:30", "0930" or "3pm"; up to 10 per task. Chips on the task show them (crossed out once they've fired today, 💤 while snoozed).
- Default snooze length in Settings.
- Missed reminders (PC off, app closed, asleep): one summary notification on startup or wake.

## Remember panel
- Right-hand side, separate per workspace, never tied to a date.
- Table: Label · Value (monospace, with **Copy**) · Note, plus "Updated …" on hover.
- Click any cell to edit the row; Enter saves, Esc cancels. **+ Add row**, search across all fields, Move up/down, delete with **Undo**. Collapsible.
- A row needs at least a label or a value.
- For names and locations, not passwords (stored as plain text).

## App behaviour
- Lives in the tray / menu bar; closing the window keeps reminders working.
- Start at login (on by default).
- Global shortcut: `Ctrl/Cmd + Shift + D`.
- Light/dark follows the OS.

## First launch
1. Create workspaces (Work / Personal pre-filled).
2. Allow notifications (banner with "Fix this" if denied).
3. Start at login (on by default).

## Settings
- Default snooze, start at login, open data folder, export / import (JSON), delete my data.

## Distribution
- Download page (GitHub Pages) detects the OS.
- Installers: macOS `.dmg`, Windows `.exe`, Linux `.AppImage` / `.deb`.
- Automatic, signed updates.
- Unsigned for now: one-time "open anyway" step on macOS and Windows.

## Not in v1
Cloud sync, mobile, recurring tasks, secret storage, idle detection, code signing.
