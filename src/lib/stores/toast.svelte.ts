export interface ToastAction {
  label: string;
  run: () => void | Promise<void>;
}

/** One toast at a time, bottom of the window. */
class ToastStore {
  message = $state<string | null>(null);
  action = $state<ToastAction | null>(null);
  private timer: ReturnType<typeof setTimeout> | undefined;

  show(message: string, opts: { action?: ToastAction; ms?: number } = {}) {
    clearTimeout(this.timer);
    this.message = message;
    this.action = opts.action ?? null;
    this.timer = setTimeout(() => this.hide(), opts.ms ?? 6000);
  }

  hide() {
    clearTimeout(this.timer);
    this.message = null;
    this.action = null;
  }

  async runAction() {
    const a = this.action;
    this.hide();
    await a?.run();
  }
}

export const toast = new ToastStore();
