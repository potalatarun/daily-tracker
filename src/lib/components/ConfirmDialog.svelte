<script lang="ts">
  interface Props {
    title: string;
    message: string;
    confirmLabel?: string;
    danger?: boolean;
    onconfirm: () => void | Promise<void>;
    oncancel: () => void;
  }
  let { title, message, confirmLabel = "Confirm", danger = false, onconfirm, oncancel }: Props =
    $props();

  let busy = $state(false);
  let cancelBtn: HTMLButtonElement | undefined = $state();

  $effect(() => cancelBtn?.focus());

  async function confirm() {
    busy = true;
    try {
      await onconfirm();
    } finally {
      busy = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") oncancel();
  }
</script>

<div class="backdrop" role="presentation" onclick={oncancel}>
  <div
    class="dialog"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => {
      e.stopPropagation();
      onkeydown(e);
    }}
  >
    <h2 id="confirm-title">{title}</h2>
    <p>{message}</p>
    <div class="actions">
      <button class="btn" bind:this={cancelBtn} onclick={oncancel} disabled={busy}>Cancel</button>
      <button class="btn" class:danger class:primary={!danger} onclick={confirm} disabled={busy}>
        {confirmLabel}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: grid;
    place-items: center;
    z-index: 100;
  }
  .dialog {
    width: min(400px, calc(100vw - 48px));
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
    padding: 20px;
  }
  h2 {
    margin: 0 0 8px;
    font-size: 16px;
    font-weight: 600;
  }
  p {
    margin: 0 0 20px;
    color: var(--text-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
