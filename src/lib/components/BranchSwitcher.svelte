<script lang="ts">
  import { tick } from "svelte";
  import { app } from "../store.svelte";
  import { autofocus } from "../actions";

  let open = $state(false);
  let filter = $state("");
  let active = $state(-1);
  let menuEl = $state<HTMLDivElement | null>(null);

  const locals = $derived(app.localBranches.filter(match));
  const remotes = $derived(app.remoteBranches.filter(match));
  const items = $derived([...locals.map((b) => b.name), ...remotes.map((b) => b.name)]);
  const current = $derived(app.repo?.branch);

  function match(b: { name: string }) {
    const f = filter.trim().toLowerCase();
    return !f || b.name.toLowerCase().includes(f);
  }

  function toggle() {
    open = !open;
    if (open) {
      filter = "";
      active = -1;
    }
  }

  async function highlight(delta: number) {
    if (items.length === 0) return;
    active =
      active === -1 && delta > 0 && delta === 1
        ? 0
        : ((active + delta) % items.length + items.length) % items.length;
    await tick();
    menuEl
      ?.querySelector(".item.active")
      ?.scrollIntoView({ block: "nearest" });
  }

  function menukey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      open = false;
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      void highlight(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      void highlight(-1);
    } else if (e.key === "Enter" && active >= 0 && active < items.length) {
      e.preventDefault();
      void pick(items[active]);
    }
  }

  async function pick(name: string) {
    open = false;
    if (name === current) return;
    await app.checkout(name);
  }
</script>

<svelte:window onkeydown={menukey} />

<div class="switcher">
  <button
    class="chip"
    class:open
    onclick={toggle}
    aria-haspopup="listbox"
    aria-expanded={open}
    title="{app.repo?.path} — click to switch branch"
  >
    <span class="dot" class:dirty={app.repo?.dirty}></span>
    <span class="branch-name">{current}</span>
    {#if app.repo?.ahead}
      <span class="badge blue" title="{app.repo.ahead} commit(s) ahead"
        >↑{app.repo.ahead}</span
      >
    {/if}
    {#if app.repo?.behind}
      <span class="badge yellow" title="{app.repo.behind} commit(s) behind"
        >↓{app.repo.behind}</span
      >
    {/if}
    <span class="caret">▾</span>
  </button>

  {#if open}
    <div class="backdrop" onclick={() => (open = false)} role="presentation"></div>
    <div class="menu" role="listbox" aria-label="Switch branch" bind:this={menuEl}>
      <div class="filter">
        <span class="prompt">/</span>
        <input
          class="filter-input"
          placeholder="Switch branch…"
          bind:value={filter}
          oninput={() => (active = -1)}
          use:autofocus
        />
      </div>
      <div class="list main-scrollbar">
        {#each locals as b, i (b.name)}
          <button
            class="item"
            class:current={b.isHead}
            class:active={i === active}
            role="option"
            aria-selected={i === active}
            onclick={() => pick(b.name)}
          >
            <span class="check">{b.isHead ? "●" : ""}</span>
            <span class="bname">{b.name}</span>
            {#if b.upstream}<span class="up">→ {b.upstream}</span>{/if}
          </button>
        {/each}

        {#if remotes.length}
          <div class="divider label">Remote branches</div>
          {#each remotes as b, j (b.name)}
            <button
              class="item"
              class:active={locals.length + j === active}
              role="option"
              aria-selected={locals.length + j === active}
              onclick={() => pick(b.name)}
            >
              <span class="check"></span>
              <span class="bname remote-name">{b.name}</span>
            </button>
          {/each}
        {/if}

        {#if locals.length === 0 && remotes.length === 0}
          <div class="empty">No branches yet</div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .switcher {
    position: relative;
    display: inline-flex;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 10px;
    border: 1px solid var(--border-strong);
    border-radius: 0;
    background: transparent;
    color: var(--text-color);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
  }
  .chip:hover,
  .chip.open {
    border-color: var(--primary-color);
    background: var(--hover-color);
  }
  .dot {
    width: 6px;
    height: 6px;
    background: var(--green);
    flex: 0 0 auto;
  }
  .dot.dirty {
    background: var(--yellow);
  }
  .branch-name {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .caret {
    font-size: 10px;
    color: var(--muted-color);
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 50;
    width: 320px;
    max-height: 400px;
    display: flex;
    flex-direction: column;
    background: var(--surface-color);
    border: 1px solid var(--border-strong);
    border-radius: 0;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }
  .filter {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-color);
    flex: 0 0 auto;
  }
  .prompt {
    color: var(--primary-color);
    font-size: 12px;
  }
  .filter-input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-color);
    font-family: inherit;
    font-size: 12px;
  }
  .filter-input::placeholder {
    color: var(--muted-color);
  }
  .list {
    overflow: auto;
    flex: 1;
    min-height: 0;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border: none;
    border-bottom: 1px solid rgba(36, 39, 47, 0.6);
    background: transparent;
    color: var(--text-color);
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    font-size: 12px;
    transition: background 0.15s ease;
  }
  .item:hover,
  .item.active {
    background: var(--hover-color);
  }
  .item.current {
    background: var(--surface-2);
  }
  .check {
    width: 10px;
    color: var(--green);
    font-size: 10px;
    flex: 0 0 auto;
  }
  .bname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .remote-name {
    color: var(--muted-color);
  }
  .up {
    font-size: 10px;
    color: var(--muted-color);
    flex: 0 0 auto;
  }
  .divider {
    padding: 10px 12px 5px;
    border-top: 1px solid var(--border-color);
    border-bottom: 1px solid var(--border-color);
    background: var(--background-color);
  }
</style>
