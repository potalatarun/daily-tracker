<script lang="ts">
  import { tick } from "svelte";
  import { errorMessage, type Note } from "../api";
  import { shortDate, todayYmd, ymdOf } from "../dates";
  import { notes } from "../stores/notes.svelte";
  import { workspaces } from "../stores/workspaces.svelte";

  interface Props {
    onhide: () => void;
  }
  let { onhide }: Props = $props();

  let query = $state("");

  // Row being edited: an existing id, "new" for the add form, or null.
  let editId = $state<string | null>(null);
  let fLabel = $state("");
  let fValue = $state("");
  let fNote = $state("");
  let fError = $state<string | null>(null);
  let saving = $state(false);
  let labelInput: HTMLInputElement | undefined = $state();

  let menuFor = $state<string | null>(null);
  let copiedId = $state<string | null>(null);

  // Reload rows when the workspace changes.
  $effect(() => {
    if (workspaces.selectedId) {
      editId = null;
      query = "";
      notes.load();
    }
  });

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return notes.items;
    return notes.items.filter((n) => `${n.label}\n${n.value}\n${n.note}`.toLowerCase().includes(q));
  });

  async function startEdit(n: Note | null, focus: "label" | "value" | "note" = "label") {
    menuFor = null;
    editId = n ? n.id : "new";
    fLabel = n?.label ?? "";
    fValue = n?.value ?? "";
    fNote = n?.note ?? "";
    fError = null;
    await tick();
    const el = document.querySelector<HTMLElement>(`.editor [data-field="${focus}"]`);
    (el ?? labelInput)?.focus();
  }

  async function save() {
    if (!editId) return;
    saving = true;
    try {
      if (editId === "new") {
        if (!fLabel.trim() && !fValue.trim()) {
          if (fNote.trim()) {
            fError = "Add a label or a value";
            return;
          }
          editId = null;
          return;
        }
        await notes.create(fLabel, fValue, fNote);
      } else {
        await notes.update(editId, { label: fLabel, value: fValue, note: fNote });
      }
      editId = null;
    } catch (e) {
      fError = errorMessage(e);
    } finally {
      saving = false;
    }
  }

  function editorKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      editId = null;
    }
    const isNote = (e.target as HTMLElement).dataset.field === "note";
    if (e.key === "Enter" && (!isNote || e.metaKey || e.ctrlKey) && !e.shiftKey) {
      e.preventDefault();
      save();
    }
  }

  async function copy(n: Note) {
    const text = n.value || n.label;
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Older web views: fall back to a temporary textarea.
      const ta = document.createElement("textarea");
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      ta.remove();
    }
    copiedId = n.id;
    setTimeout(() => {
      if (copiedId === n.id) copiedId = null;
    }, 1500);
  }

  function updatedLabel(n: Note): string {
    const d = ymdOf(n.updatedAt);
    return d === todayYmd() ? "today" : shortDate(d);
  }
</script>

<svelte:window onclick={() => (menuFor = null)} />

