<script lang="ts">
  import { longDate, parseYmd, relativeDay, shortDate } from "../dates";
  import MiniCalendar from "./MiniCalendar.svelte";

  interface Props {
    value: string;
    today: string;
    onchange: (date: string) => void;
    /** "long" = "Sunday, 22 November · Today", "short" = "Nov 22". */
    format?: "long" | "short";
    /** Optional label id for accessibility. */
    labelledby?: string;
    /** Disallow dates after this one (inclusive bound). */
    max?: string;
  }
  let { value, today, onchange, format = "long", labelledby, max }: Props = $props();

  let open = $state(false);
  let year = $state(0);
  let month = $state(0);

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    const d = parseYmd(value);
    year = d.getFullYear();
    month = d.getMonth();
    open = !open;
  }

  function pick(d: string) {
    if (max && d > max) return;
    open = false;
    onchange(d);
  }

  const rel = $derived(relativeDay(value, today));
</script>

<svelte:window onclick={() => (open = false)} />

<div class="wrap" class:short={format === "short"}>
  <button
    type="button"
    class="btn-date"
    aria-labelledby={labelledby}
    aria-haspopup="dialog"
    aria-expanded={open}
    onclick={toggle}
  >
    <span class="cal-icon" aria-hidden="true"></span>
    {#if format === "long"}
      {longDate(value)}
      {#if ["Today", "Tomorrow", "Yesterday"].includes(rel)}<span class="rel">· {rel}</span>{/if}
    {:else}
      {["Today", "Yesterday"].includes(rel) ? rel : shortDate(value, today)}
    {/if}
  </button>
  {#if open}
    <div
      class="pop"
      role="dialog"
      aria-label="Choose date"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          open = false;
        }
      }}
    >
      <MiniCalendar
        {year}
        {month}
        selected={value}
        {today}
        onselect={pick}
        onmonth={(d) => {
          const n = new Date(year, month + d, 1);
          year = n.getFullYear();
          month = n.getMonth();
        }}
      />
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    flex: 1;
  }
  .wrap.short {
    flex: none;
  }
  .btn-date {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--surface);
    text-align: left;
    white-space: nowrap;
  }
  .btn-date:hover {
    border-color: var(--text-3);
  }
  .rel {
    color: var(--text-2);
  }
  .cal-icon {
    flex: none;
    width: 14px;
    height: 13px;
    border: 1.5px solid var(--text-2);
    border-top-width: 3px;
    border-radius: 3px;
  }
  .pop {
    position: absolute;
    top: 36px;
    left: 0;
    z-index: 30;
    width: 250px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    box-shadow: var(--shadow);
  }
</style>
