<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api, errorMessage, type Alert } from "./lib/api";
  import { displayTime, hhmm, parseSnooze, ymdOf, ymd } from "./lib/dates";

  let alerts = $state<Alert[]>([]);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let defaultSnooze = $state(10);

  // Which alert has its snooze options open, and the custom text.
  let snoozing = $state<string | null>(null);
  let custom = $state("");
  let customError = $state<string | null>(null);

  const quick = $derived([...new Set([defaultSnooze, 5, 15, 30, 60])].sort((a, b) => a - b));

  async function load() {
    try {
      alerts = await api.getAlerts();
      error = null;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      loaded = true;
    }
  }

  onMount(() => {
    load();
    api
      .getSetting("default_snooze_minutes")
      .then((v) => {
        const n = Number(v);
        if (v && n > 0) defaultSnooze = n;
      })
      .catch(() => {});
    const un = listen("alerts-changed", load);
    return () => {
      un.then((f) => f());
    };
  });

  async function run(fn: () => Promise<void>) {
    try {
      await fn();
      snoozing = null;
      await load();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  const done = (a: Alert) => run(() => api.alertDone(a.taskId));
  const dismiss = (a: Alert) => run(() => api.alertDismiss(a.reminderId));
  const snoozeFor = (a: Alert, minutes: number) =>
    run(() => api.alertSnooze(a.reminderId, Date.now() + minutes * 60_000));

  function snoozeCustom(a: Alert) {
    const until = parseSnooze(custom);
    if (until === null) {
      customError = "Try 45, 1h30 or 16:30";
      return;
    }
    run(() => api.alertSnooze(a.reminderId, until));
  }

  function openSnooze(a: Alert) {
    snoozing = snoozing === a.reminderId ? null : a.reminderId;
    custom = "";
    customError = null;
  }

  function untilLabel(ms: number): string {
    const sameDay = ymdOf(ms) === ymd(new Date());
    return `until ${displayTime(hhmm(ms))}${sameDay ? "" : " tomorrow"}`;
  }

  const customPreview = $derived.by(() => {
    const u = parseSnooze(custom);
    return u === null ? null : untilLabel(u);
  });

  function label(a: Alert): string {
    if (a.missed) return `Missed · ${displayTime(a.time)}`;
    if (a.fromSnooze) return "Snoozed reminder";
    return displayTime(a.time);
  }
</script>

<main>
  <header>
    <span class="bell" aria-hidden="true">🔔</span>
    <h1>{alerts.length === 1 ? "Reminder" : `${alerts.length} reminders`}</h1>
  </header>

  {#if error}<p class="error-text pad">{error}</p>{/if}

  {#if loaded && alerts.length === 0}
    <p class="empty">All caught up.</p>
  {/if}

  <ul>
    {#each alerts as a (a.reminderId)}
      <li class="card" class:missed={a.missed}>
        <div class="meta">
          <span>{label(a)}</span>
          <span class="dot">·</span>
          <span>{a.workspaceName}</span>
        </div>
        <div class="title">{a.taskTitle}</div>

        <div class="actions">
          <button type="button" class="btn primary small" onclick={() => done(a)}>✓ Done</button>
          <button
            type="button"
            class="btn small"
            class:on={snoozing === a.reminderId}
            aria-expanded={snoozing === a.reminderId}
            onclick={() => openSnooze(a)}>Snooze</button
          >
          <button type="button" class="btn small ghost" onclick={() => dismiss(a)}>Dismiss</button>
        </div>

        {#if snoozing === a.reminderId}
          <div class="snooze">
            <div class="quick">
              {#each quick as m (m)}
                <button type="button" class="chip" class:def={m === defaultSnooze} onclick={() => snoozeFor(a, m)}>
                  {m < 60 ? `${m} min` : m % 60 === 0 ? `${m / 60} h` : `${Math.floor(m / 60)}h ${m % 60}m`}
                </button>
              {/each}
            </div>
            <form
              class="custom"
              onsubmit={(e) => {
                e.preventDefault();
                snoozeCustom(a);
              }}
            >
              <input
                class="input"
                placeholder="Custom: 45, 1h30, 16:30"
                aria-label="Custom snooze"
                bind:value={custom}
                oninput={() => (customError = null)}
              />
              <button type="submit" class="btn small">Snooze</button>
            </form>
            {#if customError}
              <div class="error-text">{customError}</div>
            {:else if customPreview}
              <div class="hint">{customPreview}</div>
            {/if}
          </div>
        {/if}
      </li>
    {/each}
  </ul>

  <footer>
    <button type="button" class="link" onclick={() => api.showMainWindow()}>Open Daily Tracker</button>
    {#if alerts.length > 1}
      <button type="button" class="link muted" onclick={() => run(() => api.dismissAllAlerts())}>Dismiss all</button>
    {/if}
  </footer>
</main>

<style>
  :global(body) {
    background: var(--bg);
  }
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 16px 8px;
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .bell {
    font-size: 16px;
  }
  ul {
    flex: 1;
    overflow-y: auto;
    list-style: none;
    margin: 0;
    padding: 4px 12px 12px;
  }
  .card {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
  }
  .card + .card {
    margin-top: 8px;
  }
  .card.missed {
    border-left: 3px solid #ba7517;
  }
  .meta {
    display: flex;
    gap: 4px;
    font-size: 12px;
    color: var(--text-2);
  }
  .missed .meta span:first-child {
    color: #ba7517;
    font-weight: 600;
  }
  .title {
    margin: 2px 0 10px;
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .btn.small {
    height: 30px;
    padding: 0 12px;
  }
  .btn.on {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--active);
  }
  .btn.ghost {
    border-color: transparent;
    color: var(--text-2);
    background: transparent;
  }
  .btn.ghost:hover {
    background: var(--hover);
  }
  .snooze {
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }
  .chip {
    height: 28px;
    padding: 0 10px;
    border-radius: 14px;
    border: 1px solid var(--border-strong);
    font-size: 13px;
  }
  .chip:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .chip.def {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--active);
  }
  .custom {
    display: flex;
    gap: 6px;
  }
  .custom .input {
    flex: 1;
    height: 30px;
  }
  .hint {
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-2);
  }
  .empty {
    padding: 24px 16px;
    color: var(--text-2);
    text-align: center;
  }
  .pad {
    padding: 0 16px;
  }
  footer {
    display: flex;
    justify-content: space-between;
    padding: 10px 16px 14px;
    border-top: 1px solid var(--border);
  }
  .link {
    color: var(--accent);
    font-weight: 500;
    font-size: 13px;
  }
  .link:hover {
    text-decoration: underline;
  }
  .link.muted {
    color: var(--text-2);
  }
</style>
