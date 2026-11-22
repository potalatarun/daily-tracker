<script lang="ts">
  import { errorMessage, type TaskDisplay, type TaskView } from "../api";
  import { daysBetween, longDate, relativeDay } from "../dates";
  import { day } from "../stores/day.svelte";
  import { toast } from "../stores/toast.svelte";
  import { workspaces } from "../stores/workspaces.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import TaskForm, { type TaskFormValues } from "./TaskForm.svelte";
  import TaskItem from "./TaskItem.svelte";
  import TimeLog from "./TimeLog.svelte";
  import { timer } from "../stores/timer.svelte";

  let adding = $state(false);
  let editing = $state<TaskView | null>(null);
  let deleting = $state<TaskView | null>(null);
  let logging = $state<TaskView | null>(null);

  const GROUPS: { key: TaskDisplay; label: (isToday: boolean) => string }[] = [
    { key: "carried", label: () => "Carried over" },
    { key: "planned", label: (t) => (t ? "To do" : "Planned") },
    { key: "done", label: () => "Done" },
    { key: "moved", label: () => "Not finished, moved to today" },
    { key: "completedLater", label: () => "Finished on a later day" },
  ];

  const groups = $derived(
    GROUPS.map((g) => ({
      key: g.key,
      label: g.label(day.isToday),
      tasks: day.tasks.filter((t) => t.display === g.key),
    })).filter((g) => g.tasks.length > 0),
  );

  const pendingCount = $derived(day.tasks.filter((t) => t.display === "planned" || t.display === "carried").length);
  const isPast = $derived(daysBetween(day.today, day.date) < 0);
  const rel = $derived(relativeDay(day.date, day.today));
  const isMac = navigator.platform.includes("Mac");

  function typing(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    return el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName);
  }

  function onkeydown(e: KeyboardEvent) {
    if (adding || editing || deleting || logging) return;
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (mod && e.key.toLowerCase() === "n") {
      e.preventDefault();
      adding = true;
      return;
    }
    if (typing(e) || e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === "ArrowLeft") day.shift(-1);
    else if (e.key === "ArrowRight") day.shift(1);
    else if (e.key.toLowerCase() === "t") day.goToday();
  }

  async function saveNew(v: TaskFormValues) {
    await day.create(v.date, v.title, v.description, v.reminders);
    adding = false;
  }

  async function saveEdit(v: TaskFormValues) {
    if (!editing) return;
    const changes: { title?: string; description?: string; date?: string } = {};
    if (v.title !== editing.title) changes.title = v.title;
    if (v.description !== editing.description) changes.description = v.description;
    if (v.date !== editing.originalDate) changes.date = v.date;
    const before = editing.reminders.map((r) => r.time).sort().join(",");
    const remindersChanged = before !== [...v.reminders].sort().join(",");
    if (Object.keys(changes).length || remindersChanged) {
      await day.update(editing.id, changes, remindersChanged ? v.reminders : undefined);
    }
    editing = null;
  }
</script>

<svelte:window {onkeydown} />

