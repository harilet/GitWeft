<script lang="ts">
  interface Segment {
    text: string;
    hit: boolean;
  }

  let {
    lines,
    startNo = 1,
    paneKey,
    query = "",
    caseSensitive = false,
    activePane = null,
    activeLine = null,
    style = "",
    class: className = "",
  }: {
    lines: string[];
    startNo?: number;
    paneKey: string;
    query?: string;
    caseSensitive?: boolean;
    activePane?: string | null;
    activeLine?: number | null;
    style?: string;
    class?: string;
  } = $props();

  const needle = $derived(query.trim());

  function segments(line: string): Segment[] {
    const q = needle;
    if (!q) return [{ text: line, hit: false }];
    const hay = caseSensitive ? line : line.toLowerCase();
    const target = caseSensitive ? q : q.toLowerCase();
    const out: Segment[] = [];
    let from = 0;
    let idx = hay.indexOf(target);
    while (idx !== -1) {
      if (idx > from) out.push({ text: line.slice(from, idx), hit: false });
      out.push({ text: line.slice(idx, idx + target.length), hit: true });
      from = idx + target.length;
      idx = hay.indexOf(target, from);
    }
    if (from < line.length) out.push({ text: line.slice(from), hit: false });
    return out;
  }
</script>

<div class="lines {className}" {style}>
  {#each lines as line, i (i)}
    {@const segs = segments(line)}
    <div
      class="ln"
      class:hit={segs.some((s) => s.hit)}
      class:active={activePane === paneKey && activeLine === i}
      data-pane={paneKey}
      data-line={i}
    >
      <span class="no">{startNo + i}</span>
      <span class="code">
        {#each segs as seg, si (si)}
          {#if seg.hit}<span class="match">{seg.text}</span>{:else}{seg.text}{/if}
        {/each}
      </span>
    </div>
  {:else}
    <div class="ln"><span class="no"></span><span class="code"> </span></div>
  {/each}
</div>

<style>
  .lines {
    margin: 0;
    padding: 10px 12px;
    min-height: 44px;
    max-height: 240px;
    overflow: auto;
    scrollbar-color: var(--border-strong) transparent;
    scrollbar-width: thin;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.55;
  }
  .ln {
    display: flex;
    align-items: flex-start;
    min-height: 1.55em;
  }
  .ln.hit {
    background: rgba(209, 176, 107, 0.07);
  }
  .ln.active {
    background: rgba(209, 176, 107, 0.16);
  }
  .no {
    flex: 0 0 auto;
    width: 5ch;
    margin-right: 10px;
    text-align: right;
    color: var(--muted-color);
    font-size: 10px;
    line-height: 1.9;
    opacity: 0.75;
    user-select: none;
  }
  .code {
    flex: 1 1 auto;
    min-width: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .match {
    background: rgba(209, 176, 107, 0.3);
    color: #ffe4b3;
  }
  .ln.active .match {
    background: var(--yellow);
    color: #14161c;
  }
</style>
