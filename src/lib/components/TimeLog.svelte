<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorMessage, type Session, type TaskView } from "../api";
  import { addDays, formatDuration, hhmm, localInstant, parseTime, shortDate, ymdOf } from "../dates";
  import { clock } from "../stores/clock.svelte";
  import DateButton from "./DateButton.svelte";

  interface Props {
    task: TaskView;
    today: string;
    /** Called after any change so the day view and sidebar refresh. */
    onchanged: () => void;
    onclose: () => void;
  }
  let { task, today, onchanged, onclose }: Props = $props();

  let sessions = $state<Session[]>([]);
  let loadError = $state<string | null>(null);

  // Row being edited (or "new" for the add form).
  let editId = $state<string | null>(null);
  let fDate = $state("");
  let fStart = $state("");
  let fEnd = $state("");
  let fError = $state<string | null>(null);
  let busy = $state(false);

  const total = $derived(
    sessions.reduce((sum, s) => sum + ((s.endedAt ?? clock.now) - s.startedAt), 0),
  );

  async function load() {
    try {
      sessions = (await api.listSessions(task.id)).reverse(); // newest first
      loadError = null;
    } catch (e) {
      loadError = errorMessage(e);
    }
  }
  onMount(load);

  function startEdit(s: Session) {
    editId = s.id;
    fDate = ymdOf(s.startedAt);
    fStart = hhmm(s.startedAt);
    fEnd = s.endedAt ? hhmm(s.endedAt) : "";
    fError = null;
  }

  function startAdd() {
    editId = "new";
    fDate = today;
    fStart = "";
    fEnd = "";
    fError = null;
  }

  /** End is on the start's day, or the next day if the end time is earlier. */
  function instants(): { start: number; end: number | null; overnight: boolean } | string {
    const s = parseTime(fStart);
    if (s === null) return "Enter a start time like 09:30";
    const running = editId !== "new" && sessions.find((x) => x.id === editId)?.endedAt === null;
    if (running && fEnd.trim() === "") return { start: localInstant(fDate, s), end: null, overnight: false };
    const e = parseTime(fEnd);
    if (e === null) return "Enter an end time like 11:00";
    const overnight = e <= s;
    return {
      start: localInstant(fDate, s),
      end: localInstant(overnight ? addDays(fDate, 1) : fDate, e),
      overnight,
    };
  }

  const preview = $derived(instants());

  async function save() {
    const v = instants();
    if (typeof v === "string") {
      fError = v;
      return;
    }
    busy = true;
    try {
      if (editId === "new") await api.addSession(task.id, v.start, v.end!);
      else if (editId) await api.updateSession(editId, v.start, v.end);
      editId = null;
      await load();
      onchanged();
    } catch (e) {
      fError = errorMessage(e);
    } finally {
      busy = false;
    }
  }

  async function remove(s: Session) {
    try {
      await api.deleteSession(s.id);
      await load();
      onchanged();
    } catch (e) {
      loadError = errorMessage(e);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") {
      if (editId) editId = null;
      else onclose();
    }
    if (e.key === "Enter" && editId && (e.target as HTMLElement).tagName === "INPUT") save();
  }
</script>

