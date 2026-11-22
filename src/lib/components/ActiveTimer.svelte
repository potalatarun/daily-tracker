<script lang="ts">
  import { formatClock } from "../dates";
  import { clock } from "../stores/clock.svelte";
  import { day } from "../stores/day.svelte";
  import { timer } from "../stores/timer.svelte";
  import { workspaces } from "../stores/workspaces.svelte";

  const a = $derived(timer.active);
  const ws = $derived(a ? workspaces.items.find((w) => w.id === a.workspaceId) : undefined);

  async function show() {
    if (!a) return;
    if (workspaces.selectedId !== a.workspaceId) await workspaces.select(a.workspaceId);
    day.setDate(a.taskDate > day.today ? a.taskDate : day.today);
  }
</script>

{#if a}
  <div class="bar" role="status" aria-label="Timer running">
    <button type="button" class="info" onclick={show} title="Show this task">
      <span class="pulse" aria-hidden="true"></span>
      <span class="text">
        <span class="title">{a.taskTitle}</span>
        {#if ws && ws.id !== workspaces.selectedId}<span class="ws">{ws.name}</span>{/if}
      </span>
      <span class="clock">{formatClock(a.previousMs + clock.now - a.startedAt)}</span>
    </button>
    <button type="button" class="pause" onclick={() => timer.pause(a.taskId)} aria-label="Pause timer" title="Pause">
      <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="4" y="3" width="3" height="10" rx="1" /><rect x="9" y="3" width="3" height="10" rx="1" /></svg>
    </button>
  </div>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 0 0 14px;
    padding: 4px 4px 4px 8px;
    border-radius: 8px;
    background: var(--active);
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
  }
  .pulse {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.4s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse {
      animation: none;
    }
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 500;
  }
  .ws {
    font-size: 11px;
    color: var(--text-2);
  }
  .clock {
    flex: none;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 13px;
    color: var(--accent);
  }
  .pause {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--accent);
  }
  .pause:hover {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .pause svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }
</style>
