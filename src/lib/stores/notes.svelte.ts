import { api, errorMessage, type Note } from "../api";
import { toast } from "./toast.svelte";
import { workspaces } from "./workspaces.svelte";

/** Rows of the Remember panel for the selected workspace. */
class NotesStore {
  items = $state<Note[]>([]);
  error = $state<string | null>(null);
  private loadedFor: string | null = null;

  async load() {
    const ws = workspaces.selectedId;
    this.loadedFor = ws;
    if (!ws) {
      this.items = [];
      return;
    }
    try {
      const rows = await api.listNotes(ws);
      if (this.loadedFor === ws) this.items = rows; // ignore stale responses
      this.error = null;
    } catch (e) {
      this.error = errorMessage(e);
    }
  }

  /** Throws with a readable message on validation errors. */
  async create(label: string, value: string, note: string) {
    const ws = workspaces.selectedId;
    if (!ws) return;
    const n = await api.createNote(ws, label, value, note);
    this.items = [...this.items, n];
    return n;
  }

  async update(id: string, changes: { label?: string; value?: string; note?: string }) {
    const n = await api.updateNote(id, changes);
    this.items = this.items.map((x) => (x.id === id ? n : x));
  }

  async remove(n: Note) {
    await api.deleteNote(n.id);
    this.items = this.items.filter((x) => x.id !== n.id);
    toast.show(`Deleted "${n.label || n.value}"`, {
      action: {
        label: "Undo",
        run: async () => {
          await api.restoreNote(n.id);
          await this.load();
        },
      },
    });
  }

  async move(id: string, delta: number) {
    const ws = workspaces.selectedId;
    const ids = this.items.map((n) => n.id);
    const from = ids.indexOf(id);
    const to = from + delta;
    if (!ws || from < 0 || to < 0 || to >= ids.length) return;
    ids.splice(from, 1);
    ids.splice(to, 0, id);
    try {
      this.items = await api.reorderNotes(ws, ids);
    } catch (e) {
      toast.show(errorMessage(e));
      await this.load();
    }
  }
}

export const notes = new NotesStore();
