<script lang="ts">
  import { tick } from "svelte";
  import { api, errorMessage } from "../api";
  import { WORKSPACE_COLORS } from "../colors";
  import { workspaces } from "../stores/workspaces.svelte";

  interface Props {
    ondone: () => void;
  }
  let { ondone }: Props = $props();

  let step = $state(1);
  let names = $state(["Work", "Personal"]);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let testSent = $state(false);
  let startAtLogin = $state(true);

  let list: HTMLUListElement | undefined = $state();

  async function addAnother() {
    names = [...names, ""];
    await tick();
    list?.querySelector<HTMLInputElement>("li:last-child input")?.focus();
  }

  const cleaned = $derived(names.map((n) => n.trim()).filter(Boolean));

  async function createWorkspaces() {
    const lower = cleaned.map((n) => n.toLowerCase());
    if (cleaned.length === 0) {
      error = "Add at least one workspace";
      return;
    }
    if (new Set(lower).size !== lower.length) {
      error = "Each workspace needs a different name";
      return;
    }
    busy = true;
    error = null;
    try {
      for (const [i, name] of cleaned.entries()) {
        await workspaces.create(name, WORKSPACE_COLORS[i % WORKSPACE_COLORS.length].hex);
      }
      step = 2;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }

  /** Moving from another computer: restore an export instead of starting fresh. */
  async function importBackup() {
    error = null;
    try {
      // On success the core emits "data-replaced" and the app reloads.
      await api.importData();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function sendTest() {
    try {
      await api.sendTestNotification();
      testSent = true;
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function finish() {
    busy = true;
    try {
      await api.setStartAtLogin(startAtLogin).catch(() => {});
      await api.setSetting("onboarded", "1");
      ondone();
    } finally {
      busy = false;
    }
  }
</script>

<div class="wrap">
  <div class="card">
    <div class="steps" aria-label="Step {step} of 3">
      {#each [1, 2, 3] as s (s)}
        <span class="pip" class:on={s <= step}></span>
      {/each}
    </div>

    {#if step === 1}
      <img src="/icon.svg" alt="" width="48" height="48" />
      <h1>Welcome to Daily Tracker</h1>
      <p class="lead">Plan your day by date, get reminders, and let unfinished work carry over by itself.</p>

      <h2>Your workspaces</h2>
      <p class="muted">Keep different parts of your life apart. You can rename or add more later.</p>
      <ul class="names" bind:this={list}>
        {#each names as _, i (i)}
          <li>
            <span class="dot" style:background={WORKSPACE_COLORS[i % WORKSPACE_COLORS.length].hex}></span>
            <input
              class="input"
              bind:value={names[i]}
              maxlength="40"
              aria-label="Workspace {i + 1} name"
              oninput={() => (error = null)}
              onkeydown={(e) => e.key === "Enter" && createWorkspaces()}
            />
            {#if names.length > 1}
              <button type="button" class="x" aria-label="Remove" onclick={() => (names = names.filter((_, j) => j !== i))}>×</button>
            {/if}
          </li>
        {/each}
      </ul>
      {#if names.length < 8}
        <button type="button" class="add" onclick={addAnother}>+ Add another</button>
      {/if}
      {#if error}<p class="error-text">{error}</p>{/if}
      <div class="actions spread">
        <button type="button" class="link" onclick={importBackup}>Moving from another computer? Import a backup</button>
        <button class="btn primary" disabled={busy} onclick={createWorkspaces}>Continue</button>
      </div>
    {:else if step === 2}
      <h1>Reminders</h1>
      <p class="lead">When a reminder is due you'll get a system notification, and a small reminder window appears in the corner with <strong>Done</strong>, <strong>Snooze</strong> and <strong>Dismiss</strong>.</p>
      <div class="test">
        <button class="btn" onclick={sendTest}>Send a test notification</button>
        {#if testSent}
          <p class="muted small">
            Didn't see it? Allow notifications for Daily Tracker in your system settings
            (macOS: System Settings → Notifications; Windows: Settings → System → Notifications).
            The reminder window works either way.
          </p>
        {/if}
      </div>
      {#if error}<p class="error-text">{error}</p>{/if}
      <div class="actions">
        <button class="btn primary" onclick={() => (step = 3)}>Continue</button>
      </div>
    {:else}
      <h1>Start at login</h1>
      <p class="lead">Reminders only work while Daily Tracker is running. Starting it when you log in keeps it quietly in the tray (menu bar on Mac), so you never miss one.</p>
      <label class="toggle-row">
        <input type="checkbox" bind:checked={startAtLogin} />
        <span>Start Daily Tracker when I log in <em>(recommended)</em></span>
      </label>
      <p class="muted small">Closing the window keeps the app running in the tray. Use “Quit” in the tray menu to stop it.</p>
      <div class="actions">
        <button class="btn primary" disabled={busy} onclick={finish}>Start using Daily Tracker</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .wrap {
    display: grid;
    place-items: center;
    min-height: 100%;
    padding: 32px;
  }
  .card {
    width: min(480px, 100%);
    padding: 28px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .steps {
    display: flex;
    gap: 6px;
    margin-bottom: 20px;
  }
  .pip {
    width: 28px;
    height: 4px;
    border-radius: 2px;
    background: var(--border);
  }
  .pip.on {
    background: var(--accent);
  }
  h1 {
    margin: 12px 0 6px;
    font-size: 22px;
    font-weight: 650;
  }
  h2 {
    margin: 20px 0 4px;
    font-size: 14px;
    font-weight: 600;
  }
  .lead {
    margin: 0;
    color: var(--text-2);
  }
  .muted {
    margin: 0 0 10px;
    color: var(--text-2);
  }
  .small {
    font-size: 12.5px;
  }
  .names {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .names li {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .x {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--text-2);
    font-size: 18px;
  }
  .x:hover {
    background: var(--hover);
  }
  .add {
    margin-top: 8px;
    color: var(--accent);
    font-weight: 500;
  }
  .test {
    margin-top: 18px;
    display: grid;
    gap: 10px;
    justify-items: start;
  }
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 18px 0 10px;
    font-weight: 500;
  }
  .toggle-row input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .toggle-row em {
    font-style: normal;
    font-weight: 400;
    color: var(--text-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 12px;
    margin-top: 22px;
  }
  .actions.spread {
    justify-content: space-between;
  }
  .link {
    color: var(--text-2);
    font-size: 12.5px;
    text-align: left;
  }
  .link:hover {
    color: var(--accent);
    text-decoration: underline;
  }
</style>
