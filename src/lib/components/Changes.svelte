<script lang="ts">
  import type { FileStatus } from "../api";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { app } from "../store.svelte";

  let selected = $state<string | null>(null);
  let committing = $state(false);

  async function submitCommit() {
    if (!app.commitMessage.trim() || app.staged.length === 0) return;
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

  function codeClass(c: string) {
    return `code-${c === "?" ? "\\?" : c}`;
  }
</script>

<div class="changes">
  <section class="list staged">
    <div class="panel-head">
      <span class="label">Staged &middot; {app.staged.length}</span>
      <button
        class="btn ghost sm"
        disabled={app.staged.length === 0}
        onclick={() => app.unstageAll()}>Unstage all</button
      >
    </div>
    <div class="files main-scrollbar">
      {#each app.staged as file (file.path)}
        <div
          class="file"
          class:selected={selected === file.path + ":staged"}
          role="button"
          tabindex="0"
          onclick={() => select(file, true)}
          onkeydown={(e) => e.key === "Enter" && select(file, true)}
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
              }}>−</button
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
      <button
        class="btn ghost sm"
        title="Generate a commit message with AI"
        onclick={() => app.openAi("commit")}>AI generate</button
      >
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
        disabled={committing || !app.commitMessage.trim() || app.staged.length === 0}
        onclick={submitCommit}
      >
        {committing ? "Committing…" : `Commit ${app.staged.length || ""}`}
      </button>
      <span class="spacer"></span>
      <button
        class="btn"
        disabled={app.unstaged.length === 0}
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
      <span class="label">Changes &middot; {app.unstaged.length}</span>
      <span class="label">Working tree</span>
    </div>
    <div class="files main-scrollbar">
      {#each app.unstaged as file (file.path)}
        <div
          class="file"
          class:selected={selected === file.path + ":work"}
          role="button"
          tabindex="0"
          onclick={() => select(file, false)}
          onkeydown={(e) => e.key === "Enter" && select(file, false)}
        >
          <span class="file-status-code {codeClass(file.worktree)}">{file.worktree}</span>
          <span class="path">{file.path}</span>
          <div class="actions">
            <button
              class="btn ghost sm act"
              title="Discard changes"
              onclick={async (e) => {
                e.stopPropagation();
                if (await confirm(`Discard changes in ${file.path}?`))
                  app.discard(file.path);
              }}>↺</button
            >
            <button
              class="btn ghost sm act"
              title="Stage"
              onclick={(e) => {
                e.stopPropagation();
                app.stage(file.path);
              }}>+</button
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
    border-left: 2px solid transparent;
    cursor: pointer;
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
  }
  .file:hover {
    background: var(--hover-color);
  }
  .file.selected {
    background: var(--surface-2);
    border-left-color: var(--primary-color);
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
    opacity: 0;
    font-size: 13px;
    line-height: 1;
    padding: 2px 7px;
  }
  .file:hover .act,
  .file.selected .act {
    opacity: 1;
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
