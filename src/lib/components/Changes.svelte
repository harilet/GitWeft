<script lang="ts">
  import type { FileStatus } from "../api";
  import { app } from "../store.svelte";

  let selected = $state<string | null>(null);
  let committing = $state(false);

  const conflicts = $derived(app.conflicts);
  const hasConflicts = $derived(conflicts.length > 0);
  const staged = $derived(app.staged.filter((f) => !isConflict(f)));
  const unstaged = $derived(app.unstaged.filter((f) => !isConflict(f)));

  function isConflict(file: FileStatus) {
    return file.index === "U" || file.worktree === "U";
  }

  async function submitCommit() {
    if (!app.commitMessage.trim() || staged.length === 0 || hasConflicts) return;
    committing = true;
    const ok = await app.commit(app.commitMessage);
    committing = false;
    if (ok) {
      app.commitMessage = "";
      selected = null;
    }
  }

  function select(file: FileStatus, staged: boolean) {
    selected = file.path + (staged ? ":staged" : ":work");
    app.loadWorkingDiff(file.path, staged);
  }

  /** Enter and Space both activate selection-style rows. */
  function rowKey(e: KeyboardEvent, action: () => void) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      action();
    }
  }

  function codeClass(c: string) {
    return `code-${c === "?" ? "\\?" : c}`;
  }

  async function abortMerge() {
    if (
      await app.confirm({
        title: "Abort merge?",
        details:
          "Aborts the merge and restores the pre-merge state. Files already resolved in this merge are restored to their pre-merge content.",
        confirmLabel: "Abort merge",
        danger: true,
      })
    )
      app.abortMerge();
  }

  async function discard(file: FileStatus) {
    const untracked = file.worktree === "?";
    const ok = await app.confirm({
      title: "Discard changes?",
      message: file.path,
      details: untracked
        ? "This file is untracked — discarding deletes it from disk. It cannot be restored from Git."
        : "All working-tree changes in this file are reset to the last commit and cannot be recovered.",
      confirmLabel: "Discard",
      danger: true,
    });
    if (ok) app.discard(file.path);
  }
</script>

