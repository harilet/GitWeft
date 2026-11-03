<script lang="ts">
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { app } from "../store.svelte";
  import { autofocus } from "../actions";
  import Modal from "./Modal.svelte";

  let tab = $state<"branches" | "remotes">("branches");

  let showCreate = $state(false);
  let newName = $state("");
  let checkoutAfter = $state(true);

  let showAdd = $state(false);
  let remoteName = $state("origin");
  let remoteUrl = $state("");

  const currentBranch = $derived(app.branches.find((b) => b.isHead));
  const hasUpstream = $derived(!!currentBranch?.upstream);

  function trackedBy(remote: string) {
    const upstream = currentBranch?.upstream;
    return (
      !!upstream && (upstream === remote || upstream.startsWith(remote + "/"))
    );
  }

  async function create() {
    if (!newName.trim()) return;
    const ok = await app.createBranch(newName.trim(), checkoutAfter);
    if (ok) {
      showCreate = false;
      newName = "";
    }
  }

  async function addRemote() {
    if (!remoteName.trim() || !remoteUrl.trim()) return;
    const ok = await app.addRemote(remoteName.trim(), remoteUrl.trim());
    if (ok) {
      showAdd = false;
      remoteUrl = "";
    }
  }

  async function removeBranch(name: string, merged: boolean) {
    if (
      !merged &&
      !(await confirm(
        `Branch "${name}" has unmerged commits. Force delete it? Unmerged commits will be lost.`,
      ))
    )
      return;
    if (!(await confirm(`Delete branch "${name}"?`))) return;
    await app.deleteBranch(name, !merged);
  }
</script>

