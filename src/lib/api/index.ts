// Typed wrappers around the Rust commands in src-tauri/src/commands.
import { invoke } from "@tauri-apps/api/core";

export interface Workspace {
  id: string;
  name: string;
  color: string;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
}

export interface Task {
  id: string;
  workspaceId: string;
  title: string;
  description: string;
  originalDate: string;
  status: "pending" | "done";
  completedAt: number | null;
  completedOn: string | null;
  createdAt: number;
  updatedAt: number;
}

/** How a task appears on the viewed day (see ARCHITECTURE.md §5.1). */
export type TaskDisplay = "planned" | "carried" | "done" | "moved" | "completedLater";

export interface TaskView extends Task {
  display: TaskDisplay;
  carryDays: number | null;
  doneAfterDays: number | null;
  /** Finished time sessions, ms. */
  trackedMs: number;
  /** Start of the running session (UTC ms), if the timer is on. */
  runningSince: number | null;
  reminders: Reminder[];
}

export interface Reminder {
  id: string;
  taskId: string;
  /** Local "HH:MM". */
  time: string;
  enabled: boolean;
  lastFiredOn: string | null;
  snoozedUntil: number | null;
}

/** A fired reminder waiting in the popup. */
export interface Alert {
  reminderId: string;
  taskId: string;
  taskTitle: string;
  workspaceName: string;
  time: string;
  missed: boolean;
  fromSnooze: boolean;
  firedAt: number;
}

export interface Session {
  id: string;
  taskId: string;
  startedAt: number;
  endedAt: number | null;
}

export interface ActiveTimer {
  sessionId: string;
  taskId: string;
  taskTitle: string;
  taskDate: string;
  workspaceId: string;
  startedAt: number;
  previousMs: number;
}

export interface StartResult {
  active: ActiveTimer;
  pausedTaskTitle: string | null;
}

export interface DayView {
  date: string;
  today: string;
  tasks: TaskView[];
}

export interface MonthSummary {
  days: { date: string; pending: number; done: number }[];
  carriedToday: number;
}

export interface Note {
  id: string;
  workspaceId: string;
  label: string;
  value: string;
  note: string;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
}

export type SettingKey =
  | "last_workspace_id"
  | "remember_panel_open"
  | "default_snooze_minutes"
  | "auto_popup"
  | "onboarded";

export interface AppInfo {
  version: string;
  dataDir: string;
  startAtLogin: boolean;
}

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string | null;
}

export interface ImportSummary {
  workspaces: number;
  tasks: number;
  notes: number;
}

/** Shape of errors thrown by commands (see src-tauri/src/error.rs). */
export interface AppError {
  code: "validation" | "not_found" | "conflict" | "db" | "io";
  message: string;
}

export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as AppError).message);
  return String(e);
}

export const api = {
  listWorkspaces: () => invoke<Workspace[]>("list_workspaces"),
  createWorkspace: (name: string, color: string) =>
    invoke<Workspace>("create_workspace", { name, color }),
  updateWorkspace: (id: string, changes: { name?: string; color?: string }) =>
    invoke<Workspace>("update_workspace", { id, ...changes }),
  reorderWorkspaces: (ids: string[]) => invoke<Workspace[]>("reorder_workspaces", { ids }),
  deleteWorkspace: (id: string) => invoke<void>("delete_workspace", { id }),

  getToday: () => invoke<string>("get_today"),
  getDay: (workspaceId: string, date: string) => invoke<DayView>("get_day", { workspaceId, date }),
  getMonthSummary: (workspaceId: string, from: string, to: string) =>
    invoke<MonthSummary>("get_month_summary", { workspaceId, from, to }),
  createTask: (workspaceId: string, date: string, title: string, description: string) =>
    invoke<Task>("create_task", { workspaceId, date, title, description }),
  updateTask: (id: string, changes: { title?: string; description?: string; date?: string }) =>
    invoke<Task>("update_task", { id, ...changes }),
  completeTask: (id: string) => invoke<Task>("complete_task", { id }),
  reopenTask: (id: string) => invoke<Task>("reopen_task", { id }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
  restoreTask: (id: string) => invoke<Task>("restore_task", { id }),

  startTimer: (taskId: string) => invoke<StartResult>("start_timer", { taskId }),
  pauseTimer: (taskId: string) => invoke<void>("pause_timer", { taskId }),
  getActiveTimer: () => invoke<ActiveTimer | null>("get_active_timer"),
  listSessions: (taskId: string) => invoke<Session[]>("list_sessions", { taskId }),
  addSession: (taskId: string, startedAt: number, endedAt: number) =>
    invoke<Session>("add_session", { taskId, startedAt, endedAt }),
  updateSession: (id: string, startedAt: number, endedAt: number | null) =>
    invoke<Session>("update_session", { id, startedAt, endedAt }),
  deleteSession: (id: string) => invoke<void>("delete_session", { id }),

  setTaskReminders: (taskId: string, times: string[]) =>
    invoke<Reminder[]>("set_task_reminders", { taskId, times }),
  getAlerts: () => invoke<Alert[]>("get_alerts"),
  alertDone: (taskId: string) => invoke<void>("alert_done", { taskId }),
  alertSnooze: (reminderId: string, until: number) => invoke<void>("alert_snooze", { reminderId, until }),
  alertDismiss: (reminderId: string) => invoke<void>("alert_dismiss", { reminderId }),
  dismissAllAlerts: () => invoke<void>("dismiss_all_alerts"),
  showAlerts: () => invoke<void>("show_alerts"),
  showMainWindow: () => invoke<void>("show_main_window"),

  listNotes: (workspaceId: string) => invoke<Note[]>("list_notes", { workspaceId }),
  createNote: (workspaceId: string, label: string, value: string, note: string) =>
    invoke<Note>("create_note", { workspaceId, label, value, note }),
  updateNote: (id: string, changes: { label?: string; value?: string; note?: string }) =>
    invoke<Note>("update_note", { id, ...changes }),
  deleteNote: (id: string) => invoke<void>("delete_note", { id }),
  restoreNote: (id: string) => invoke<Note>("restore_note", { id }),
  reorderNotes: (workspaceId: string, ids: string[]) => invoke<Note[]>("reorder_notes", { workspaceId, ids }),

  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  setStartAtLogin: (enabled: boolean) => invoke<boolean>("set_start_at_login", { enabled }),
  openDataFolder: () => invoke<void>("open_data_folder"),
  sendTestNotification: () => invoke<void>("send_test_notification"),
  /** Shows a save dialog; resolves to the saved path, or null if cancelled. */
  exportData: () => invoke<string | null>("export_data"),
  /** Shows an open dialog and replaces all data; null if cancelled. */
  importData: () => invoke<ImportSummary | null>("import_data"),
  deleteAllData: () => invoke<void>("delete_all_data"),

  /** null when already up to date. */
  checkForUpdates: () => invoke<UpdateInfo | null>("check_for_updates"),
  /** Downloads, installs and restarts the app. */
  installUpdate: () => invoke<void>("install_update"),

  getSetting: (key: SettingKey) => invoke<string | null>("get_setting", { key }),
  setSetting: (key: SettingKey, value: string) => invoke<void>("set_setting", { key, value }),
};
