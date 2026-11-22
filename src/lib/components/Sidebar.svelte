<script lang="ts">
  import { tick } from "svelte";
  import { errorMessage, type Workspace } from "../api";
  import { nextColor } from "../colors";
  import { workspaces } from "../stores/workspaces.svelte";
  import { day } from "../stores/day.svelte";
  import ActiveTimer from "./ActiveTimer.svelte";
  import ColorPicker from "./ColorPicker.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import MiniCalendar from "./MiniCalendar.svelte";

  // Calendar dots: accent = something still to do that day, grey = only history.
  const marks = $derived.by(() => {
    const m = new Map<string, "pending" | "done">();
    for (const d of day.month.days) {
      m.set(d.date, d.pending > 0 && d.date >= day.today ? "pending" : "done");
    }
    if (day.month.carriedToday > 0) m.set(day.today, "pending");
    return m;
  });

  // ── menu ────────────────────────────────────────────────
  let menuFor = $state<string | null>(null);
  let pickingColor = $state(false);

  function openMenu(e: MouseEvent, id: string) {
    e.stopPropagation();
    pickingColor = false;
    menuFor = menuFor === id ? null : id;
  }
  function closeMenu() {
    menuFor = null;
    pickingColor = false;
  }

  // ── rename ──────────────────────────────────────────────
  let renamingId = $state<string | null>(null);
  let renameValue = $state("");
  let renameError = $state<string | null>(null);
  let renameInput: HTMLInputElement | undefined = $state();

  async function startRename(w: Workspace) {
    closeMenu();
    renamingId = w.id;
    renameValue = w.name;
    renameError = null;
    await tick();
    renameInput?.select();
  }
  async function commitRename() {
    if (!renamingId) return;
    const w = workspaces.items.find((x) => x.id === renamingId);
    if (w && renameValue.trim() === w.name) {
      renamingId = null;
      return;
    }
    try {
      await workspaces.update(renamingId, { name: renameValue });
      renamingId = null;
    } catch (e) {
      renameError = errorMessage(e);
      renameInput?.focus();
    }
  }
  function renameKey(e: KeyboardEvent) {
    if (e.key === "Enter") commitRename();
    if (e.key === "Escape") renamingId = null;
  }

  // ── create ──────────────────────────────────────────────
  let adding = $state(false);
  let newName = $state("");
  let newColor = $state("");
  let addError = $state<string | null>(null);
  let addInput: HTMLInputElement | undefined = $state();

  async function startAdd() {
    adding = true;
    newName = "";
    addError = null;
    newColor = nextColor(workspaces.items.map((w) => w.color));
    await tick();
    addInput?.focus();
  }
  async function commitAdd() {
    if (!newName.trim()) {
      adding = false;
      return;
    }
    try {
      const w = await workspaces.create(newName, newColor);
      await workspaces.select(w.id);
      adding = false;
    } catch (e) {
      addError = errorMessage(e);
      addInput?.focus();
    }
  }
  function addKey(e: KeyboardEvent) {
    if (e.key === "Enter") commitAdd();
    if (e.key === "Escape") adding = false;
  }

  // ── delete ──────────────────────────────────────────────
  let confirmDelete = $state<Workspace | null>(null);

  // ── drag to reorder ─────────────────────────────────────
  let dragId = $state<string | null>(null);
  let dropIndex = $state<number | null>(null);

  function onDragStart(e: DragEvent, id: string) {
    dragId = id;
    e.dataTransfer?.setData("text/plain", id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }
  function onDragOver(e: DragEvent, index: number) {
    if (!dragId) return;
    e.preventDefault();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    dropIndex = e.clientY < rect.top + rect.height / 2 ? index : index + 1;
  }
  async function onDrop(e: DragEvent) {
    e.preventDefault();
    if (dragId !== null && dropIndex !== null) {
      const from = workspaces.items.findIndex((w) => w.id === dragId);
      const to = dropIndex > from ? dropIndex - 1 : dropIndex;
      await workspaces.moveTo(dragId, to);
    }
    dragId = null;
    dropIndex = null;
  }
  function onDragEnd() {
    dragId = null;
    dropIndex = null;
  }
</script>

<svelte:window onclick={closeMenu} onkeydown={(e) => e.key === "Escape" && closeMenu()} />

<nav class="sidebar" aria-label="Workspaces">
  <div class="brand">
    <img src="/icon.svg" alt="" width="22" height="22" />
    <span>Daily Tracker</span>
  </div>

  <ActiveTimer />

  <div class="section-title">Workspaces</div>

  <ul class="list" role="list" ondragover={(e) => e.preventDefault()} ondrop={onDrop}>
    {#each workspaces.items as w, i (w.id)}
      <li
        class="item"
        class:selected={w.id === workspaces.selectedId}
        class:dragging={w.id === dragId}
        class:drop-before={dropIndex === i && dragId !== w.id}
        class:drop-after={dropIndex === i + 1 && i === workspaces.items.length - 1}
        draggable={renamingId !== w.id}
        ondragstart={(e) => onDragStart(e, w.id)}
        ondragover={(e) => onDragOver(e, i)}
        ondragend={onDragEnd}
      >
        {#if renamingId === w.id}
          <div class="rename">
            <span class="dot" style:background={w.color}></span>
            <input
              class="input small"
              bind:this={renameInput}
              bind:value={renameValue}
              onkeydown={renameKey}
              onblur={commitRename}
              maxlength="40"
              aria-label="Workspace name"
            />
          </div>
          {#if renameError}<div class="error-text pad">{renameError}</div>{/if}
        {:else}
          <button
            class="row"
            onclick={() => workspaces.select(w.id)}
            ondblclick={() => startRename(w)}
            aria-current={w.id === workspaces.selectedId ? "page" : undefined}
          >
            <span class="dot" style:background={w.color}></span>
            <span class="name">{w.name}</span>
          </button>
          <button
            class="more"
            class:open={menuFor === w.id}
            aria-label="Options for {w.name}"
            aria-haspopup="menu"
            aria-expanded={menuFor === w.id}
            onclick={(e) => openMenu(e, w.id)}>⋯</button
          >
        {/if}

        {#if menuFor === w.id}
          <div
            class="menu"
            role="menu"
            tabindex="-1"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
          >
            {#if pickingColor}
              <div class="menu-pad">
                <ColorPicker
                  value={w.color}
                  onpick={async (hex) => {
                    await workspaces.update(w.id, { color: hex });
                    closeMenu();
                  }}
                />
              </div>
            {:else}
              <button role="menuitem" onclick={() => startRename(w)}>Rename</button>
              <button role="menuitem" onclick={() => (pickingColor = true)}>Change color</button>
              <button
                role="menuitem"
                disabled={i === 0}
                onclick={() => {
                  closeMenu();
                  workspaces.moveTo(w.id, i - 1);
                }}>Move up</button
              >
              <button
                role="menuitem"
                disabled={i === workspaces.items.length - 1}
                onclick={() => {
                  closeMenu();
                  workspaces.moveTo(w.id, i + 1);
                }}>Move down</button
              >
              <div class="sep"></div>
              <button
                role="menuitem"
                class="danger"
                onclick={() => {
                  closeMenu();
                  confirmDelete = w;
                }}>Delete…</button
              >
            {/if}
          </div>
        {/if}
      </li>
    {/each}
  </ul>

  {#if adding}
    <div class="add-form">
      <input
        class="input small"
        placeholder="Workspace name"
        bind:this={addInput}
        bind:value={newName}
        onkeydown={addKey}
        maxlength="40"
        aria-label="New workspace name"
      />
      <ColorPicker value={newColor} onpick={(hex) => (newColor = hex)} />
      {#if addError}<div class="error-text">{addError}</div>{/if}
      <div class="add-actions">
        <button class="btn small" onclick={() => (adding = false)}>Cancel</button>
        <button class="btn primary small" onclick={commitAdd}>Add</button>
      </div>
    </div>
  {:else}
    <button class="add" onclick={startAdd}>
      <span class="plus">+</span> New workspace
    </button>
  {/if}

  {#if workspaces.selected}
    <div class="calendar">
      <MiniCalendar
        year={day.calYear}
        month={day.calMonth}
        selected={day.date}
        today={day.today}
        {marks}
        onselect={(d) => day.setDate(d)}
        onmonth={(d) => day.shiftMonth(d)}
      />
    </div>
  {/if}
</nav>

{#if confirmDelete}
  {@const w = confirmDelete}
  <ConfirmDialog
    title={`Delete "${w.name}"?`}
    message="All of its tasks and Remember notes will be deleted too. This can't be undone."
    confirmLabel="Delete workspace"
    danger
    oncancel={() => (confirmDelete = null)}
    onconfirm={async () => {
      await workspaces.remove(w.id);
      confirmDelete = null;
    }}
  />
{/if}

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 14px 10px;
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    overflow-y: auto;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 8px 18px;
    font-weight: 600;
    font-size: 15px;
  }
  .section-title {
    padding: 0 8px 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .item {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    border-radius: 6px;
  }
  .item:hover {
    background: var(--hover);
  }
  .item.selected {
    background: var(--active);
  }
  .item.dragging {
    opacity: 0.4;
  }
  .item.drop-before {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .item.drop-after {
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  .row {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 8px;
    text-align: left;
  }
  .item.selected .name {
    font-weight: 600;
  }
  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .more {
    width: 26px;
    height: 26px;
    margin-right: 3px;
    border-radius: 5px;
    color: var(--text-2);
    opacity: 0;
  }
  .item:hover .more,
  .more.open,
  .more:focus-visible {
    opacity: 1;
  }
  .more:hover {
    background: var(--hover);
  }
  .rename {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 2px 4px 2px 8px;
  }
  .pad {
    padding: 0 8px 4px 28px;
  }
  .input.small {
    height: 28px;
  }
  .menu {
    position: absolute;
    top: 30px;
    right: 0;
    z-index: 20;
    min-width: 170px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
  }
  .menu button {
    display: block;
    width: 100%;
    padding: 6px 10px;
    border-radius: 5px;
    text-align: left;
  }
  .menu button:hover:not(:disabled) {
    background: var(--hover);
  }
  .menu button:disabled {
    color: var(--text-3);
    cursor: default;
  }
  .menu .danger {
    color: var(--danger);
  }
  .menu-pad {
    padding: 8px;
  }
  .sep {
    height: 1px;
    margin: 4px 2px;
    background: var(--border);
  }
  .add {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    margin-top: 4px;
    padding: 0 8px;
    border-radius: 6px;
    color: var(--text-2);
  }
  .add:hover {
    background: var(--hover);
    color: var(--text);
  }
  .plus {
    width: 10px;
    text-align: center;
    font-size: 16px;
  }
  .add-form {
    display: grid;
    gap: 10px;
    margin-top: 6px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
  }
  .add-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  .calendar {
    margin-top: auto;
    padding: 14px 4px 0;
    border-top: 1px solid var(--border);
  }
  .btn.small {
    height: 28px;
    padding: 0 10px;
  }
</style>
