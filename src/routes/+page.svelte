<script lang="ts">
  import { app, type ViewKey } from "$lib/store.svelte";
  import type { FileStatus } from "$lib/api";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Confirm from "$lib/components/Confirm.svelte";
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

  let fetchOpen = $state(false);

  function doFetch(remote: string | null) {
    fetchOpen = false;
    app.fetch(remote);
  }

  async function pushCurrent() {
  const branch = app.repo?.branch;
  const remote = toolbarRemote;
  if (!branch || !remote) return;
  const ahead = app.repo?.ahead ?? 0;
  const parts = [
    ahead > 0
      ? `${ahead} commit${ahead === 1 ? "" : "s"} will be published to ${remote}/${branch}.`
      : `Publishes ${branch} to ${remote}/${branch}.`,
    currentBranch?.upstream ? null : "This also sets upstream tracking.",
  ].filter(Boolean);
  if (
    await app.confirm({
      title: `Push ${branch} to ${remote}?`,
      details: parts.join(" "),
      confirmLabel: "Push",
      danger: false,
    })
  )
    app.push(remote, branch, !currentBranch?.upstream);
}

  function changeCount(): string {
    const isConflict = (f: FileStatus) => f.index === "U" || f.worktree === "U";
    const rows =
      app.staged.filter((f) => !isConflict(f)).length +
      app.unstaged.filter((f) => !isConflict(f)).length +
      app.conflicts.length;
    return rows.toString();
  }

  const VIEW_ORDER: ViewKey[] = [
    "changes",
    "history",
    "branches",
    "tags",
    "stashes",
  ];

  const nav: { key: ViewKey; label: string; count: () => string }[] = [
    { key: "changes", label: "Changes", count: changeCount },
    { key: "history", label: "History", count: () => app.log.length.toString() },
    { key: "branches", label: "Branches", count: () => {
      const localBranches = app.branches.filter((b) => !b.isRemote);
      const remoteBranches = app.branches.filter((b) => b.isRemote);
      return localBranches.length + "/" + remoteBranches.length} },
    { key: "tags", label: "Tags", count: () => app.tags.length.toString() },
    { key: "stashes", label: "Stashes", count: () => app.stashes.length.toString() },
  ];

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape" && fetchOpen) {
      fetchOpen = false;
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key === ",") {
      e.preventDefault();
      app.settingsOpen = true;
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r" && app.repo) {
      e.preventDefault();
      app.refreshAll();
      return;
    }
    if (
      (e.ctrlKey || e.metaKey) &&
      e.shiftKey &&
      e.key.toLowerCase() === "s" &&
      app.repo
    ) {
      e.preventDefault();
      if (!app.conflicts.length) app.stageAll();
      return;
    }
    if (
      (e.ctrlKey || e.metaKey) &&
      !e.shiftKey &&
      /^[1-5]$/.test(e.key) &&
      app.repo
    ) {
      e.preventDefault();
      app.view = VIEW_ORDER[Number(e.key) - 1];
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
            <div class="network">
              <button
                class="btn sm network-pull"
                title={currentBranch?.upstream
                  ? `Pull from ${currentBranch.upstream}`
                  : "Current branch has no upstream"}
                disabled={!currentBranch?.upstream}
                onclick={() => app.pull(null)}>Pull</button
              >
              <button
                class="btn sm network-caret"
                title="Fetch"
                aria-haspopup="menu"
                aria-expanded={fetchOpen}
                onclick={() => (fetchOpen = !fetchOpen)}>▾</button
              >
              {#if fetchOpen}
                <div
                  class="backdrop"
                  role="presentation"
                  onclick={() => (fetchOpen = false)}
                ></div>
                <div class="network-menu" role="menu" aria-label="Fetch">
                  {#each app.remotes as r (r.name)}
                    <button
                      class="menu-item"
                      role="menuitem"
                      onclick={() => doFetch(r.name)}>Fetch {r.name}</button
                    >
                  {/each}
                  {#if app.remotes.length > 1}
                    <button
                      class="menu-item"
                      role="menuitem"
                      onclick={() => doFetch(null)}>Fetch all remotes</button
                    >
                  {/if}
                </div>
              {/if}
            </div>
            <button
              class="btn sm"
              disabled={cannotPush}
              title={
                currentBranch?.upstream
                  ? `Push to ${currentBranch.upstream}`
                  : "Push and set upstream"
              }
              onclick={pushCurrent}>Push</button
            >
            <span class="divider" aria-hidden="true"></span>
          {/if}
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
            {#each nav as item (item.key)}
              <button
                class="nav-item"
                class:active={app.view === item.key}
                onclick={() => (app.view = item.key)}
              >
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
      <button
        class="toast-action"
        aria-label="Dismiss"
        onclick={() => (app.toast = null)}>✕</button
      >
    {/if}
  </div>
{/if}

<Confirm />

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
  .divider {
    width: 1px;
    height: 18px;
    background: var(--border-strong);
    margin: 0 4px;
    flex: 0 0 auto;
  }
  .network {
    position: relative;
    display: inline-flex;
    align-items: stretch;
    flex: 0 0 auto;
  }
  .network-pull {
    border-right: none;
  }
  .network-caret {
    padding: 5px 8px;
    font-size: 10px;
    line-height: 1.1;
  }
  .network-menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 60;
    min-width: 170px;
    display: flex;
    flex-direction: column;
    background: var(--surface-color);
    border: 1px solid var(--border-strong);
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6);
  }
  .menu-item {
    display: block;
    width: 100%;
    padding: 8px 12px;
    border: none;
    border-bottom: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-color);
    font-family: inherit;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    text-align: start;
    cursor: pointer;
  }
  .menu-item:last-child {
    border-bottom: none;
  }
  .menu-item:hover {
    background: var(--hover-color);
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 55;
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

  @media (prefers-reduced-motion: reduce) {
    *,
    *::before,
    *::after {
      transition: none !important;
      animation: none !important;
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
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 14px;
    border: none;
    background: transparent;
    color: var(--muted-color);
    font-family: inherit;
    text-align: start;
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      box-shadow 0.15s ease;
  }
  .nav-item:hover {
    background: var(--hover-color);
    color: var(--text-color);
  }
  .nav-item.active {
    color: var(--text-color);
    background: var(--hover-color);
    box-shadow: inset 0 0 0 1px var(--border-strong);
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
    white-space: nowrap;
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
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    cursor: pointer;
  }
  .toast-action:hover {
    background: var(--hover-color);
  }
  .toast.error {
    border-color: #5a2a2f;
    color: #ffd0cb;
  }
  .toast.success {
    border-color: #25452f;
    color: var(--green);
  }
</style>