{#snippet editor()}
  <div class="editor" role="group" aria-label={editId === "new" ? "New row" : "Edit row"}>
    <label>
      <span>Label</span>
      <input class="input" data-field="label" bind:this={labelInput} bind:value={fLabel} maxlength="200" placeholder="e.g. Secrets file" onkeydown={editorKey} oninput={() => (fError = null)} />
    </label>
    <label>
      <span>Value</span>
      <input class="input mono" data-field="value" bind:value={fValue} maxlength="2000" placeholder="e.g. secrets.env" onkeydown={editorKey} oninput={() => (fError = null)} />
    </label>
    <label>
      <span>Note <em>optional</em></span>
      <textarea class="input" data-field="note" rows="2" bind:value={fNote} maxlength="2000" placeholder="Why, where, when to change it…" onkeydown={editorKey}></textarea>
    </label>
    {#if fError}<div class="error-text">{fError}</div>{/if}
    <div class="editor-actions">
      {#if editId !== "new"}
        {@const n = notes.items.find((x) => x.id === editId)}
        {#if n}
          <button type="button" class="del" onclick={() => { editId = null; notes.remove(n); }}>Delete</button>
        {/if}
      {/if}
      <button type="button" class="btn small" onclick={() => (editId = null)}>Cancel</button>
      <button type="button" class="btn primary small" disabled={saving} onclick={save}>Save</button>
    </div>
  </div>
{/snippet}

<aside class="panel" aria-label="Remember panel">
  <header>
    <h2>Remember {#if notes.items.length}<span class="count">{notes.items.length}</span>{/if}</h2>
    <button class="icon-btn" onclick={onhide} aria-label="Hide Remember panel" title="Hide">⟩</button>
  </header>

  {#if notes.items.length > 0}
    <div class="search">
      <input class="input" type="search" placeholder="Search labels, values, notes" aria-label="Search the Remember panel" bind:value={query} />
    </div>
  {/if}

  {#if notes.error}<p class="error-text">{notes.error}</p>{/if}

  {#if notes.items.length === 0 && editId !== "new"}
    <div class="empty">
      <p>Keep names, paths and decisions for <strong>{workspaces.selected?.name}</strong> here, so you never have to dig for them again.</p>
      <div class="example" aria-hidden="true">
        <span class="ex-label">Secrets file</span>
        <span class="ex-value">secrets.env</span>
      </div>
      <button class="btn primary small" onclick={() => startEdit(null)}>+ Add your first row</button>
    </div>
  {:else}
    {#if filtered.length > 0}
      <div class="cols" aria-hidden="true"><span>Label</span><span>Value</span></div>
    {/if}
    <ul>
      {#each filtered as n, i (n.id)}
        <li class="row" class:editing={editId === n.id}>
          {#if editId === n.id}
            {@render editor()}
          {:else}
            <button type="button" class="cell label" onclick={() => startEdit(n, "label")} title="Edit">{n.label || "—"}</button>
            <div class="value-wrap">
              <button type="button" class="cell value" onclick={() => startEdit(n, "value")} title="Edit">{n.value || "—"}</button>
              {#if n.value || n.label}
                <button type="button" class="copy" class:done={copiedId === n.id} onclick={() => copy(n)} aria-label="Copy {n.value || n.label}" title="Copy">
                  {copiedId === n.id ? "Copied" : "Copy"}
                </button>
              {/if}
            </div>
            {#if n.note}
              <button type="button" class="cell note" onclick={() => startEdit(n, "note")} title="Edit">{n.note}</button>
            {/if}
            <div class="foot">
              <span class="updated" title={new Date(n.updatedAt).toLocaleString()}>Updated {updatedLabel(n)}</span>
              <div class="menu-wrap">
                <button
                  type="button"
                  class="more"
                  aria-label="Options for {n.label || n.value}"
                  aria-expanded={menuFor === n.id}
                  onclick={(e) => {
                    e.stopPropagation();
                    menuFor = menuFor === n.id ? null : n.id;
                  }}>⋯</button
                >
                {#if menuFor === n.id}
                  <div class="menu" role="menu">
                    <button type="button" role="menuitem" onclick={() => startEdit(n)}>Edit</button>
                    {#if !query}
                      <button type="button" role="menuitem" disabled={i === 0} onclick={() => notes.move(n.id, -1)}>Move up</button>
                      <button type="button" role="menuitem" disabled={i === filtered.length - 1} onclick={() => notes.move(n.id, 1)}>Move down</button>
                    {/if}
                    <div class="sep"></div>
                    <button type="button" role="menuitem" class="danger" onclick={() => notes.remove(n)}>Delete</button>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </li>
      {/each}
    </ul>

    {#if query && filtered.length === 0}
      <p class="muted">No rows match “{query}”.</p>
    {/if}

    {#if editId === "new"}
      <div class="row editing">{@render editor()}</div>
    {:else}
      <button class="add" onclick={() => startEdit(null)}>+ Add row</button>
    {/if}
  {/if}

  <p class="hint">Saved as plain text on this computer. Note <em>where</em> a password or key lives, not the secret itself.</p>
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    padding: 18px 16px;
    border-left: 1px solid var(--border);
    background: var(--surface);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .count {
    margin-left: 4px;
    font-weight: 500;
    color: var(--text-3);
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
  .search {
    margin-bottom: 10px;
  }
  .cols {
    display: grid;
    grid-template-columns: 40% 60%;
    padding: 0 8px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .row {
    display: grid;
    grid-template-columns: 40% 60%;
    align-items: start;
    padding: 8px;
    border-top: 1px solid var(--border);
  }
  .row:last-child {
    border-bottom: 1px solid var(--border);
  }
  .row:hover {
    background: var(--hover);
  }
  .row.editing {
    display: block;
    background: var(--bg);
  }
  .cell {
    min-width: 0;
    text-align: left;
    overflow-wrap: anywhere;
    border-radius: 4px;
  }
  .label {
    padding-right: 8px;
    font-weight: 600;
    font-size: 13px;
  }
  .value-wrap {
    display: flex;
    align-items: flex-start;
    gap: 4px;
    min-width: 0;
  }
  .value {
    flex: 1;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12.5px;
  }
  .note {
    grid-column: 1 / -1;
    margin-top: 4px;
    color: var(--text-2);
    font-size: 12.5px;
    white-space: pre-line;
  }
  .copy {
    flex: none;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    opacity: 0;
  }
  .row:hover .copy,
  .copy:focus-visible,
  .copy.done {
    opacity: 1;
  }
  .copy:hover {
    background: var(--active);
  }
  .copy.done {
    color: var(--text-2);
  }
  .foot {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 18px;
    margin-top: 2px;
  }
  .updated {
    font-size: 11px;
    color: var(--text-3);
    opacity: 0;
  }
  .row:hover .updated {
    opacity: 1;
  }
  .menu-wrap {
    position: relative;
  }
  .more {
    width: 24px;
    height: 18px;
    border-radius: 4px;
    color: var(--text-2);
    opacity: 0;
  }
  .row:hover .more,
  .more:focus-visible,
  .more[aria-expanded="true"] {
    opacity: 1;
  }
  .more:hover {
    background: var(--hover);
  }
  .menu {
    position: absolute;
    right: 0;
    top: 20px;
    z-index: 20;
    min-width: 140px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
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
  }
  .menu .danger {
    color: var(--danger);
  }
  .sep {
    height: 1px;
    margin: 4px 2px;
    background: var(--border);
  }
  .editor {
    display: grid;
    gap: 8px;
  }
  .editor label {
    display: grid;
    gap: 3px;
  }
  .editor label span {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-2);
  }
  .editor em {
    font-style: normal;
    font-weight: 400;
    color: var(--text-3);
  }
  .editor .input {
    height: 30px;
  }
  .editor textarea.input {
    height: auto;
    padding: 6px 10px;
    resize: vertical;
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12.5px;
  }
  .editor-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  .del {
    margin-right: auto;
    color: var(--danger);
    font-size: 13px;
  }
  .btn.small {
    height: 28px;
    padding: 0 10px;
  }
  .add {
    align-self: flex-start;
    margin-top: 8px;
    padding: 6px 0;
    color: var(--accent);
    font-weight: 500;
  }
  .add:hover {
    text-decoration: underline;
  }
  .empty {
    display: grid;
    gap: 12px;
    justify-items: start;
    color: var(--text-2);
    font-size: 13px;
  }
  .empty p {
    margin: 0;
  }
  .example {
    display: grid;
    grid-template-columns: 40% 60%;
    width: 100%;
    padding: 8px;
    border: 1px dashed var(--border-strong);
    border-radius: 8px;
    opacity: 0.8;
  }
  .ex-label {
    font-weight: 600;
    color: var(--text);
  }
  .ex-value {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }
  .muted {
    color: var(--text-2);
    font-size: 13px;
  }
  .hint {
    margin: auto 0 0;
    padding-top: 16px;
    font-size: 11.5px;
    color: var(--text-3);
  }
</style>