<div class="branches">
  <header class="panel-head">
    <div class="titles">
      <span class="label">Branches</span>
      {#if tab === "branches"}
        <span class="label"
          >{app.localBranches.length} local &middot; {app.remoteBranches.length}
          remote-tracking</span
        >
      {:else}
        <span class="label"
          >{app.remotes.length}
          {app.remotes.length === 1 ? "remote" : "remotes"} configured</span
        >
      {/if}
    </div>

    <div class="head-actions">
      <div class="seg">
        <button
          class:active={tab === "branches"}
          onclick={() => (tab = "branches")}>Branches</button
        >
        <button class:active={tab === "remotes"} onclick={() => (tab = "remotes")}
          >Remotes</button
        >
      </div>
      {#if tab === "branches"}
        <button class="btn sm" onclick={() => (showCreate = true)}>
          New branch
        </button>
      {:else}
        <button class="btn sm" onclick={() => (showAdd = true)}>Add remote</button>
      {/if}
    </div>
  </header>

  {#if tab === "branches"}
    <div class="content main-scrollbar">
      <div class="section-head">
        <span class="label">Local branches</span>
        {#if currentBranch}
          <span class="label">
            on {currentBranch.name}
            {#if hasUpstream}· tracking {currentBranch?.upstream}{:else}· no
              upstream{/if}
          </span>
        {/if}
      </div>

      {#each app.localBranches as b (b.name)}
        <div class="row" class:current={b.isHead}>
          <span class="mark" class:on={b.isHead}>{b.isHead ? "●" : "○"}</span>
          <span class="name">{b.name}</span>
          {#if b.upstream}
            <span class="up label">→ {b.upstream}</span>
          {/if}
          {#if b.ahead}
            <span class="badge blue" title="{b.ahead} commit(s) ahead">↑{b.ahead}</span>
          {/if}
          {#if b.behind}
            <span class="badge yellow" title="{b.behind} commit(s) behind"
              >↓{b.behind}</span
            >
          {/if}
          <span class="spacer"></span>
          <div class="actions">
            {#if !b.isHead}
              <button class="btn ghost sm" onclick={() => app.checkout(b.name)}>
                Checkout</button
              >
              <button class="btn ghost sm" onclick={() => app.merge(b.name)}>
                Merge</button
              >
              <button
                class="btn ghost sm danger-text"
                onclick={() => removeBranch(b.name, b.merged)}>Delete</button
              >
            {:else}
              <span class="badge green">current</span>
            {/if}
          </div>
        </div>
      {:else}
        <div class="empty">No local branches</div>
      {/each}

      {#if app.remoteBranches.length}
        <div class="section-head">
          <span class="label">Remote-tracking</span>
          <span class="label">{app.remoteBranches.length}</span>
        </div>
        {#each app.remoteBranches as b (b.name)}
          <div class="row">
            <span class="mark">R</span>
            <span class="name">{b.name}</span>
            <span class="spacer"></span>
            <div class="actions">
              <button class="btn ghost sm" onclick={() => app.checkout(b.name)}>
                Checkout</button
              >
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="content main-scrollbar">
      <div class="section-head">
        <span class="label">Configured remotes</span>
        <span class="label">{app.remotes.length}</span>
      </div>

      {#each app.remotes as remote (remote.name)}
        <div class="row remote-row">
          <div class="remote-info">
            <span class="name">{remote.name}</span>
            <span class="url">{remote.url}</span>
          </div>
          <span class="spacer"></span>
          <div class="actions">
            <button class="btn ghost sm" onclick={() => app.fetch(remote.name)}>
              Fetch</button
            >
            <button
              class="btn ghost sm"
              disabled={!currentBranch}
              onclick={() =>
                app.pull(
                  remote.name,
                  trackedBy(remote.name) ? null : (currentBranch?.name ?? null),
                )}>Pull</button
            >
            <button
              class="btn ghost sm"
              disabled={!currentBranch}
              onclick={async () => {
                if (
                  await confirm(
                    `Push ${currentBranch?.name} to ${remote.name}?`,
                  )
                )
                  app.push(
                    remote.name,
                    currentBranch?.name ?? null,
                    !trackedBy(remote.name),
                  );
              }}>Push</button
            >
            <button
              class="btn ghost sm danger-text"
              onclick={async () => {
                if (await confirm(`Remove remote "${remote.name}"?`))
                  app.removeRemote(remote.name);
              }}>Remove</button
            >
          </div>
        </div>
      {:else}
        <div class="empty">No remotes configured</div>
      {/each}

      {#if currentBranch}
        <div class="current-bar">
          <span class="label">Current branch</span>
          <span class="mono">{currentBranch.name}</span>
          {#if hasUpstream}
            <span class="badge green">tracking {currentBranch.upstream}</span>
            {#if app.repo?.ahead}
              <span class="badge blue">↑ {app.repo.ahead} ahead</span>
            {/if}
            {#if app.repo?.behind}
              <span class="badge yellow">↓ {app.repo.behind} behind</span>
            {/if}
          {:else}
            <span class="badge yellow">no upstream</span>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if showCreate}
  <Modal title="Create branch" onclose={() => (showCreate = false)}>
    <label class="field">
      <span>Branch name</span>
      <input
        class="input"
        placeholder="feature/my-branch"
        bind:value={newName}
        use:autofocus
      />
    </label>
    <label class="row check">
      <input type="checkbox" bind:checked={checkoutAfter} />
      <span>Checkout after creating</span>
    </label>
    {#snippet footer()}
      <button class="btn" onclick={() => (showCreate = false)}>Cancel</button>
      <button class="btn primary" disabled={!newName.trim()} onclick={create}>
        Create
      </button>
    {/snippet}
  </Modal>
{/if}

{#if showAdd}
  <Modal title="Add remote" onclose={() => (showAdd = false)}>
    <label class="field">
      <span>Name</span>
      <input class="input" bind:value={remoteName} use:autofocus />
    </label>
    <label class="field">
      <span>URL</span>
      <input
        class="input"
        placeholder="git@github.com:user/repo.git"
        bind:value={remoteUrl}
      />
    </label>
    {#snippet footer()}
      <button class="btn" onclick={() => (showAdd = false)}>Cancel</button>
      <button
        class="btn primary"
        disabled={!remoteName.trim() || !remoteUrl.trim()}
        onclick={addRemote}>Add</button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .branches {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .titles {
    display: flex;
    align-items: baseline;
    gap: 12px;
    min-width: 0;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .content {
    flex: 1;
    min-height: 0;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px 6px;
    border-bottom: 1px solid var(--border-color);
    background: var(--background-color);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-bottom: 1px solid rgba(36, 39, 47, 0.7);
    border-left: 2px solid transparent;
    min-width: 0;
  }
  .row:hover {
    background: var(--hover-color);
  }
  .row.current {
    background: var(--surface-2);
    border-left-color: var(--green);
  }
  .mark {
    color: var(--muted-color);
    font-size: 9px;
    flex: 0 0 auto;
    width: 12px;
    text-align: center;
  }
  .mark.on {
    color: var(--green);
  }
  .name {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 0 1 auto;
  }
  .up {
    font-size: 10px;
    text-transform: none;
    letter-spacing: 0.02em;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spacer {
    flex: 1;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: 0 0 auto;
  }
  .danger-text {
    color: var(--muted-color);
  }
  .danger-text:hover {
    color: var(--red);
    border-color: var(--red);
  }
  .remote-row {
    align-items: center;
    padding: 12px 14px;
  }
  .remote-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .url {
    font-size: 10px;
    color: var(--muted-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .current-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    padding: 14px;
    margin-top: 8px;
    border-top: 1px solid var(--border-color);
    font-size: 11px;
  }
  .check {
    gap: 8px;
    font-size: 12px;
  }
  .check input {
    accent-color: var(--primary-color);
  }
</style>
