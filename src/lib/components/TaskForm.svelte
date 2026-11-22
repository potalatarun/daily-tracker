<script lang="ts" module>
  export interface TaskFormValues {
    date: string;
    title: string;
    description: string;
    /** Reminder times, "HH:MM". */
    reminders: string[];
  }
</script>

<script lang="ts">
  import { tick, untrack } from "svelte";
  import { errorMessage } from "../api";
  import { addDays, displayTime, normalizeTime } from "../dates";
  import DateButton from "./DateButton.svelte";

  interface Props {
    heading: string;
    submitLabel: string;
    initial: TaskFormValues;
    today: string;
    onsave: (v: TaskFormValues) => Promise<void>;
    onclose: () => void;
  }
  let { heading, submitLabel, initial, today, onsave, onclose }: Props = $props();

  // The form edits a copy; later changes to `initial` are intentionally ignored.
  const init = untrack(() => ({ ...initial }));
  let date = $state(init.date);
  let title = $state(init.title);
  let description = $state(init.description);
  let reminders = $state<string[]>([...init.reminders].sort());
  let reminderInput = $state("");
  let reminderError = $state<string | null>(null);

  /** Adds the typed time. Returns false if the input is invalid. */
  function addReminder(): boolean {
    if (!reminderInput.trim()) return true;
    const t = normalizeTime(reminderInput);
    if (!t) {
      reminderError = "Use a time like 15:00, 9:30 or 3pm";
      return false;
    }
    if (!reminders.includes(t)) reminders = [...reminders, t].sort();
    if (reminders.length > 10) {
      reminders = reminders.slice(0, 10);
      reminderError = "At most 10 reminders per task";
      return false;
    }
    reminderInput = "";
    reminderError = null;
    return true;
  }

  function onReminderKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !(e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      addReminder();
    }
  }
  let error = $state<string | null>(null);
  let saving = $state(false);


  let titleInput: HTMLInputElement | undefined = $state();
  $effect(() => {
    tick().then(() => titleInput?.focus());
  });


  async function submit(e?: Event) {
    e?.preventDefault();
    if (!title.trim()) {
      error = "Give the task a title";
      titleInput?.focus();
      return;
    }
    saving = true;
    try {
      // A time typed but not yet added still counts.
      if (!addReminder()) {
        saving = false;
        return;
      }
      await onsave({ date, title, description, reminders });
    } catch (err) {
      error = errorMessage(err);
      saving = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) submit();
  }
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="task-form-title"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => {
      e.stopPropagation();
      onkeydown(e);
    }}
  >
  <form class="form" onsubmit={submit}>
    <h2 id="task-form-title">{heading}</h2>

    <div class="field">
      <span class="label" id="date-label">Date</span>
      <div class="date-row">
        <DateButton value={date} {today} labelledby="date-label" onchange={(d) => {
          date = d;
          titleInput?.focus();
        }} />
        <button type="button" class="chip" class:on={date === today} onclick={() => (date = today)}>Today</button>
        <button type="button" class="chip" class:on={date === addDays(today, 1)} onclick={() => (date = addDays(today, 1))}>Tomorrow</button>
      </div>
    </div>

    <label class="field">
      <span class="label">Title</span>
      <input
        class="input"
        bind:this={titleInput}
        bind:value={title}
        maxlength="200"
        placeholder="What needs to be done?"
        oninput={() => (error = null)}
      />
    </label>

    <label class="field">
      <span class="label">Description <span class="opt">optional</span></span>
      <textarea class="input area" bind:value={description} maxlength="5000" rows="4" placeholder="Details, links, notes…"></textarea>
    </label>

    <div class="field">
      <span class="label" id="rem-label">Reminders <span class="opt">optional · repeat daily while the task is open</span></span>
      <div class="rem-row">
        {#each reminders as r (r)}
          <span class="rem-chip">
            ⏰ {displayTime(r)}
            <button type="button" aria-label="Remove reminder at {displayTime(r)}" onclick={() => (reminders = reminders.filter((x) => x !== r))}>×</button>
          </span>
        {/each}
        <input
          class="input rem-input"
          placeholder={reminders.length ? "Add another" : "e.g. 15:00 or 3pm"}
          aria-labelledby="rem-label"
          bind:value={reminderInput}
          onkeydown={onReminderKey}
          oninput={() => (reminderError = null)}
          onblur={() => addReminder()}
        />
        <button type="button" class="chip" onclick={() => addReminder()}>Add</button>
      </div>
      {#if reminderError}<div class="error-text">{reminderError}</div>{/if}
    </div>

    {#if error}<div class="error-text">{error}</div>{/if}

    <div class="actions">
      <span class="hint">{navigator.platform.includes("Mac") ? "⌘" : "Ctrl"} + Enter to save</span>
      <button type="button" class="btn" onclick={onclose}>Cancel</button>
      <button type="submit" class="btn primary" disabled={saving}>{submitLabel}</button>
    </div>
  </form>
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
    width: min(520px, calc(100vw - 48px));
    padding: 20px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .form {
    display: grid;
    gap: 14px;
  }
  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  .field {
    display: grid;
    gap: 6px;
  }
  .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }
  .opt {
    font-weight: 400;
    color: var(--text-3);
  }
  .date-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .chip {
    height: 32px;
    padding: 0 10px;
    border-radius: 6px;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
  }
  .chip:hover {
    background: var(--hover);
  }
  .chip.on {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--active);
  }
  .rem-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .rem-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 4px 0 10px;
    border-radius: 14px;
    background: var(--active);
    color: var(--accent);
    font-weight: 500;
    font-size: 13px;
  }
  .rem-chip button {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: inherit;
    font-size: 15px;
    line-height: 1;
  }
  .rem-chip button:hover {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }
  .rem-input {
    width: 150px;
    height: 28px;
  }
  .rem-row .chip {
    height: 28px;
  }
  .area {
    height: auto;
    padding: 8px 10px;
    resize: vertical;
    min-height: 80px;
    line-height: 1.45;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .hint {
    margin-right: auto;
    font-size: 12px;
    color: var(--text-3);
  }
</style>
