/** A shared clock that ticks every second, for live timers. */
class Clock {
  now = $state(Date.now());

  constructor() {
    setInterval(() => (this.now = Date.now()), 1000);
  }
}

export const clock = new Clock();
