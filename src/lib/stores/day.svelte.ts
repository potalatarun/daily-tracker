import { api, errorMessage, type MonthSummary, type TaskView } from "../api";
import { addDays, parseYmd, shortDate, todayYmd, ymd } from "../dates";
import { timer } from "./timer.svelte";
import { toast } from "./toast.svelte";
import { workspaces } from "./workspaces.svelte";

/** State of the day being viewed and the mini calendar. */
class DayStore {
  date = $state(todayYmd());
  today = $state(todayYmd());
  tasks = $state<TaskView[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);

  /** Month shown in the mini calendar (0-based month). */
  calYear = $state(new Date().getFullYear());
  calMonth = $state(new Date().getMonth());
  month = $state<MonthSummary>({ days: [], carriedToday: 0 });

  isToday = $derived(this.date === this.today);

  private loadSeq = 0;

  /** Take "today" from the Rust core so UI and core can never disagree. */
  async init() {
    try {
      const t = await api.getToday();
      this.today = t;
      this.setDate(t);
    } catch {
      /* fall back to the browser clock already set */
    }
  }

  async load() {
    const ws = workspaces.selectedId;
    if (!ws) {
      this.tasks = [];
      return;
    }
    const seq = ++this.loadSeq;
    this.loading = true;
    try {
      const view = await api.getDay(ws, this.date);
      if (seq !== this.loadSeq) return; // a newer load superseded this one
      this.tasks = view.tasks;
      this.today = view.today;
      this.error = null;
    } catch (e) {
      if (seq === this.loadSeq) this.error = errorMessage(e);
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
    this.loadMonth();
    // Completing/deleting a task can stop the running timer; keep the sidebar in sync.
    timer.load();
  }

  async loadMonth() {
    const ws = workspaces.selectedId;
    if (!ws) return;
    const from = ymd(new Date(this.calYear, this.calMonth, 1));
    const to = ymd(new Date(this.calYear, this.calMonth + 1, 0));
    try {
      this.month = await api.getMonthSummary(ws, from, to);
    } catch {
      /* calendar dots are non-essential */
    }
  }

  setDate(date: string) {
    this.date = date;
    const d = parseYmd(date);
    if (d.getFullYear() !== this.calYear || d.getMonth() !== this.calMonth) {
      this.calYear = d.getFullYear();
      this.calMonth = d.getMonth();
    }
    this.load();
  }

  shift(days: number) {
    this.setDate(addDays(this.date, days));
  }

  goToday() {
    this.setDate(this.today);
  }

  shiftMonth(delta: number) {
    const d = new Date(this.calYear, this.calMonth + delta, 1);
    this.calYear = d.getFullYear();
    this.calMonth = d.getMonth();
    this.loadMonth();
  }

  /**
   * Called periodically: if the local date changed (midnight passed, or the
   * laptop woke up on a new day), move "Today" forward. Nothing is rewritten in
   * the database; carry-over is just a different query.
   */
  async checkRollover() {
    let now: string;
    try {
      now = await api.getToday();
    } catch {
      return;
    }
    if (now === this.today) return;
    const wasOnToday = this.date === this.today;
    this.today = now;
    if (wasOnToday) this.setDate(now);
    else this.load();
  }

  // ── mutations ────────────────────────────────────────────

  async create(date: string, title: string, description: string) {
    const ws = workspaces.selectedId;
    if (!ws) return;
    await api.createTask(ws, date, title, description);
    if (date !== this.date && !(this.isToday && date < this.today)) {
      toast.show(`Task added to ${date === this.today ? "Today" : shortDate(date, this.today)}`, {
        action: { label: "Go to day", run: () => this.setDate(date) },
      });
    }
    await this.load();
  }

  async update(id: string, changes: { title?: string; description?: string; date?: string }) {
    await api.updateTask(id, changes);
    await this.load();
  }

  async toggle(t: TaskView) {
    try {
      if (t.status === "done") await api.reopenTask(t.id);
      else await api.completeTask(t.id);
    } catch (e) {
      toast.show(errorMessage(e));
    }
    await this.load();
  }

  async remove(t: TaskView) {
    await api.deleteTask(t.id);
    await this.load();
    toast.show(`Deleted "${t.title}"`, {
      action: {
        label: "Undo",
        run: async () => {
          await api.restoreTask(t.id);
          await this.load();
        },
      },
    });
  }
}

export const day = new DayStore();
