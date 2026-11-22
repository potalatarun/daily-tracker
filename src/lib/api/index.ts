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

export type SettingKey = "last_workspace_id" | "remember_panel_open";

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

  getSetting: (key: SettingKey) => invoke<string | null>("get_setting", { key }),
  setSetting: (key: SettingKey, value: string) => invoke<void>("set_setting", { key, value }),
};
