<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api, errorMessage, type UpdateInfo } from "./lib/api";
  import { WORKSPACE_COLORS } from "./lib/colors";
  import { day } from "./lib/stores/day.svelte";
  import { workspaces } from "./lib/stores/workspaces.svelte";
  import DayView from "./lib/components/DayView.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import RememberPanel from "./lib/components/RememberPanel.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import Onboarding from "./lib/components/Onboarding.svelte";
  import SettingsDialog from "./lib/components/SettingsDialog.svelte";
  import { ui } from "./lib/stores/ui.svelte";

  let rememberOpen = $state(true);
  let creatingDefaults = $state(false);
  let waitingAlerts = $state(0);
  let update = $state<UpdateInfo | null>(null);
  let updating = $state(false);
  let updateError = $state<string | null>(null);

  async function installUpdate() {
    updating = true;
    updateError = null;
    try {
      await api.installUpdate(); // the app restarts when done
    } catch (e) {
      updateError = errorMessage(e);
      updating = false;
    }
  }
  /** null while loading, true while first-run setup is showing. */
  let onboarding = $state<boolean | null>(null);

  onMount(() => {
    day.init()
      .then(() => workspaces.load())
      .then(async () => {
        // First run: no workspaces and setup not done yet. Existing users
        // (who already have workspaces) skip setup.
        const done = await api.getSetting("onboarded").catch(() => "1");
        if (done) onboarding = false;
        else if (workspaces.items.length > 0) {
          onboarding = false;
          api.setSetting("onboarded", "1").catch(() => {});
        } else onboarding = true;
      });
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

    // Events from the core: reminders fired or handled in the popup, midnight passed.
    const loadAlerts = () => api.getAlerts().then((a) => (waitingAlerts = a.length)).catch(() => {});
    loadAlerts();
    const unlisteners = [
      listen("data-changed", () => day.load()),
      listen("day-changed", () => day.checkRollover()),
      listen("alerts-changed", loadAlerts),
      // Import or "delete all data" replaced everything: start fresh.
      listen("data-replaced", () => window.location.reload()),
      listen<UpdateInfo>("update-available", (e) => (update = e.payload)),
    ];

    return () => {
      clearInterval(tick);
      window.removeEventListener("focus", onFocus);
      unlisteners.forEach((u) => u.then((f) => f()));
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

{#if onboarding}
  <div class="onboarding">
    <Onboarding ondone={() => (onboarding = false)} />
  </div>
{:else}
<div class="shell" class:remember-closed={!rememberOpen || !workspaces.selected}>
  <Sidebar />

  <main class="day">
    {#if !workspaces.loaded || onboarding === null}
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
        {#if update}
          <div class="update-bar" role="status">
            <span>
              <strong>Daily Tracker {update.version}</strong> is available
              {#if updateError}<span class="error-text"> · {updateError}</span>{/if}
            </span>
            <button class="btn primary small" disabled={updating} onclick={installUpdate}>
              {updating ? "Updating…" : "Update & restart"}
            </button>
            <button class="later" onclick={() => (update = null)}>Later</button>
          </div>
        {/if}
        {#if waitingAlerts > 0}
          <button class="alerts-banner" onclick={() => api.showAlerts()}>
            <span aria-hidden="true">🔔</span>
            {waitingAlerts === 1 ? "1 reminder is waiting" : `${waitingAlerts} reminders are waiting`}
            <span class="open">Open</span>
          </button>
        {/if}
        <DayView />
      </div>
      {#if !rememberOpen}
        <button class="remember-tab" onclick={toggleRemember} aria-label="Show Remember panel">Remember</button>
      {/if}
    {/if}
  </main>

  {#if rememberOpen && workspaces.selected}
    <RememberPanel onhide={toggleRemember} />
  {/if}
</div>
{/if}

{#if ui.settingsOpen}
  <SettingsDialog onclose={() => (ui.settingsOpen = false)} />
{/if}

<Toast />

<style>
  .onboarding {
    height: 100%;
    overflow-y: auto;
    background: var(--bg);
  }
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
  .update-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 16px;
    padding: 8px 10px 8px 14px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
  }
  .update-bar > span {
    flex: 1;
  }
  .update-bar .btn.small {
    height: 28px;
    padding: 0 12px;
  }
  .update-bar .later {
    color: var(--text-2);
    font-size: 13px;
    padding: 4px 6px;
  }
  .alerts-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    margin-bottom: 16px;
    padding: 10px 14px;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
    background: var(--active);
    text-align: left;
    font-weight: 500;
  }
  .alerts-banner .open {
    margin-left: auto;
    color: var(--accent);
    font-weight: 600;
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
</style>
