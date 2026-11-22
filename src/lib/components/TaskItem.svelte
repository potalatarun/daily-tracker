<script lang="ts">
  import type { TaskView } from "../api";
  import { plural, shortDate } from "../dates";

  interface Props {
    task: TaskView;
    today: string;
    ontoggle: () => void;
    onedit: () => void;
    ondelete: () => void;
    ongo: (date: string) => void;
  }
  let { task: t, today, ontoggle, onedit, ondelete, ongo }: Props = $props();

  let menuOpen = $state(false);
  const done = $derived(t.status === "done");
  const ghost = $derived(t.display === "moved" || t.display === "completedLater");
</script>

<svelte:window onclick={() => (menuOpen = false)} />

<li class="task" class:done class:ghost>
  <button
    type="button"
    class="check"
    class:on={done}
    role="checkbox"
    aria-checked={done}
    aria-label={done ? `Mark "${t.title}" as not done` : `Mark "${t.title}" as done`}
    onclick={ontoggle}
  >
    {#if done}<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>{/if}
  </button>

  <div class="body">
    <button type="button" class="title" onclick={onedit}>{t.title}</button>
    {#if t.description}
      <p class="desc">{t.description}</p>
    {/if}

    {#if t.display === "carried" && t.carryDays}
      <div class="meta">
        <span class="badge carried" title="Not finished on {shortDate(t.originalDate, today)}, so it carried over automatically">
          From {shortDate(t.originalDate, today)} · carried {plural(t.carryDays, "day")}
        </span>
      </div>
    {:else if t.display === "moved"}
      <div class="meta">
        <button type="button" class="badge link" onclick={() => ongo(today)}>→ Moved to today</button>
      </div>
    {:else if t.display === "completedLater" && t.completedOn}
      <div class="meta">
        <button type="button" class="badge link" onclick={() => ongo(t.completedOn!)}>
          ✓ Completed {shortDate(t.completedOn, today)}
        </button>
      </div>
    {:else if done && t.doneAfterDays}
      <div class="meta">
        <span class="badge">From {shortDate(t.originalDate, today)} · done after {plural(t.doneAfterDays, "day")}</span>
      </div>
    {/if}
  </div>

  <div class="menu-wrap">
    <button
      type="button"
      class="more"
      class:open={menuOpen}
      aria-label="Options for {t.title}"
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      onclick={(e) => {
        e.stopPropagation();
        menuOpen = !menuOpen;
      }}>⋯</button
    >
    {#if menuOpen}
      <div class="menu" role="menu">
        <button type="button" role="menuitem" onclick={onedit}>Edit</button>
        <button type="button" role="menuitem" onclick={ontoggle}>{done ? "Mark not done" : "Mark done"}</button>
        <div class="sep"></div>
        <button type="button" role="menuitem" class="danger" onclick={ondelete}>Delete</button>
      </div>
    {/if}
  </div>
</li>

<style>
  .task {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 10px 12px 14px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
  }
  .task + :global(.task) {
    margin-top: 8px;
  }
  .task.ghost {
    opacity: 0.6;
    background: transparent;
    border-style: dashed;
  }
  .check {
    flex: none;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-top: 1px;
    border-radius: 50%;
    border: 1.5px solid var(--border-strong);
    transition: background 0.12s, border-color 0.12s;
  }
  .check:hover {
    border-color: var(--accent);
    background: var(--active);
  }
  .check.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .check svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: var(--accent-text);
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  .title {
    display: block;
    max-width: 100%;
    text-align: left;
    font-weight: 500;
    line-height: 22px;
    overflow-wrap: anywhere;
  }
  .title:hover {
    color: var(--accent);
  }
  .done .title {
    color: var(--text-3);
    text-decoration: line-through;
  }
  .desc {
    margin: 2px 0 0;
    color: var(--text-2);
    font-size: 13px;
    white-space: pre-line;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .done .desc {
    color: var(--text-3);
  }
  .meta {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    font-size: 12px;
    color: var(--text-2);
    background: var(--hover);
  }
  .badge.carried {
    color: #854f0b;
    background: #faeeda;
  }
  @media (prefers-color-scheme: dark) {
    .badge.carried {
      color: #fac775;
      background: #412402;
    }
  }
  .badge.link:hover {
    color: var(--accent);
    background: var(--active);
  }
  .menu-wrap {
    position: relative;
  }
  .more {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--text-2);
    opacity: 0;
  }
  .task:hover .more,
  .more.open,
  .more:focus-visible {
    opacity: 1;
  }
  .more:hover {
    background: var(--hover);
  }
  .menu {
    position: absolute;
    top: 30px;
    right: 0;
    z-index: 20;
    min-width: 160px;
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
  .menu button:hover {
    background: var(--hover);
  }
  .menu .danger {
    color: var(--danger);
  }
  .sep {
    height: 1px;
    margin: 4px 2px;
    background: var(--border);
  }
</style>
