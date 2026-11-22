import { api, errorMessage, type Workspace } from "../api";

/** App-wide workspace state. The Rust core is the source of truth; this mirrors it. */
class WorkspaceStore {
  items = $state<Workspace[]>([]);
  selectedId = $state<string | null>(null);
  loaded = $state(false);
  loadError = $state<string | null>(null);

  selected = $derived(this.items.find((w) => w.id === this.selectedId) ?? null);

  async load() {
    try {
      this.items = await api.listWorkspaces();
      const last = await api.getSetting("last_workspace_id");
      const keep = this.items.find((w) => w.id === last) ?? this.items[0];
      this.selectedId = keep?.id ?? null;
      this.loadError = null;
    } catch (e) {
      this.loadError = errorMessage(e);
    } finally {
      this.loaded = true;
    }
  }

  async select(id: string) {
    this.selectedId = id;
    await api.setSetting("last_workspace_id", id).catch(() => {});
  }

  /** Throws with a user-readable message on validation errors. */
  async create(name: string, color: string) {
    const w = await api.createWorkspace(name, color);
    this.items = [...this.items, w];
    if (!this.selectedId) await this.select(w.id);
    return w;
  }

  async update(id: string, changes: { name?: string; color?: string }) {
    const w = await api.updateWorkspace(id, changes);
    this.items = this.items.map((x) => (x.id === id ? w : x));
    return w;
  }

  async moveTo(id: string, toIndex: number) {
    const ids = this.items.map((w) => w.id);
    const from = ids.indexOf(id);
    if (from < 0 || toIndex < 0 || toIndex >= ids.length || from === toIndex) return;
    ids.splice(from, 1);
    ids.splice(toIndex, 0, id);
    // Optimistic update so dragging feels instant; reconciled with the server result.
    const byId = new Map(this.items.map((w) => [w.id, w]));
    this.items = ids.map((x) => byId.get(x)!);
    try {
      this.items = await api.reorderWorkspaces(ids);
    } catch {
      await this.load();
    }
  }

  async remove(id: string) {
    await api.deleteWorkspace(id);
    const idx = this.items.findIndex((w) => w.id === id);
    this.items = this.items.filter((w) => w.id !== id);
    if (this.selectedId === id) {
      const next = this.items[Math.min(idx, this.items.length - 1)];
      this.selectedId = next?.id ?? null;
      if (next) await api.setSetting("last_workspace_id", next.id).catch(() => {});
    }
  }
}

export const workspaces = new WorkspaceStore();