<header class="head">
  <div class="title-block">
    {#if workspaces.selected}
      <div class="ws-label">
        <span class="dot" style:background={workspaces.selected.color}></span>
        {workspaces.selected.name}
      </div>
    {/if}
    <h1>
      {#if ["Today", "Yesterday", "Tomorrow"].includes(rel)}
        {rel} <span class="date">· {longDate(day.date)}</span>
      {:else}
        {longDate(day.date)}
      {/if}
    </h1>
  </div>

  <div class="controls">
    <div class="nav" role="group" aria-label="Change day">
      <button type="button" onclick={() => day.shift(-1)} aria-label="Previous day" title="Previous day (←)">‹</button>
      <button type="button" class="today-btn" disabled={day.isToday} onclick={() => day.goToday()} title="Go to today (T)">Today</button>
      <button type="button" onclick={() => day.shift(1)} aria-label="Next day" title="Next day (→)">›</button>
    </div>
    <button type="button" class="btn primary" onclick={() => (adding = true)} title="New task ({isMac ? '⌘' : 'Ctrl'}+N)">
      <span class="plus">+</span> Add task
    </button>
  </div>
</header>

{#if day.isToday && day.tasks.length > 0}
  <p class="summary">
    {#if pendingCount === 0}
      All done for today.
    {:else}
      {pendingCount} to do{day.tasks.some((t) => t.display === "carried") ? ", including carried-over tasks" : ""}.
    {/if}
  </p>
{/if}

{#if day.error}
  <p class="error-text">{day.error}</p>
{:else if day.tasks.length === 0 && !day.loading}
  <div class="empty">
    {#if isPast}
      <p>Nothing was planned for this day.</p>
    {:else}
      <p>{day.isToday ? "Nothing on your list today." : "Nothing planned yet."}</p>
      <button type="button" class="btn" onclick={() => (adding = true)}>+ Add a task</button>
    {/if}
  </div>
{:else}
  {#each groups as g (g.key)}
    <section class="group">
      <h2>{g.label} <span class="count">{g.tasks.length}</span></h2>
      <ul>
        {#each g.tasks as t (t.id)}
          <TaskItem
            task={t}
            today={day.today}
            ontoggle={() => day.toggle(t)}
            onedit={() => (editing = t)}
            ondelete={() => (deleting = t)}
            ongo={(d) => day.setDate(d)}
            onstart={() => timer.start(t.id)}
            onpause={() => timer.pause(t.id)}
            onlog={() => (logging = t)}
          />
        {/each}
      </ul>
    </section>
  {/each}
{/if}

{#if adding}
  <TaskForm
    heading="New task"
    submitLabel="Add task"
    today={day.today}
    initial={{ date: day.date < day.today ? day.today : day.date, title: "", description: "", reminders: [] }}
    onsave={saveNew}
    onclose={() => (adding = false)}
  />
{/if}

{#if editing}
  <TaskForm
    heading="Edit task"
    submitLabel="Save"
    today={day.today}
    initial={{
      date: editing.originalDate,
      title: editing.title,
      description: editing.description,
      reminders: editing.reminders.map((r) => r.time),
    }}
    onsave={saveEdit}
    onclose={() => (editing = null)}
  />
{/if}

{#if logging}
  <TimeLog task={logging} today={day.today} onchanged={() => day.load()} onclose={() => (logging = null)} />
{/if}

{#if deleting}
  {@const t = deleting}
  <ConfirmDialog
    title="Delete this task?"
    message={`"${t.title}" will be removed. You can undo right after.`}
    confirmLabel="Delete"
    danger
    oncancel={() => (deleting = null)}
    onconfirm={async () => {
      const target = t; // read before clearing: `t` follows `deleting`
      deleting = null;
      try {
        await day.remove(target);
      } catch (e) {
        toast.show(errorMessage(e));
      }
    }}
  />
{/if}

<style>
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 8px;
  }
  .ws-label {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-bottom: 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .date {
    font-weight: 400;
    color: var(--text-2);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }
  .nav {
    display: flex;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--surface);
  }
  .nav button {
    height: 30px;
    min-width: 30px;
    padding: 0 8px;
    font-size: 16px;
  }
  .nav button:hover:not(:disabled) {
    background: var(--hover);
  }
  .nav .today-btn {
    font-size: 13px;
    font-weight: 500;
    border-left: 1px solid var(--border);
    border-right: 1px solid var(--border);
  }
  .nav .today-btn:disabled {
    color: var(--text-3);
    cursor: default;
  }
  .plus {
    font-size: 16px;
    line-height: 1;
  }
  .summary {
    margin: 0 0 18px;
    color: var(--text-2);
  }
  .group {
    margin-top: 18px;
  }
  .group h2 {
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .count {
    margin-left: 4px;
    font-weight: 500;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .empty {
    margin-top: 24px;
    padding: 48px 0;
    text-align: center;
    border: 1px dashed var(--border-strong);
    border-radius: 12px;
    color: var(--text-2);
  }
  .empty p {
    margin: 0 0 12px;
  }
</style>
