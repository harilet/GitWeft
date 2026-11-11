<script lang="ts">
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { app, type ViewKey } from "$lib/store.svelte";
  import Welcome from "$lib/components/Welcome.svelte";
  import Changes from "$lib/components/Changes.svelte";
  import History from "$lib/components/History.svelte";
  import Branches from "$lib/components/Branches.svelte";
  import Tags from "$lib/components/Tags.svelte";
  import Stashes from "$lib/components/Stashes.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import AiPanel from "$lib/components/AiPanel.svelte";
  import ConflictResolver from "$lib/components/ConflictResolver.svelte";
  import DiffView from "$lib/components/DiffView.svelte";
  import BranchSwitcher from "$lib/components/BranchSwitcher.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";

  const currentBranch = $derived(app.branches.find((b) => b.isHead));
  const toolbarRemote = $derived(
    app.remotes.find(
      (r) =>
        currentBranch?.upstream === r.name ||
        currentBranch?.upstream?.startsWith(r.name + "/"),
    )?.name ??
      app.remotes[0]?.name ??
      null,
  );
  const cannotPush = $derived(!app.repo || app.repo.detached || app.repo.unborn);

  const nav: { key: ViewKey; label: string; count: () => string }[] = [
    { key: "changes", label: "Changes test", count: () => app.status.length.toString() },
    { key: "history", label: "History", count: () => app.log.length.toString() },
    { key: "branches", label: "Branches", count: () => {
      const localBranches = app.branches.filter((b) => !b.isRemote);
      const remoteBranches = app.branches.filter((b) => b.isRemote);
      return localBranches.length + "/" + remoteBranches.length} },
    { key: "tags", label: "Tags", count: () => app.tags.length.toString() },
    { key: "stashes", label: "Stashes", count: () => app.stashes.length.toString() },
  ];

  function onkey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === ",") {
      e.preventDefault();
      app.settingsOpen = true;
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r" && app.repo) {
      e.preventDefault();
      app.refreshAll();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="root">
  <TitleBar />
  <div class="main">
    {#if !app.repo}
      <Welcome />
    {:else}
      <div class="app">
        <header class="toolbar">
          <div class="repo">
            <span class="repo-name">{app.repo.name}</span>
            <BranchSwitcher />
            {#if app.repo.dirty}<span class="badge yellow">modified</span>{/if}
          </div>

          <div class="spacer"></div>

          <div class="tools">
            {#if app.remotes.length}
              <button class="btn sm" onclick={() => app.fetch(null)}>Fetch</button>
              <button
                class="btn sm"
                title={currentBranch?.upstream
                  ? `Pull from ${currentBranch.upstream}`
                  : "Current branch has no upstream"}
                disabled={!currentBranch?.upstream}
                onclick={() => app.pull(null)}>Pull</button
              >
              <button
                class="btn sm"
                disabled={cannotPush}
                onclick={async () => {
                  const branch = app.repo?.branch ?? "current branch";
                  if (
                    await confirm(
                      `Push ${branch} to ${toolbarRemote ?? "remote"}?`,
                    )
                  )
                    app.push(
                      toolbarRemote,
                      app.repo?.branch ?? null,
                      !currentBranch?.upstream,
                    );
                }}>Push</button
              >
            {/if}
            <button
              class="btn sm"
              onclick={() => app.refreshAll()}
              title="Refresh (Ctrl+R)">Refresh</button
            >
            {#if !app.aiDisabled}
              <button
                class="btn sm"
                title="Repository AI assistant"
                onclick={() => app.openAi("chat")}>AI</button
              >
            {/if}
            <button
              class="btn sm"
              title="Settings (Ctrl+,)"
              onclick={() => (app.settingsOpen = true)}>Settings</button
            >
          </div>

          {#if app.progress}
            <span class="progress label" title={app.progress}>{app.progress}</span>
          {/if}
          {#if app.busy}<span class="spinner" title="Working…"></span>{/if}
        </header>

        <div class="body">
          <nav class="rail">
            <div class="rail-head">
              <span class="label">Workspace</span>
              <span class="label">{String(nav.length).padStart(2, "0")}</span>
            </div>

            {#each nav as item, i (item.key)}
              <button
                class="nav-item"
                class:active={app.view === item.key}
                onclick={() => (app.view = item.key)}
              >
                <span class="nav-index">{String(i + 1).padStart(2, "0")}</span>
                <span class="nav-label">{item.label}</span>
                <span class="count">{item.count()}</span>
              </button>
            {/each}

            <div class="spacer"></div>

            <div class="rail-foot">
              <div class="rail-block">
                <span class="label">Path</span>
                <span class="rail-path" title={app.repo.path}>{app.repo.path}</span>
              </div>
              <div class="rail-block">
                <span class="label">Head</span>
                <span class="rail-path" title={app.repo.head}>
                  {app.repo.detached
                    ? "detached"
                    : (app.repo.branch || "unborn")}{app.repo.head
                    ? ` · ${app.repo.head.slice(0, 7)}`
                    : ""}
                </span>
              </div>
            </div>
          </nav>

          <main class="content">
            {#if app.view === "changes"}
              <Changes />
            {:else if app.view === "history"}
              <History />
            {:else if app.view === "branches"}
              <Branches />
            {:else if app.view === "tags"}
              <Tags />
            {:else if app.view === "stashes"}
              <Stashes />
            {/if}
          </main>

          {#if app.diff}
            <aside class="diff-panel">
              <header class="panel-head diff-head">
                <div class="diff-titles">
                  <span class="diff-title">{app.diff.title}</span>
                  <span class="label">{app.diff.subtitle}</span>
                </div>
                <button
                  class="btn ghost icon"
                  aria-label="Close diff"
                  onclick={() => (app.diff = null)}>✕</button
                >
              </header>
              <div class="diff-body">
                <DiffView patch={app.diff.patch} />
              </div>
            </aside>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

{#if app.toast}
  <div class="toast {app.toast.kind}">
    <span class="toast-text">{app.toast.text}</span>
    {#if app.toast.kind === "error"}
      <button
        class="toast-action"
        onclick={() => {
          app.toast = null;
          app.openErrorLog();
        }}>View log</button
      >
    {/if}
  </div>
{/if}

{#if app.settingsOpen}
  <Settings />
{/if}

{#if app.aiOpen}
  <AiPanel />
{/if}

{#if app.conflict}
  {#key app.conflict.path}
    <ConflictResolver />
  {/key}
{/if}

<style>
  .root {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .main {
    flex: 1;
    min-height: 0;
    display: flex;
    overflow: hidden;
  }
  .app {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    height: 100%;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    background: var(--surface-color);
    border-bottom: 1px solid var(--border-color);
    flex: 0 0 auto;
    min-height: 46px;
  }
  .repo {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .repo-name {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.04em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .progress {
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    letter-spacing: 0.02em;
    text-transform: none;
  }
  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--primary-color);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex: 0 0 auto;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .rail {
    display: flex;
    flex-direction: column;
    width: 188px;
    flex: 0 0 auto;
    background: var(--surface-color);
    border-right: 1px solid var(--border-color);
    overflow: hidden;
  }
  .rail-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-color);
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 14px;
    border: none;
    border-left: 2px solid transparent;
    border-bottom: 1px solid transparent;
    background: transparent;
    color: var(--muted-color);
    font-family: inherit;
    text-align: start;
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease;
  }
  .nav-item:hover {
    background: var(--hover-color);
    color: var(--text-color);
  }
  .nav-item.active {
    color: var(--text-color);
    background: var(--hover-color);
    border-left-color: var(--primary-color);
  }
  .nav-index {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--muted-color);
    flex: 0 0 auto;
  }
  .nav-item.active .nav-index {
    color: var(--primary-color);
  }
  .nav-label {
    flex: 1;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-size: 10px;
    color: var(--muted-color);
    min-width: 18px;
    text-align: right;
    flex: 0 0 auto;
  }
  .nav-item.active .count {
    color: var(--text-color);
  }

  .rail-foot {
    border-top: 1px solid var(--border-color);
    padding: 4px 0 8px;
  }
  .rail-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 9px 16px;
  }
  .rail-path {
    font-size: 10px;
    line-height: 1.5;
    color: var(--muted-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .diff-panel {
    display: flex;
    flex-direction: column;
    width: 48%;
    min-width: 340px;
    flex: 0 0 auto;
    border-left: 1px solid var(--border-color);
    background: var(--background-color);
  }
  .diff-head {
    padding: 8px 14px;
  }
  .diff-titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .diff-title {
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .diff-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .toast {
    position: fixed;
    right: 16px;
    bottom: 16px;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    max-width: 420px;
    padding: 11px 14px;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    box-shadow: 0 20px 44px rgba(0, 0, 0, 0.55);
    font-size: 12px;
    z-index: 200;
    white-space: pre-wrap;
  }
  .toast-text {
    flex: 1;
    min-width: 0;
  }
  .toast-action {
    flex: 0 0 auto;
    padding: 3px 7px;
    border: 1px solid var(--border-strong);
    background: transparent;
    color: inherit;
    font-family: inherit;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    cursor: pointer;
  }
  .toast-action:hover {
    background: var(--hover-color);
  }
  .toast.error {
    border-color: #6e2a2a;
    color: #ffdcd7;
  }
  .toast.success {
    border-color: #25452f;
    color: #aff5b4;
  }
</style>