{#snippet editor()}
  <div class="editor">
    <DateButton value={fDate} {today} format="short" max={today} onchange={(d) => (fDate = d)} />
    <input class="input time" placeholder="09:30" bind:value={fStart} aria-label="Start time" oninput={() => (fError = null)} />
    <span class="dash">–</span>
    <input
      class="input time"
      placeholder={editId !== "new" && sessions.find((x) => x.id === editId)?.endedAt === null ? "running" : "11:00"}
      bind:value={fEnd}
      aria-label="End time"
      oninput={() => (fError = null)}
    />
    <span class="dur">
      {#if typeof preview !== "string"}
        {preview.end === null ? "running" : formatDuration(preview.end - preview.start)}{preview.overnight ? " (ends next day)" : ""}
      {/if}
    </span>
    <button type="button" class="btn small" onclick={() => (editId = null)}>Cancel</button>
    <button type="button" class="btn primary small" disabled={busy} onclick={save}>Save</button>
  </div>
  {#if fError}<div class="error-text">{fError}</div>{/if}
{/snippet}

<div class="backdrop" role="presentation" onclick={onclose}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="timelog-title"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    {onkeydown}
  >
    <header>
      <div>
        <h2 id="timelog-title">Time log</h2>
        <p class="task-title">{task.title}</p>
      </div>
      <div class="total">
        <span class="total-num">{formatDuration(total)}</span>
        <span class="total-label">total</span>
      </div>
    </header>

    {#if loadError}<p class="error-text">{loadError}</p>{/if}

    {#if sessions.length === 0}
      <p class="empty">No time tracked yet. Press ▶ on the task to start, or add time by hand below.</p>
    {:else}
      <ul class="list">
        {#each sessions as s (s.id)}
          <li>
            {#if editId === s.id}
              {@render editor()}
            {:else}
              <span class="when">{shortDate(ymdOf(s.startedAt), today)}</span>
              <span class="range">
                {hhmm(s.startedAt)} – {s.endedAt ? hhmm(s.endedAt) : "now"}
                {#if s.endedAt && ymdOf(s.endedAt) !== ymdOf(s.startedAt)}<span class="next">+1 day</span>{/if}
              </span>
              <span class="dur" class:live={!s.endedAt}>{formatDuration((s.endedAt ?? clock.now) - s.startedAt)}</span>
              <span class="row-actions">
                <button type="button" onclick={() => startEdit(s)}>Edit</button>
                <button type="button" class="danger" onclick={() => remove(s)} aria-label="Delete entry">Delete</button>
              </span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    {#if editId === "new"}
      <div class="add-row">{@render editor()}</div>
    {:else}
      <button type="button" class="add" onclick={startAdd}>+ Add time manually</button>
    {/if}

    <div class="foot">
      <button type="button" class="btn" onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: start center;
    padding-top: 12vh;
    background: rgba(0, 0, 0, 0.35);
  }
  .dialog {
    width: min(600px, calc(100vw - 48px));
    max-height: 76vh;
    overflow-y: auto;
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 14px;
  }
  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  .task-title {
    margin: 2px 0 0;
    color: var(--text-2);
  }
  .total {
    text-align: right;
  }
  .total-num {
    display: block;
    font-size: 20px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  .total-label {
    font-size: 12px;
    color: var(--text-3);
  }
  .empty {
    color: var(--text-2);
  }
  .list {
    list-style: none;
    margin: 0 0 10px;
    padding: 0;
    border-top: 1px solid var(--border);
  }
  .list li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
  }
  .when {
    width: 72px;
    color: var(--text-2);
  }
  .range {
    flex: 1;
    font-variant-numeric: tabular-nums;
  }
  .next {
    margin-left: 6px;
    font-size: 12px;
    color: var(--text-3);
  }
  .dur {
    font-variant-numeric: tabular-nums;
    color: var(--text-2);
  }
  .dur.live {
    color: var(--accent);
    font-weight: 600;
  }
  .row-actions {
    display: flex;
    gap: 2px;
  }
  .row-actions button {
    padding: 4px 8px;
    border-radius: 5px;
    color: var(--text-2);
    font-size: 13px;
  }
  .row-actions button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .row-actions .danger:hover {
    color: var(--danger);
  }
  .editor {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    width: 100%;
  }
  .editor .time {
    width: 74px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .editor .dur {
    flex: 1;
    min-width: 90px;
    font-size: 13px;
  }
  .dash {
    color: var(--text-3);
  }
  .add-row {
    padding: 10px 0;
  }
  .add {
    padding: 8px 0;
    color: var(--accent);
    font-weight: 500;
  }
  .add:hover {
    text-decoration: underline;
  }
  .btn.small {
    height: 30px;
    padding: 0 10px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    margin-top: 12px;
  }
</style>