<div class="changes">
  {#if app.repo?.merging}
    <div class="merge-banner" class:ready={!hasConflicts}>
      <div class="merge-text">
        <span class="label"
          >{hasConflicts ? "Merge in progress" : "Merge ready"}</span
        >
        <span class="merge-hint">
          {hasConflicts
            ? `${conflicts.length} conflicted file${conflicts.length === 1 ? "" : "s"} — resolve them, then commit.`
            : "All conflicts are resolved. Commit to finish the merge."}
        </span>
      </div>
      {#if hasConflicts}
        <button class="btn ghost sm danger-text" onclick={abortMerge}
          >Abort merge</button
        >
      {/if}
    </div>
  {/if}

  {#if hasConflicts}
    <section class="list conflicts">
      <div class="panel-head">
        <span class="label">Conflicts &middot; {conflicts.length}</span>
        <span class="label">Resolve before committing</span>
      </div>
      <div class="files main-scrollbar">
        {#each conflicts as file (file.path)}
          <div
            class="file conflict"
            role="button"
            tabindex="0"
            onclick={() => app.openConflict(file.path)}
            onkeydown={(e) => rowKey(e, () => app.openConflict(file.path))}
          >
            <span class="file-status-code code-U">!</span>
            <span class="path">{file.path}</span>
            <div class="actions">
              <button
                class="btn ghost sm act"
                title="Open the conflict resolver"
                onclick={(e) => {
                  e.stopPropagation();
                  app.openConflict(file.path);
                }}>Resolve</button
              >
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <section class="list staged">
    <div class="panel-head">
      <span class="label">Staged &middot; {staged.length}</span>
      <button
        class="btn ghost sm"
        disabled={staged.length === 0}
        onclick={() => app.unstageAll()}>Unstage all</button
      >
    </div>
    <div class="files main-scrollbar">
      {#each staged as file (file.path)}
        <div
          class="file"
          class:selected={selected === file.path + ":staged"}
          role="button"
          tabindex="0"
          onclick={() => select(file, true)}
          onkeydown={(e) => rowKey(e, () => select(file, true))}
        >
          <span class="file-status-code {codeClass(file.index)}">{file.index}</span>
          <span class="path">{file.path}</span>
          <div class="actions">
            <button
              class="btn ghost sm act"
              title="Unstage"
              onclick={(e) => {
                e.stopPropagation();
                app.unstage(file.path);
              }}>Unstage</button
            >
          </div>
        </div>
      {:else}
        <div class="empty">Nothing staged</div>
      {/each}
    </div>
  </section>

  <section class="commit-box">
    <div class="commit-head">
      <span class="label commit-label">Commit message</span>
      {#if !app.aiDisabled}
        <button
          class="btn ghost sm"
          title="Generate a commit message with AI"
          onclick={() => app.openAi("commit")}>AI generate</button
        >
      {/if}
    </div>
    <textarea
      class="textarea"
      placeholder="Describe your changes…"
      bind:value={app.commitMessage}
      onkeydown={(e) => {
        if ((e.ctrlKey || e.metaKey) && e.key === "Enter") submitCommit();
      }}
    ></textarea>
    <div class="row">
      <button
        class="btn primary"
        disabled={committing || !app.commitMessage.trim() || staged.length === 0 || hasConflicts}
        onclick={submitCommit}
      >
        {committing ? "Committing…" : staged.length ? `Commit ${staged.length}` : "Commit"}
      </button>
      <span class="spacer"></span>
      <button
        class="btn accent"
        disabled={unstaged.length === 0 || hasConflicts}
        title={hasConflicts ? "Resolve all conflicts before staging everything" : ""}
        onclick={() => app.stageAll()}>Stage all</button
      >
    </div>
    {#if app.user}
      <div class="hint label">
        {app.user.name || "unset"} &lt;{app.user.email || "unset"}&gt;
      </div>
    {/if}
  </section>

  <section class="list unstaged">
    <div class="panel-head">
      <span class="label">Changes &middot; {unstaged.length}</span>
      <span class="label">Working tree</span>
    </div>
    <div class="files main-scrollbar">
      {#each unstaged as file (file.path)}
        <div
          class="file"
          class:selected={selected === file.path + ":work"}
          role="button"
          tabindex="0"
          onclick={() => select(file, false)}
          onkeydown={(e) => rowKey(e, () => select(file, false))}
        >
          <span class="file-status-code {codeClass(file.worktree)}">{file.worktree}</span>
          <span class="path">{file.path}</span>
          <div class="actions">
            <button
              class="btn ghost sm act"
              title="Discard changes"
              onclick={(e) => {
                e.stopPropagation();
                discard(file);
              }}>Discard</button
            >
            <button
              class="btn ghost sm act"
              title="Stage"
              onclick={(e) => {
                e.stopPropagation();
                app.stage(file.path);
              }}>Stage</button
            >
          </div>
        </div>
      {:else}
        <div class="empty">Working tree clean</div>
      {/each}
    </div>
  </section>
</div>

<style>
  .changes {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .list {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1 1 0;
    overflow: hidden;
  }
  .list.staged {
    max-height: 34%;
  }
  .list.conflicts {
    flex: 0 1 auto;
    max-height: 30%;
  }
  .merge-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    background: #2a2410;
    border-bottom: 1px solid #4a3a12;
    flex: 0 0 auto;
  }
  .merge-banner.ready {
    background: #12261a;
    border-bottom-color: #25452f;
  }
  .merge-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    flex: 1;
  }
  .merge-banner .label {
    color: var(--yellow);
  }
  .merge-banner.ready .label {
    color: var(--green);
  }
  .merge-hint {
    font-size: 11px;
    color: var(--text-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file.conflict {
    background: color-mix(in srgb, var(--yellow) 6%, transparent);
  }
  .file.conflict:hover {
    background: #2a2410;
  }
  .files {
    flex: 1;
    min-height: 0;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 14px;
    border-bottom: 1px solid rgba(36, 39, 47, 0.7);
    cursor: pointer;
    transition:
      background 0.15s ease,
      box-shadow 0.15s ease;
  }
  .file:hover {
    background: var(--hover-color);
  }
  .file.selected {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .actions {
    display: flex;
    gap: 2px;
    flex: 0 0 auto;
  }
  .act {
    font-size: 10px;
    line-height: 1;
    padding: 3px 8px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }
  .commit-box {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    background: var(--surface-color);
    border-top: 1px solid var(--border-color);
    border-bottom: 1px solid var(--border-color);
    flex: 0 0 auto;
  }
  .commit-label {
    display: block;
  }
  .commit-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .commit-box .textarea {
    min-height: 68px;
    padding: 10px 12px;
  }
  .hint {
    text-transform: none;
    letter-spacing: 0.02em;
    font-size: 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
