<script lang="ts">
  import type { TaskView } from "../api";
  import { displayTime, formatClock, formatDuration, hhmm, plural, shortDate } from "../dates";
  import { clock } from "../stores/clock.svelte";

  interface Props {
    task: TaskView;
    today: string;
    ontoggle: () => void;
    onedit: () => void;
    ondelete: () => void;
    ongo: (date: string) => void;
    onstart: () => void;
    onpause: () => void;
    onlog: () => void;
  }
  let { task: t, today, ontoggle, onedit, ondelete, ongo, onstart, onpause, onlog }: Props = $props();

  let menuOpen = $state(false);
  const done = $derived(t.status === "done");
  const ghost = $derived(t.display === "moved" || t.display === "completedLater");
  const running = $derived(t.runningSince !== null);
  const totalMs = $derived(t.trackedMs + (t.runningSince !== null ? Math.max(0, clock.now - t.runningSince) : 0));
  const canTrack = $derived(!done && !ghost);
  const showReminders = $derived(canTrack && t.reminders.length > 0);
</script>

<svelte:window onclick={() => (menuOpen = false)} />

<li class="task" class:done class:ghost>
  <button
    type="button"
    class="check"
    class:on={done}
    role="checkbox"
    aria-checked={done}
    aria-label={done ? `Mark "${t.title}" as not done` : `Mark "${t.title}" as done`}
    onclick={ontoggle}
  >
    {#if done}<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>{/if}
  </button>

  <div class="body">
    <button type="button" class="title" onclick={onedit}>{t.title}</button>
    {#if t.description}
      <p class="desc">{t.description}</p>
    {/if}

    {#if showReminders || running || totalMs > 0 || (t.display === "carried" && t.carryDays) || t.display === "moved" || (t.display === "completedLater" && t.completedOn) || (done && t.doneAfterDays)}
      <div class="meta">
        {#if running}
          <span class="badge running" aria-live="off">
            <span class="pulse" aria-hidden="true"></span>
            {formatClock(clock.now - (t.runningSince ?? clock.now))}
            {#if t.trackedMs > 0}<span class="sub">· {formatDuration(totalMs)} total</span>{/if}
          </span>
        {:else if totalMs > 0}
          <button type="button" class="badge time" onclick={onlog} title="Open time log">⏱ {formatDuration(totalMs)}</button>
        {/if}

        {#if showReminders}
          {#each t.reminders as r (r.id)}
            {#if r.snoozedUntil}
              <span class="badge rem snoozed" title="Snoozed">💤 {displayTime(hhmm(r.snoozedUntil))}</span>
            {:else}
              <span
                class="badge rem"
                class:fired={r.lastFiredOn === today}
                title={r.lastFiredOn === today ? "Already reminded today; repeats tomorrow if still open" : "Reminder"}
              >⏰ {displayTime(r.time)}</span>
            {/if}
          {/each}
        {/if}

        {#if t.display === "carried" && t.carryDays}
          <span class="badge carried" title="Not finished on {shortDate(t.originalDate, today)}, so it carried over automatically">
            From {shortDate(t.originalDate, today)} · carried {plural(t.carryDays, "day")}
          </span>
        {:else if t.display === "moved"}
          <button type="button" class="badge link" onclick={() => ongo(today)}>→ Moved to today</button>
        {:else if t.display === "completedLater" && t.completedOn}
          <button type="button" class="badge link" onclick={() => ongo(t.completedOn!)}>
            ✓ Completed {shortDate(t.completedOn, today)}
          </button>
        {:else if done && t.doneAfterDays}
          <span class="badge">From {shortDate(t.originalDate, today)} · done after {plural(t.doneAfterDays, "day")}</span>
        {/if}
      </div>
    {/if}
  </div>

  {#if canTrack}
    <button
      type="button"
      class="timer-btn"
      class:on={running}
      aria-label={running ? `Pause timer for "${t.title}"` : `Start timer for "${t.title}"`}
      title={running ? "Pause" : t.trackedMs > 0 ? "Resume timer" : "Start timer"}
      onclick={running ? onpause : onstart}
    >
      {#if running}
        <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="4" y="3" width="3" height="10" rx="1" /><rect x="9" y="3" width="3" height="10" rx="1" /></svg>
      {:else}
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5v9l7.5-4.5z" /></svg>
      {/if}
    </button>
  {/if}

  <div class="menu-wrap">
    <button
      type="button"
      class="more"
      class:open={menuOpen}
      aria-label="Options for {t.title}"
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      onclick={(e) => {
        e.stopPropagation();
        menuOpen = !menuOpen;
      }}>⋯</button
    >
    {#if menuOpen}
      <div class="menu" role="menu">
        <button type="button" role="menuitem" onclick={onedit}>Edit</button>
        <button type="button" role="menuitem" onclick={ontoggle}>{done ? "Mark not done" : "Mark done"}</button>
        <button type="button" role="menuitem" onclick={onlog}>Time log…</button>
        <div class="sep"></div>
        <button type="button" role="menuitem" class="danger" onclick={ondelete}>Delete</button>
      </div>
    {/if}
  </div>
</li>

<style>
  .task {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 10px 12px 14px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
  }
  .task + :global(.task) {
    margin-top: 8px;
  }
  .task.ghost {
    opacity: 0.6;
    background: transparent;
    border-style: dashed;
  }
  .check {
    flex: none;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-top: 1px;
    border-radius: 50%;
    border: 1.5px solid var(--border-strong);
    transition: background 0.12s, border-color 0.12s;
  }
  .check:hover {
    border-color: var(--accent);
    background: var(--active);
  }
  .check.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .check svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: var(--accent-text);
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  .title {
    display: block;
    max-width: 100%;
    text-align: left;
    font-weight: 500;
    line-height: 22px;
    overflow-wrap: anywhere;
  }
  .title:hover {
    color: var(--accent);
  }
  .done .title {
    color: var(--text-3);
    text-decoration: line-through;
  }
  .desc {
    margin: 2px 0 0;
    color: var(--text-2);
    font-size: 13px;
    white-space: pre-line;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .done .desc {
    color: var(--text-3);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    font-size: 12px;
    color: var(--text-2);
    background: var(--hover);
  }
  .badge.carried {
    color: #854f0b;
    background: #faeeda;
  }
  @media (prefers-color-scheme: dark) {
    .badge.carried {
      color: #fac775;
      background: #412402;
    }
  }
  .badge.link:hover {
    color: var(--accent);
    background: var(--active);
  }
  .badge.rem {
    color: var(--accent);
    background: var(--active);
  }
  .badge.rem.fired {
    color: var(--text-3);
    background: var(--hover);
    text-decoration: line-through;
  }
  .badge.rem.snoozed {
    color: var(--text-2);
    background: var(--hover);
  }
  .badge .sub {
    margin-left: 4px;
    opacity: 0.8;
  }
  .badge.time:hover {
    color: var(--accent);
    background: var(--active);
  }
  .badge.running {
    gap: 6px;
    color: var(--accent);
    background: var(--active);
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .pulse {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.4s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse {
      animation: none;
    }
  }
  .timer-btn {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    color: var(--text-2);
    opacity: 0;
  }
  .task:hover .timer-btn,
  .timer-btn:focus-visible,
  .timer-btn.on {
    opacity: 1;
  }
  .timer-btn:hover {
    background: var(--hover);
    color: var(--accent);
  }
  .timer-btn.on {
    background: var(--accent);
    color: var(--accent-text);
  }
  .timer-btn svg {
    width: 13px;
    height: 13px;
    fill: currentColor;
  }
  .menu-wrap {
    position: relative;
  }
  .more {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--text-2);
    opacity: 0;
  }
  .task:hover .more,
  .more.open,
  .more:focus-visible {
    opacity: 1;
  }
  .more:hover {
    background: var(--hover);
  }
  .menu {
    position: absolute;
    top: 30px;
    right: 0;
    z-index: 20;
    min-width: 160px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .menu button {
    display: block;
    width: 100%;
    padding: 6px 10px;
    border-radius: 5px;
    text-align: left;
  }
  .menu button:hover {
    background: var(--hover);
  }
  .menu .danger {
    color: var(--danger);
  }
  .sep {
    height: 1px;
    margin: 4px 2px;
    background: var(--border);
  }
</style>
