<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    onclose,
    children,
    footer,
  }: {
    title: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onkey} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="overlay"
  onclick={onclose}
  onkeydown={(e) => e.key === "Escape" && onclose()}
  role="presentation"
>
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <div class="head-titles">
        <h3>{title}</h3>
      </div>
      <button class="btn ghost icon" onclick={onclose} aria-label="Close">✕</button>
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(2, 3, 5, 0.78);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    width: min(480px, 92vw);
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    background: var(--surface-color);
    border: 1px solid var(--border-strong);
    border-radius: 0;
    box-shadow: 0 40px 80px rgba(0, 0, 0, 0.6);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 18px;
    border-bottom: 1px solid var(--border-color);
  }
  .head-titles {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  h3 {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .body {
    padding: 18px;
    overflow: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 18px;
    border-top: 1px solid var(--border-color);
  }
</style>
