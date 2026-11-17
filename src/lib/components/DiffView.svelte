<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import { parsePatch, type DiffLine } from "../format";

  let { patch, empty }: { patch: string; empty?: string } = $props();

  const files = $derived(parsePatch(patch));

  let collapsedFiles = new SvelteSet<string>();
  let lastPatch = $state<string | null>(null);

  // A new patch is a new context: fresh collapse state per commit.
  $effect(() => {
    if (patch !== lastPatch) {
      lastPatch = patch;
      collapsedFiles.clear();
    }
  });

  function fileKey(file: (typeof files)[number], fi: number): string {
    return file.header ?? `#${fi}`;
  }

  function toggleFile(key: string) {
    if (collapsedFiles.has(key)) collapsedFiles.delete(key);
    else collapsedFiles.add(key);
  }

  function stats(file: (typeof files)[number]): { add: number; del: number } {
    let add = 0;
    let del = 0;
    for (const line of file.lines) {
      if (line.type === "add") add++;
      else if (line.type === "del") del++;
    }
    return { add, del };
  }

  function marker(line: DiffLine): string {
    if (line.type === "add") return "+";
    if (line.type === "del") return "-";
    return " ";
  }
</script>

<div class="diff main-scrollbar">
  {#if files.length === 0}
    <div class="empty">{empty ?? "No changes to display"}</div>
  {:else}
    <div class="inner">
      {#each files as file, fi (fileKey(file, fi))}
        {@const key = fileKey(file, fi)}
        {@const isCollapsed = collapsedFiles.has(key)}
        {@const s = stats(file)}
        {#if file.header}
          <button
            class="file-header"
            class:collapsed-header={isCollapsed}
            aria-expanded={!isCollapsed}
            title={isCollapsed ? "Show this file" : "Hide this file"}
            onclick={() => toggleFile(key)}
          >
            <span class="chevron" aria-hidden="true"
              >{isCollapsed ? "▸" : "▾"}</span
            >
            <span class="tag">F</span>
            <span class="path">{file.header}</span>
            <span class="spacer"></span>
            <span class="stat add-stat">+{s.add}</span>
            <span class="stat del-stat">−{s.del}</span>
          </button>
        {/if}
        {#if !isCollapsed || !file.header}
          {#each file.lines as line, li (li)}
            {#if line.type === "hunk"}
              <div class="row hunk">
                <span class="tag">H</span>
                <span class="code">{line.content}</span>
              </div>
            {:else if line.type === "meta"}
              <div class="row meta">
                <span class="code">{line.content}</span>
              </div>
            {:else if line.type === "none"}
              <div class="row none"></div>
            {:else}
              <div class="row {line.type}">
                <span class="marker">{marker(line)}</span>
                <span class="num old">{line.oldNo ?? ""}</span>
                <span class="num new">{line.newNo ?? ""}</span>
                <span class="code">{line.content}</span>
              </div>
            {/if}
          {/each}
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .diff {
    height: 100%;
    background: var(--background-color);
  }
  .empty {
    padding: 40px 20px;
    text-align: center;
  }
  .inner {
    display: inline-block;
    min-width: 100%;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
  }
  .row {
    display: flex;
    align-items: stretch;
    min-height: 1.6em;
  }
  .row .code {
    flex: 1;
    white-space: pre;
    padding: 1px 16px 1px 4px;
  }
  .marker {
    flex: 0 0 auto;
    width: 18px;
    text-align: center;
    user-select: none;
  }
  .num {
    flex: 0 0 auto;
    width: 44px;
    text-align: right;
    padding-right: 10px;
    color: var(--muted-color);
    user-select: none;
  }
  .num.old {
    border-left: 1px solid var(--border-color);
  }

  .add {
    background: #12261a;
    color: #c3f0cd;
  }
  .add .marker,
  .add .num {
    background: #1a4d2c;
    color: #7ee2a8;
  }
  .del {
    background: #3a1d20;
    color: #ffd0cb;
  }
  .del .marker,
  .del .num {
    background: #5c2427;
    color: #ff9c95;
  }
  .context .marker,
  .context .num {
    background: rgba(36, 39, 47, 0.5);
  }

  .hunk {
    align-items: center;
    gap: 8px;
    padding: 2px 12px 2px 6px;
    background: color-mix(in srgb, var(--primary-color) 10%, transparent);
    color: var(--primary-color);
  }
  .meta {
    padding: 0 16px 0 24px;
    color: var(--muted-color);
  }
  .none {
    height: 1.6em;
  }

  .file-header {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 12px;
    background: var(--surface-2);
    border: none;
    border-top: 1px solid var(--border-color);
    border-bottom: 1px solid var(--border-color);
    color: var(--muted-color);
    font-family: var(--font-mono);
    font-size: 11px;
    text-align: start;
    cursor: pointer;
  }
  .file-header:hover .path {
    color: var(--text-color);
  }
  .file-header:hover .chevron,
  .file-header:hover .stat {
    color: inherit;
  }
  .chevron {
    color: var(--muted-color);
    font-size: 10px;
    width: 10px;
    flex: 0 0 auto;
    user-select: none;
  }
  .stat {
    font-size: 10px;
    flex: 0 0 auto;
    user-select: none;
  }
  .add-stat {
    color: var(--green);
  }
  .del-stat {
    color: var(--red);
  }
  .file-header .spacer {
    flex: 1;
    min-width: 0;
  }
  .file-header .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    flex: 0 0 auto;
    border: 1px solid var(--border-strong);
    background: transparent;
    color: var(--primary-color);
    font-size: 10px;
    font-weight: 700;
  }
</style>
