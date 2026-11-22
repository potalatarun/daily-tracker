<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { WORKSPACE_COLORS } from "./lib/colors";
  import { workspaces } from "./lib/stores/workspaces.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";

  let rememberOpen = $state(true);
  let creatingDefaults = $state(false);

  const todayLabel = new Date().toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
  });

  onMount(async () => {
    await workspaces.load();
    const saved = await api.getSetting("remember_panel_open").catch(() => null);
    if (saved !== null) rememberOpen = saved === "true";
  });

  function toggleRemember() {
    rememberOpen = !rememberOpen;
    api.setSetting("remember_panel_open", String(rememberOpen)).catch(() => {});
  }

  async function createDefaults() {
    creatingDefaults = true;
    try {
      await workspaces.create("Work", WORKSPACE_COLORS[0].hex);
      await workspaces.create("Personal", WORKSPACE_COLORS[1].hex);
    } finally {
      creatingDefaults = false;
    }
  }
</script>

<div class="shell" class:remember-closed={!rememberOpen}>
  <Sidebar />

  <main class="day">
    {#if !workspaces.loaded}
      <div class="center muted">Loading…</div>
    {:else if workspaces.loadError}
      <div class="center">
        <p class="error-text">Couldn't open your data: {workspaces.loadError}</p>
      </div>
    {:else if !workspaces.selected}
      <div class="center welcome">
        <img src="/icon.svg" alt="" width="56" height="56" />
        <h1>Welcome to Daily Tracker</h1>
        <p class="muted">Workspaces keep different parts of your life apart. Start with two, or make your own in the sidebar.</p>
        <button class="btn primary" onclick={createDefaults} disabled={creatingDefaults}>
          Create “Work” and “Personal”
        </button>
      </div>
    {:else}
      <header class="day-header">
        <div>
          <div class="ws-label">
            <span class="dot" style:background={workspaces.selected.color}></span>
            {workspaces.selected.name}
          </div>
          <h1>Today <span class="date">· {todayLabel}</span></h1>
        </div>
        {#if !rememberOpen}
          <button class="btn" onclick={toggleRemember} aria-label="Show Remember panel">Remember</button>
        {/if}
      </header>
      <section class="empty">
        <p>No tasks yet.</p>
        <p class="muted small">Adding tasks, reminders and carry-over arrive in the next update.</p>
      </section>
    {/if}
  </main>

  {#if rememberOpen}
    <aside class="remember" aria-label="Remember panel">
      <header>
        <h2>Remember</h2>
        <button class="icon-btn" onclick={toggleRemember} aria-label="Hide Remember panel" title="Hide">⟩</button>
      </header>
      {#if workspaces.selected}
        <p class="muted small">
          Names, paths and decisions for <strong>{workspaces.selected.name}</strong> will live here.
        </p>
      {/if}
    </aside>
  {/if}
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 232px minmax(0, 1fr) 300px;
    height: 100%;
  }
  .shell.remember-closed {
    grid-template-columns: 232px minmax(0, 1fr);
  }
  .day {
    overflow-y: auto;
    padding: 22px 32px;
    background: var(--bg);
  }
  .day-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 24px;
  }
  .ws-label {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    margin-bottom: 4px;
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
  .empty {
    padding: 48px 0;
    text-align: center;
    border: 1px dashed var(--border-strong);
    border-radius: 12px;
  }
  .empty p {
    margin: 4px 0;
  }
  .center {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 12px;
    height: 100%;
    text-align: center;
  }
  .welcome p {
    max-width: 380px;
    margin: 0 0 8px;
  }
  .muted {
    color: var(--text-2);
  }
  .small {
    font-size: 13px;
  }
  .remember {
    border-left: 1px solid var(--border);
    background: var(--surface);
    padding: 18px 18px;
    overflow-y: auto;
  }
  .remember header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .remember h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .icon-btn {
    width: 26px;
    height: 26px;
    border-radius: 5px;
    color: var(--text-2);
  }
  .icon-btn:hover {
    background: var(--hover);
  }
</style>
