# Design Decisions

Each entry records **what we chose, what we rejected, and why**. They are kept short so a new contributor can read them all in ten minutes. When a decision changes, add a new entry that supersedes the old one instead of rewriting history.

| # | Decision | Status |
|---|---|---|
| D1 | [Local-first, no cloud in v1](#d1-local-first-no-cloud-in-v1) | Accepted |
| D2 | [Tauri instead of Electron](#d2-tauri-instead-of-electron) | Accepted |
| D3 | [Svelte + TypeScript for the UI](#d3-svelte--typescript-for-the-ui) | Accepted |
| D4 | [SQLite as the only store](#d4-sqlite-as-the-only-store) | Accepted |
| D5 | [Carry-over is a query, not a copy](#d5-carry-over-is-a-query-not-a-copy) | Accepted |
| D6 | [Scheduler in the Rust core, 30-second tick](#d6-scheduler-in-the-rust-core-30-second-tick) | Accepted |
| D7 | [Reminder popup window instead of notification buttons](#d7-reminder-popup-window-instead-of-notification-buttons) | Accepted |
| D8 | [Time tracking as sessions with timestamps](#d8-time-tracking-as-sessions-with-timestamps) | Accepted |
| D9 | [One running timer at a time](#d9-one-running-timer-at-a-time) | Accepted |
| D10 | [Missed reminders as one summary](#d10-missed-reminders-as-one-summary) | Accepted |
| D11 | [UTC instants, local calendar values](#d11-utc-instants-local-calendar-values) | Accepted |
| D12 | [Sync-ready IDs and soft deletes](#d12-sync-ready-ids-and-soft-deletes) | Accepted |
| D13 | [Remember panel per workspace, plain text](#d13-remember-panel-per-workspace-plain-text) | Accepted |
| D14 | [Installers from CI, unsigned for now](#d14-installers-from-ci-unsigned-for-now) | Accepted |
| D15 | [GitHub Pages download page](#d15-github-pages-download-page) | Accepted |
| D16 | [Signed auto-updates from GitHub Releases](#d16-signed-auto-updates-from-github-releases) | Accepted |

---

### D1. Local-first, no cloud in v1

**Chose:** all data in a local SQLite file; no server, no account.
**Rejected:** cloud backend (e.g. Supabase / Firebase) from day one.
**Why:**
- Workspaces (Work / Personal) don't need a cloud; they're just a column.
- The real benefit of cloud is multi-device sync, which isn't needed yet.
- Cloud adds auth, hosting cost, offline handling, security risk, and may conflict with employer policies on storing company work data on personal servers.
- Reliable OS notifications need a native process anyway, so a cloud version would *still* need this desktop app.

**Trade-off accepted:** no sync between devices. Mitigated by JSON export/import, and by D12 keeping the door open.

### D2. Tauri instead of Electron

**Chose:** Tauri 2 (Rust core + system web view).
**Rejected:** Electron; Python (Tkinter / pywebview); separate native apps per OS; Flutter.

| | Tauri | Electron |
|---|---|---|
| Installer | ~5–10 MB | ~100–150 MB |
| Memory | ~30–60 MB | ~150–250 MB |
| Startup | Near instant | 1–2 s |
| Tray, notifications, autostart, updater | Official plugins | Built in / packages |

**Why:** "lightweight and fast" was a top requirement, and Tauri's official plugins cover every OS feature we need.
**Trade-off accepted:** building from source needs the Rust toolchain. Non-developers never build from source (D14), so this only affects contributors.

### D3. Svelte + TypeScript for the UI

**Chose:** Svelte with TypeScript, plain CSS.
**Rejected:** React, Vue, UI component libraries.
**Why:** Svelte compiles away its framework, giving the smallest bundle and fastest first paint. The UI is small (one main view, one form, one popup, one panel), so a component library would add more weight than value. TypeScript catches mismatches between the UI and Rust command signatures.

### D4. SQLite as the only store

**Chose:** one SQLite file in the OS app-data folder, WAL mode, numbered migrations.
**Rejected:** JSON files, IndexedDB/localStorage in the web view.
**Why:** transactions (needed for "start timer = stop other timer" atomically), indexes for fast day queries, crash safety, and one file that's easy to back up. Web-view storage can be cleared by the OS and isn't accessible from the Rust scheduler.

### D5. Carry-over is a query, not a copy

**Chose:** a task keeps `original_date` forever; the Today view selects pending tasks with `original_date < today`. Carry count is computed as `today − original_date`.
**Rejected:** (a) copying unfinished tasks into the next day at midnight; (b) updating the task's date and incrementing a counter.
**Why:**
- No midnight job means nothing to fail, run twice, or miss while the laptop is off.
- No duplicates, so time sessions and reminders stay attached to one task.
- The count is always right, even after a week with the app closed.
- History stays honest: past days show the task greyed "moved to today" or "completed on Nov 22".

### D6. Scheduler in the Rust core, 30-second tick

**Chose:** one background loop in the Rust process, waking every 30 s.
**Rejected:** JavaScript `setTimeout` in the UI; OS-level scheduled notifications.
**Why:** JS timers stop or get throttled when the window is hidden or closed. OS-scheduled notifications differ across platforms and can't express "repeat daily while the task is pending" or custom snooze. A 30 s tick is accurate enough for human reminders and uses effectively no CPU.
**Trade-off accepted:** a reminder may arrive up to ~30 s late.

### D7. Reminder popup window instead of notification buttons

**Chose:** clicking the OS notification opens a small always-on-top app window with Done / Snooze (presets + custom) / Dismiss.
**Rejected:** action buttons inside the OS notification.
**Why:** desktop notification actions and text inputs behave differently on macOS, Windows and Linux, and some don't support them at all. Custom snooze ("45 min", "until 4:30 PM") needs a text field. An app-owned window behaves the same everywhere.
**Trade-off accepted:** one extra click compared to an inline button.

### D8. Time tracking as sessions with timestamps

**Chose:** each Start→Pause is a `time_sessions` row with `started_at` / `ended_at` (UTC ms). Total is the sum.
**Rejected:** a single start/end pair (gives elapsed time, not worked time); a ticking counter (lost on crash or sleep).
**Why:** 1 h on Monday + 2 h on Tuesday correctly shows 3 h. Nothing has to run in the background to count time, so closing the app can't lose it.

### D9. One running timer at a time

**Chose:** starting a timer auto-pauses any other running timer, in the same transaction.
**Why:** people work on one thing at a time; parallel timers usually mean a forgotten pause, which inflates totals.

### D10. Missed reminders as one summary

**Chose:** on startup, or after a wake-from-sleep gap (a tick arriving more than 2 min late), show one "You missed N reminders" notification for today's missed reminders.
**Rejected:** firing every missed reminder (notification spam); silently skipping (user never knows).

### D11. UTC instants, local calendar values

**Chose:** moments in time (timer start/end, completion, snooze-until) as UTC milliseconds; calendar concepts (task date, completion date, reminder "15:00") as local wall-clock values.
**Why:** durations must not break on daylight-saving or time-zone changes, but "remind me at 3 PM" should mean 3 PM local time wherever the user is.

### D12. Sync-ready IDs and soft deletes

**Chose:** UUID primary keys, `updated_at` on every mutable row, `deleted_at` instead of hard deletes.
**Why:** costs almost nothing now and makes future cloud sync a matter of "send rows changed since X", with no ID collisions between devices.

### D13. Remember panel per workspace, plain text

**Chose:** a key/value table (label, value, note) scoped to each workspace, stored as plain text.
**Rejected (for v1):** a global panel; OS-keychain storage for sensitive values.
**Why:** Work and Personal facts stay separate, matching how the user thinks. The panel is for remembering *names and locations* (e.g. `secrets.env`), not secret values, so keychain storage is deferred. The UI shows a hint not to store passwords.

### D14. Installers from CI, unsigned for now

**Chose:** GitHub Actions builds `.dmg` (universal), `.exe` (per-user, WebView2 bootstrap), `.AppImage` and `.deb` on every version tag.
**Rejected (for now):** paid Apple Developer ID and Windows code-signing certificates.
**Why:** non-developers need double-click installers, not a toolchain. Signing costs money every year; the app can launch without it.
**Trade-off accepted:** first launch shows an OS warning (macOS: System Settings → Privacy & Security → Open Anyway; Windows: More info → Run anyway). Documented with screenshots. The workflow is ready to enable signing by adding secrets.

### D15. GitHub Pages download page

**Chose:** a static page that detects the visitor's OS and offers one "Download for …" button pointing at the latest release asset.
**Rejected:** sending users to the raw GitHub Releases page.
**Why:** a list of `.dmg`, `.exe`, `.AppImage`, `.deb` and `.sig` files confuses non-developers.

### D16. Signed auto-updates from GitHub Releases

**Chose:** Tauri updater checks `latest.json` on GitHub Releases at start and every 6 h; updates are verified against a public key embedded in the app.
**Why:** users should never have to re-download manually. Signature verification means only builds signed with the maintainer's private key (stored in GitHub Secrets, never in the repo) can be installed. This key is free and separate from OS code signing.
