<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { WORKSPACE_COLORS } from "./lib/colors";
  import { day } from "./lib/stores/day.svelte";
  import { workspaces } from "./lib/stores/workspaces.svelte";
  import DayView from "./lib/components/DayView.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Toast from "./lib/components/Toast.svelte";

  let rememberOpen = $state(true);
  let creatingDefaults = $state(false);

  onMount(() => {
    day.init().then(() => workspaces.load());
    api
      .getSetting("remember_panel_open")
      .then((v) => {
        if (v !== null) rememberOpen = v === "true";
      })
      .catch(() => {});

    // Move "Today" forward after midnight or after waking from sleep.
    const tick = setInterval(() => day.checkRollover(), 30_000);
    const onFocus = async () => {
      await day.checkRollover();
      day.load();
    };
    window.addEventListener("focus", onFocus);
    return () => {
      clearInterval(tick);
      window.removeEventListener("focus", onFocus);
    };
  });

  // Reload the day whenever the selected workspace changes.
  $effect(() => {
    if (workspaces.selectedId) day.load();
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
      <div class="day-inner">
        <DayView />
      </div>
      {#if !rememberOpen}
        <button class="remember-tab" onclick={toggleRemember} aria-label="Show Remember panel">Remember</button>
      {/if}
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

<Toast />

<style>
  .shell {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr) 300px;
    height: 100%;
  }
  .shell.remember-closed {
    grid-template-columns: 240px minmax(0, 1fr);
  }
  .day {
    position: relative;
    overflow-y: auto;
    background: var(--bg);
  }
  .day-inner {
    max-width: 760px;
    margin: 0 auto;
    padding: 24px 32px 80px;
  }
  .remember-tab {
    position: absolute;
    top: 24px;
    right: 0;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-right: none;
    border-radius: 8px 0 0 8px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
  }
  .remember-tab:hover {
    color: var(--text);
  }
  .center {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 12px;
    height: 100%;
    text-align: center;
  }
  .welcome h1 {
    margin: 0;
    font-size: 22px;
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
    padding: 18px;
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
