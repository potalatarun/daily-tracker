import { api, errorMessage, type ActiveTimer } from "../api";
import { day } from "./day.svelte";
import { toast } from "./toast.svelte";

/** The single running timer (if any), shown in the sidebar from any workspace. */
class TimerStore {
  active = $state<ActiveTimer | null>(null);

  async load() {
    try {
      this.active = await api.getActiveTimer();
    } catch {
      this.active = null;
    }
  }

  async start(taskId: string) {
    try {
      const r = await api.startTimer(taskId);
      this.active = r.active;
      if (r.pausedTaskTitle) toast.show(`Paused "${r.pausedTaskTitle}"`, { ms: 4000 });
    } catch (e) {
      toast.show(errorMessage(e));
    }
    await day.load();
  }

  async pause(taskId: string) {
    try {
      await api.pauseTimer(taskId);
    } catch (e) {
      toast.show(errorMessage(e));
    }
    await this.load();
    await day.load();
  }
}

export const timer = new TimerStore();
