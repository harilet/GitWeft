<script lang="ts">
  import { app } from "../store.svelte";

  let boxEl = $state<HTMLDivElement | null>(null);
  let cancelEl = $state<HTMLButtonElement | null>(null);
  let confirmEl = $state<HTMLButtonElement | null>(null);

  const req = $derived(app.confirmState);

  // Safe by default: focus the non-destructive button — destructive asks
  // land on Cancel (Enter/Space then cancel; a Tab is needed to reach the
  // destructive button), benign asks land on the confirm button.
  $effect(() => {
    if (!req) return;
    if (req.danger === false) confirmEl?.focus();
    else cancelEl?.focus();
  });

  function settle(value: boolean | null) {
    app.settleConfirm(value);
  }

  function onkey(e: KeyboardEvent) {
    if (!req) return;
    if (e.key === "Escape") {
      e.preventDefault();
      settle(false);
    } else if (e.key === "Enter" && req.danger === false) {
      e.preventDefault();
      settle(true);
    }
  }
</script>

<svelte:window onkeydown={onkey} />

{#if req}
  <div class="overlay" role="presentation">
    <div
      class="box"
      role="alertdialog"
      aria-modal="true"
      aria-label={req.title}
      tabindex="-1"
      bind:this={boxEl}
    >
      <div class="head">
        {#if req.danger !== false}
          <span class="flag" aria-hidden="true"></span>
        {/if}
        <h3>{req.title}</h3>
      </div>
      {#if req.message}<p class="message">{req.message}</p>{/if}
      {#if req.details}
        <p class="details" class:danger={req.danger !== false}>{req.details}</p>
      {/if}
      <footer>
        <button
          class="btn"
          bind:this={cancelEl}
          onclick={() => settle(false)}>
          {req.cancelLabel ?? "Cancel"}
        </button>
        {#if req.extra}
          <button class="btn ghost" onclick={() => settle(null)}>
            {req.extra.label}
          </button>
        {/if}
        <button
          class="btn {req.danger === false ? 'primary' : 'danger'}"
          bind:this={confirmEl}
          onclick={() => settle(true)}>
          {req.confirmLabel ?? "Confirm"}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(2, 3, 5, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 150;
  }
  .box {
    width: min(460px, 92vw);
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px;
    background: var(--surface-color);
    border: 1px solid var(--border-strong);
    box-shadow: 0 40px 80px rgba(0, 0, 0, 0.6);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .flag {
    width: 8px;
    height: 8px;
    background: var(--red);
    flex: 0 0 auto;
  }
  h3 {
    font-size: 15px;
    font-weight: 700;
    overflow-wrap: anywhere;
  }
  .message {
    margin: 0;
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .details {
    margin: 0;
    padding: 10px 12px;
    font-size: 11px;
    line-height: 1.6;
    color: var(--muted-color);
    background: var(--surface-2);
    border: 1px solid var(--border-color);
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }
  .details.danger {
    background: #2a171b;
    border-color: #5a2a2f;
    color: #ffd0cb;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
  }
</style>
