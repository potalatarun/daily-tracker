<script lang="ts">
  import { WORKSPACE_COLORS } from "../colors";

  interface Props {
    value: string;
    onpick: (hex: string) => void;
  }
  let { value, onpick }: Props = $props();
</script>

<div class="swatches" role="radiogroup" aria-label="Workspace color">
  {#each WORKSPACE_COLORS as c (c.hex)}
    <button
      type="button"
      role="radio"
      aria-checked={value.toUpperCase() === c.hex}
      aria-label={c.name}
      title={c.name}
      class="swatch"
      class:on={value.toUpperCase() === c.hex}
      style:--c={c.hex}
      onclick={() => onpick(c.hex)}
    ></button>
  {/each}
</div>

<style>
  .swatches {
    display: grid;
    grid-template-columns: repeat(8, 18px);
    gap: 6px;
  }
  .swatch {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--c);
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.1);
  }
  .swatch.on {
    box-shadow:
      0 0 0 2px var(--surface),
      0 0 0 4px var(--c);
  }
</style>
