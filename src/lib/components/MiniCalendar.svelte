<script lang="ts">
  import { monthGrid, monthLabel, WEEKDAYS_SHORT } from "../dates";

  interface Props {
    year: number;
    month: number;
    selected: string;
    today: string;
    /** date → "pending" | "done" marker under the day number */
    marks?: Map<string, "pending" | "done">;
    onselect: (date: string) => void;
    onmonth: (delta: number) => void;
  }
  let { year, month, selected, today, marks = new Map(), onselect, onmonth }: Props = $props();

  const cells = $derived(monthGrid(year, month));
</script>

<div class="cal">
  <div class="head">
    <button type="button" class="nav" onclick={() => onmonth(-1)} aria-label="Previous month">‹</button>
    <span class="label">{monthLabel(year, month)}</span>
    <button type="button" class="nav" onclick={() => onmonth(1)} aria-label="Next month">›</button>
  </div>
  <div class="grid" role="grid" aria-label={monthLabel(year, month)}>
    {#each WEEKDAYS_SHORT as w, i (i)}
      <div class="wd" aria-hidden="true">{w}</div>
    {/each}
    {#each cells as c (c.date)}
      <button
        type="button"
        class="day"
        class:out={!c.inMonth}
        class:today={c.date === today}
        class:sel={c.date === selected}
        aria-label={c.date}
        aria-pressed={c.date === selected}
        onclick={() => onselect(c.date)}
      >
        {c.day}
        {#if marks.get(c.date)}
          <span class="mark {marks.get(c.date)}"></span>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .cal {
    user-select: none;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 2px 6px;
  }
  .label {
    font-size: 12px;
    font-weight: 600;
  }
  .nav {
    width: 24px;
    height: 24px;
    border-radius: 5px;
    color: var(--text-2);
    font-size: 16px;
    line-height: 1;
  }
  .nav:hover {
    background: var(--hover);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 1px;
  }
  .wd {
    text-align: center;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-3);
    padding-bottom: 2px;
  }
  .day {
    position: relative;
    height: 26px;
    border-radius: 6px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .day:hover {
    background: var(--hover);
  }
  .day.out {
    color: var(--text-3);
  }
  .day.today {
    color: var(--accent);
    font-weight: 700;
  }
  .day.sel {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
  .mark {
    position: absolute;
    left: 50%;
    bottom: 2px;
    width: 4px;
    height: 4px;
    margin-left: -2px;
    border-radius: 50%;
  }
  .mark.pending {
    background: var(--accent);
  }
  .mark.done {
    background: var(--text-3);
  }
  .day.sel .mark {
    background: var(--accent-text);
  }
</style>
