<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorMessage, type AppInfo } from "../api";
  import { toast } from "../stores/toast.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  interface Props {
    onclose: () => void;
  }
  let { onclose }: Props = $props();

  const SNOOZE_CHOICES = [5, 10, 15, 30, 60];

  let info = $state<AppInfo | null>(null);
  let snooze = $state(10);
  let autoPopup = $state(true);
  let error = $state<string | null>(null);
  let confirm = $state<null | "import" | "delete">(null);
  let deleteText = $state("");

  onMount(async () => {
    try {
      info = await api.getAppInfo();
      const s = Number(await api.getSetting("default_snooze_minutes"));
      if (s > 0) snooze = s;
      autoPopup = (await api.getSetting("auto_popup")) !== "false";
    } catch (e) {
      error = errorMessage(e);
    }
  });

  async function act(fn: () => Promise<void>) {
    error = null;
    try {
      await fn();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  const setSnooze = (m: number) =>
    act(async () => {
      snooze = m;
      await api.setSetting("default_snooze_minutes", String(m));
    });

  const setAutoPopup = (on: boolean) =>
    act(async () => {
      autoPopup = on;
      await api.setSetting("auto_popup", String(on));
    });

  const setLogin = (on: boolean) =>
    act(async () => {
      const now = await api.setStartAtLogin(on);
      if (info) info = { ...info, startAtLogin: now };
    });

  const exportData = () =>
    act(async () => {
      const path = await api.exportData();
      if (path) toast.show(`Exported to ${path.split(/[\\/]/).pop()}`);
    });

  const importData = () =>
    act(async () => {
      confirm = null;
      const r = await api.importData();
      // The app reloads itself on "data-replaced"; this is just in case.
      if (r) toast.show(`Imported ${r.workspaces} workspaces, ${r.tasks} tasks, ${r.notes} notes`);
    });

  const deleteAll = () =>
    act(async () => {
      confirm = null;
      await api.deleteAllData();
    });

  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape" && !confirm) onclose();
  }
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    {onkeydown}
  >
    <header>
      <h2 id="settings-title">Settings</h2>
      <button class="x" aria-label="Close settings" onclick={onclose}>×</button>
    </header>

    {#if error}<p class="error-text">{error}</p>{/if}

    <section>
      <h3>Reminders</h3>
      <div class="row">
        <div>
          <div class="label">Default snooze</div>
          <div class="desc">Highlighted first in the reminder window.</div>
        </div>
        <div class="seg" role="radiogroup" aria-label="Default snooze">
          {#each SNOOZE_CHOICES as m (m)}
            <button role="radio" aria-checked={snooze === m} class:on={snooze === m} onclick={() => setSnooze(m)}>
              {m < 60 ? `${m}m` : "1h"}
            </button>
          {/each}
        </div>
      </div>
      <label class="row">
        <div>
          <div class="label">Open the reminder window automatically</div>
          <div class="desc">Recommended. Notifications can be silenced by the system; the window makes sure you see every reminder.</div>
        </div>
        <input type="checkbox" class="switch" checked={autoPopup} onchange={(e) => setAutoPopup(e.currentTarget.checked)} />
      </label>
      <div class="row">
        <div>
          <div class="label">Test notification</div>
          <div class="desc">If nothing appears, allow notifications for Daily Tracker in your system settings.</div>
        </div>
        <button class="btn small" onclick={() => act(() => api.sendTestNotification())}>Send test</button>
      </div>
    </section>

    <section>
      <h3>Startup</h3>
      <label class="row">
        <div>
          <div class="label">Start at login</div>
          <div class="desc">Starts quietly in the tray so reminders keep working after a restart.</div>
        </div>
        <input type="checkbox" class="switch" checked={info?.startAtLogin ?? false} disabled={!info} onchange={(e) => setLogin(e.currentTarget.checked)} />
      </label>
    </section>

    <section>
      <h3>Your data</h3>
      <div class="row">
        <div>
          <div class="label">Stored on this computer</div>
          <div class="desc path" title={info?.dataDir}>{info?.dataDir ?? "…"}</div>
        </div>
        <button class="btn small" onclick={() => act(() => api.openDataFolder())}>Open folder</button>
      </div>
      <div class="row">
        <div>
          <div class="label">Back up or move to another computer</div>
          <div class="desc">Export saves everything to one file. Import replaces all current data with a file.</div>
        </div>
        <div class="pair">
          <button class="btn small" onclick={exportData}>Export…</button>
          <button class="btn small" onclick={() => (confirm = "import")}>Import…</button>
        </div>
      </div>
      <div class="row danger-zone">
        <div>
          <div class="label">Delete all my data</div>
          <div class="desc">Removes every workspace, task, reminder, note and backup. Can't be undone.</div>
        </div>
        <button class="btn small danger-outline" onclick={() => ((deleteText = ""), (confirm = "delete"))}>Delete…</button>
      </div>
    </section>

    <footer>Daily Tracker {info?.version ?? ""} · MIT License</footer>
  </div>
</div>

{#if confirm === "import"}
  <ConfirmDialog
    title="Replace all data with an export?"
    message="Everything currently in Daily Tracker will be replaced by the file you pick. A backup copy of your current data is saved in the data folder first."
    confirmLabel="Choose file…"
    oncancel={() => (confirm = null)}
    onconfirm={importData}
  />
{/if}

{#if confirm === "delete"}
  <div class="backdrop top" role="presentation" onclick={() => (confirm = null)}>
    <div
      class="dialog small-dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="del-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        e.stopPropagation();
        if (e.key === "Escape") confirm = null;
      }}
    >
      <h2 id="del-title">Delete all data?</h2>
      <p class="desc">This permanently removes everything. Consider exporting a backup first. Type <strong>DELETE</strong> to confirm.</p>
      <input class="input" bind:value={deleteText} aria-label="Type DELETE to confirm" />
      <div class="confirm-actions">
        <button class="btn" onclick={() => (confirm = null)}>Cancel</button>
        <button class="btn danger" disabled={deleteText.trim() !== "DELETE"} onclick={deleteAll}>Delete everything</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: start center;
    padding-top: 8vh;
    background: rgba(0, 0, 0, 0.35);
  }
  .backdrop.top {
    z-index: 110;
    padding-top: 20vh;
  }
  .dialog {
    width: min(600px, calc(100vw - 48px));
    max-height: 84vh;
    overflow-y: auto;
    padding: 20px 22px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .small-dialog {
    width: min(420px, calc(100vw - 48px));
    display: grid;
    gap: 12px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }
  .x {
    width: 30px;
    height: 30px;
    border-radius: 6px;
    font-size: 20px;
    color: var(--text-2);
  }
  .x:hover {
    background: var(--hover);
  }
  section {
    margin-top: 18px;
  }
  h3 {
    margin: 0 0 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .row > div:first-child {
    min-width: 0;
  }
  .label {
    font-weight: 500;
  }
  .desc {
    margin: 2px 0 0;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .path {
    max-width: 380px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
  }
  .seg {
    display: flex;
    flex: none;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    overflow: hidden;
  }
  .seg button {
    height: 30px;
    padding: 0 10px;
    font-size: 13px;
    border-left: 1px solid var(--border);
  }
  .seg button:first-child {
    border-left: none;
  }
  .seg button.on {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
  .switch {
    flex: none;
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .pair {
    display: flex;
    flex: none;
    gap: 6px;
  }
  .btn.small {
    flex: none;
    height: 30px;
    padding: 0 12px;
  }
  .danger-outline {
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 50%, transparent);
  }
  .danger-zone {
    border-bottom: none;
  }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  footer {
    margin-top: 16px;
    font-size: 12px;
    color: var(--text-3);
  }
</style>
