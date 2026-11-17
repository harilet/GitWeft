<script lang="ts">
  import type { CommitInfo } from "../api";
  import { app } from "../store.svelte";
  import { relativeTime } from "../format";
  import Graph from "./Graph.svelte";

  let filter = $state("");
  let mode = $state<"commits" | "graph">("commits");
  let filterInput = $state<HTMLInputElement | null>(null);

  function filterKey(e: KeyboardEvent) {
    if (e.key !== "/" || e.ctrlKey || e.metaKey || e.altKey) return;
    const t = e.target as HTMLElement | null;
    if (
      t &&
      (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)
    )
      return;
    e.preventDefault();
    filterInput?.focus();
    filterInput?.select();
  }

  function rowKey(e: KeyboardEvent, commit: CommitInfo) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      load(commit);
    }
  }

  const refsByOid = $derived.by(() => {
    const map = new Map<string, { label: string; kind: string }[]>();
    for (const b of app.branches) {
      if (!b.target) continue;
      const list = map.get(b.target) ?? [];
      const label = b.name;
      list.push({ label, kind: b.isHead ? "head" : b.isRemote ? "remote" : "branch" });
      map.set(b.target, list);
    }
    for (const t of app.tags) {
      const list = map.get(t.target) ?? [];
      list.push({ label: t.name, kind: "tag" });
      map.set(t.target, list);
    }
    return map;
  });

  const commits = $derived.by(() => {
    const f = filter.trim().toLowerCase();
    if (!f) return app.log;
    return app.log.filter(
      (c) =>
        c.summary.toLowerCase().includes(f) ||
        c.author.toLowerCase().includes(f) ||
        c.oid.toLowerCase().includes(f),
    );
  });

  function refClass(kind: string) {
    if (kind === "tag") return "yellow";
    if (kind === "head") return "green";
    if (kind === "remote") return "purple";
    return "blue";
  }

  function load(commit: CommitInfo) {
    app.loadCommit(commit);
  }
</script>

<svelte:window onkeydown={filterKey} />

<div class="history">
  <header class="panel-head">
    <div class="titles">
      <span class="label view-title">History</span>
      <span class="label">{app.log.length} commits &middot; {app.branches.length} refs</span>
    </div>
    <div class="seg">
      <button
        class:active={mode === "commits"}
        onclick={() => (mode = "commits")}>Commits</button
      >
      <button class:active={mode === "graph"} onclick={() => (mode = "graph")}
        >Graph</button
      >
    </div>
  </header>

  {#if mode === "commits"}
    <div class="filter">
      <span class="filter-mark">/</span>
      <input
        class="filter-input"
        placeholder="Filter commits by message, author or hash…  ( / to focus)"
        bind:this={filterInput}
        bind:value={filter}
      />
      <span class="label">{commits.length} shown</span>
    </div>

    <div class="commits main-scrollbar">
      {#each commits as commit (commit.oid)}
        <div
          class="commit"
          class:selected={app.selectedCommit?.oid === commit.oid}
          role="button"
          tabindex="0"
          onclick={() => load(commit)}
          onkeydown={(e) => rowKey(e, commit)}
        >
          <span class="oid mono">{commit.short}</span>
          <span class="summary">{commit.summary || "(no message)"}</span>
          <span class="refs">
            {#each refsByOid.get(commit.oid) ?? [] as ref}
              <span class="badge {refClass(ref.kind)}">{ref.label}</span>
            {/each}
          </span>
          <span class="author label">{commit.author}</span>
          <span class="time label">{relativeTime(commit.time)}</span>
        </div>
      {:else}
        {#if filter.trim()}
          <div class="empty-state">
            <span class="empty">No commits match the filter</span>
            <button class="btn ghost sm" onclick={() => (filter = "")}
              >Clear filter</button
            >
          </div>
        {:else}
          <div class="empty-state">
            <span class="empty">No commits yet</span>
            {#if app.repo?.unborn}
              <button class="btn ghost sm" onclick={() => (app.view = "changes")}
                >Make your first commit</button
              >
            {/if}
          </div>
        {/if}
      {/each}
      {#if app.logHasMore}
        <button class="btn load-more" onclick={() => app.loadMoreLog()}>
          Load more commits
        </button>
      {/if}
    </div>
  {:else}
    <div class="graph-slot">
      <Graph />
    </div>
  {/if}
</div>

<style>
  .history {
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
  .filter {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-bottom: 1px solid var(--border-color);
    background: var(--background-color);
    flex: 0 0 auto;
  }
  .filter-mark {
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
  .commits {
    flex: 1;
    min-height: 0;
  }
  .commit {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 14px;
    border-bottom: 1px solid rgba(36, 39, 47, 0.7);
    cursor: pointer;
    transition:
      background 0.15s ease,
      box-shadow 0.15s ease;
  }
  .commit:hover {
    background: var(--hover-color);
  }
  .commit.selected {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  .oid {
    flex: 0 0 auto;
    color: var(--primary-color);
    font-size: 11px;
    letter-spacing: 0.04em;
  }
  .summary {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }
  .refs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 0 0 auto;
    max-width: 34%;
    overflow: hidden;
  }
  .author {
    flex: 0 0 auto;
    max-width: 110px;
    overflow: hidden;
    text-overflow: ellipsis;
    text-transform: none;
    letter-spacing: 0.02em;
    font-size: 10px;
  }
  .time {
    flex: 0 0 auto;
    width: 78px;
    text-align: right;
    text-transform: none;
    letter-spacing: 0.02em;
    font-size: 10px;
  }
  .load-more {
    display: block;
    margin: 12px auto;
  }
  .graph-slot {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
