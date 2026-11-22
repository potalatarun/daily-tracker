<script lang="ts">
  import { toast } from "../stores/toast.svelte";
</script>

{#if toast.message}
  <div class="toast" role="status" aria-live="polite">
    <span>{toast.message}</span>
    {#if toast.action}
      <button type="button" onclick={() => toast.runAction()}>{toast.action.label}</button>
    {/if}
    <button type="button" class="x" aria-label="Dismiss" onclick={() => toast.hide()}>×</button>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: 20px;
    transform: translateX(-50%);
    z-index: 200;
    display: flex;
    align-items: center;
    gap: 14px;
    max-width: min(520px, calc(100vw - 40px));
    padding: 10px 10px 10px 16px;
    border-radius: 10px;
    background: #2c2c2a;
    color: #f1efe8;
    box-shadow: var(--shadow);
  }
  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  button {
    font-weight: 600;
    color: #afa9ec;
    padding: 2px 6px;
    border-radius: 5px;
  }
  button:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .x {
    color: #b4b2a9;
    font-weight: 400;
    font-size: 16px;
  }
</style>
