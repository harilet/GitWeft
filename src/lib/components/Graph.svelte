<script lang="ts">
  import { app } from "../store.svelte";
  import type { BranchGraphNode } from "../api";

  const NODE_W = 220;
  const NODE_H = 52;
  const COL_GAP = 268;
  const ROW_GAP = 72;
  const PAD_X = 28;
  const PAD_Y = 60;

  interface RefAlias {
    name: string;
    tipShort: string;
  }
  interface GraphNode extends BranchGraphNode {
    aliases: RefAlias[];
  }
  interface PNode extends GraphNode {
    x: number;
    y: number;
  }
  interface PEdge {
    d: string;
    kind: "fork" | "merge";
    arrow?: boolean;
    group: string;
    nodes: string[];
    label: string;
  }

  type Focus = { kind: "edge" | "node"; id: string };

  let hovered = $state<Focus | null>(null);
  let pinned = $state<Focus | null>(null);
  const focus = $derived(hovered ?? pinned);

  const layout = $derived.by(() => {
    const g = app.graph;
    if (!g || g.nodes.length === 0) {
      return { nodes: [] as PNode[], edges: [] as PEdge[], width: 0, height: 0 };
    }

    // Remote-tracking refs are aliases of the local branches they track. Fold
    // them into the matching local node instead of drawing a misleading
    // "merged into" loop for every remote ref with the same tip.
    const local = g.nodes.filter((n) => !n.isRemote);
    const remote = g.nodes.filter((n) => n.isRemote);
    const nodes: GraphNode[] = local.map((n) => ({
      ...n,
      mergesInto: [...n.mergesInto],
      aliases: [],
    }));
    const localByName = new Map(nodes.map((n) => [n.name, n]));
    const nameMap = new Map<string, string>();
    for (const n of nodes) nameMap.set(n.name, n.name);

    for (const n of remote) {
      const short = n.name.includes("/")
        ? n.name.slice(n.name.indexOf("/") + 1)
        : n.name;
      const host = localByName.get(short);
      if (host) {
        host.aliases.push({ name: n.name, tipShort: n.tipShort });
        nameMap.set(n.name, host.name);
      } else {
        nodes.push({ ...n, mergesInto: [...n.mergesInto], aliases: [] });
        nameMap.set(n.name, n.name);
      }
    }

    // Fold parent / merge targets that now live on a remote alias back onto
    // their local branch so no edge is lost when a remote ref is collapsed.
    for (const n of nodes) {
      if (n.parent) {
        const mapped = nameMap.get(n.parent) ?? n.parent;
        n.parent = mapped === n.name ? null : mapped;
      }
      n.mergesInto = [
        ...new Set(n.mergesInto.map((m) => nameMap.get(m) ?? m)),
      ].filter((m) => m !== n.name);
    }

    const localNodes = nodes.filter((n) => !n.isRemote);

    // Branches that point at the same commit were merged without creating a
    // merge commit (fast-forward). Show that as a real branch relationship:
    // branch off the base branch and merge back into it.
    const byTip = new Map<string, GraphNode[]>();
    for (const n of localNodes) {
      const list = byTip.get(n.tip) ?? [];
      list.push(n);
      byTip.set(n.tip, list);
    }
    const baseFor = (list: GraphNode[]) =>
      list.find(
        (n) => !n.isRemote && (n.name === "master" || n.name === "main"),
      ) ??
      list.find((n) => n.isHead) ??
      list.find((n) => !n.isRemote) ??
      list[0];

    for (const list of byTip.values()) {
      if (list.length < 2) continue;
      const base = baseFor(list);
      for (const n of list) {
        if (n === base) continue;
        if (!n.parent && base.parent !== n.name) n.parent = base.name;
        if (!n.mergesInto.includes(base.name)) n.mergesInto.push(base.name);
      }
    }

    // Once the base branch advances past a fast-forwarded branch, the backend
    // reports master as a linear descendant of the feature branch. Anchor the
    // graph on master/main instead of making it look like the base branch was
    // forked from the feature branch.
    const anchor =
      nodes.find(
        (n) => !n.isRemote && (n.name === "master" || n.name === "main"),
      ) ?? nodes.find((n) => n.isHead);
    if (anchor?.parent) {
      const parent = nodes.find((n) => n.name === anchor.parent);
      if (parent && !parent.isRemote && parent !== anchor && !parent.parent) {
        parent.parent = anchor.name;
        anchor.parent = null;
        const merged = parent.mergesInto.indexOf(anchor.name);
        if (merged >= 0) parent.mergesInto.splice(merged, 1);
      }
    }

    const byName = new Map(nodes.map((n) => [n.name, n]));
    const children = new Map<string, string[]>();
    for (const n of nodes) {
      if (n.parent && byName.has(n.parent)) {
        const arr = children.get(n.parent) ?? [];
        arr.push(n.name);
        children.set(n.parent, arr);
      }
    }
    const sortNames = (a: string, b: string) => {
      const na = byName.get(a)!;
      const nb = byName.get(b)!;
      if (na.isRemote !== nb.isRemote) return na.isRemote ? 1 : -1;
      return a.localeCompare(b);
    };
    for (const arr of children.values()) arr.sort(sortNames);

    const visited = new Set<string>();
    let order = 0;
    const pos = new Map<string, { x: number; y: number }>();
    const walk = (name: string, depth: number) => {
      if (visited.has(name)) return;
      visited.add(name);
      pos.set(name, { x: PAD_X + depth * COL_GAP, y: PAD_Y + order * ROW_GAP });
      order++;
      for (const c of children.get(name) ?? []) walk(c, depth + 1);
    };

    const roots = nodes
      .filter((n) => !n.parent || !byName.has(n.parent))
      .map((n) => n.name)
      .sort(sortNames);
    for (const r of roots) walk(r, 0);
    for (const n of nodes) if (!visited.has(n.name)) walk(n.name, 0);

    const placed: PNode[] = nodes.map((n) => {
      const p = pos.get(n.name)!;
      return { ...n, x: p.x, y: p.y };
    });

    const edges: PEdge[] = [];
    const center = (n: PNode) => ({ x: n.x + NODE_W / 2, y: n.y + NODE_H / 2 });
    const forkPath = (x1: number, y1: number, x2: number, y2: number) => {
      if (Math.abs(y2 - y1) < 1) return `M ${x1} ${y1} L ${x2} ${y2}`;
      const busX = Math.max(x1 + 14, x2 - 24);
      const dir = y2 > y1 ? 1 : -1;
      const r = Math.min(12, Math.abs(busX - x1) / 2, Math.abs(y2 - y1) / 2);
      return `M ${x1} ${y1} L ${busX - r} ${y1} Q ${busX} ${y1} ${busX} ${y1 + dir * r} L ${busX} ${y2 - dir * r} Q ${busX} ${y2} ${busX + r} ${y2} L ${x2} ${y2}`;
    };
    for (const n of placed) {
      if (n.parent && byName.has(n.parent)) {
        const p = placed.find((m) => m.name === n.parent)!;
        edges.push({
          kind: "fork",
          group: `fork:${p.name}->${n.name}`,
          nodes: [p.name, n.name],
          label: `${n.name} branched from ${p.name}`,
          d: forkPath(p.x + NODE_W, center(p).y, n.x, center(n).y),
        });
      }
    }

    // Merges: a lone source gets a curved arrow; several sources merging into
    // the same target share a dashed rail instead of overlapping curves.
    const mergeSources = new Map<string, PNode[]>();
    for (const n of placed) {
      for (const m of n.mergesInto) {
        if (!byName.has(m) || m === n.name) continue;
        const list = mergeSources.get(m) ?? [];
        list.push(n);
        mergeSources.set(m, list);
      }
    }

    let extraRight = 0;
    for (const [targetName, sources] of mergeSources) {
      const target = placed.find((n) => n.name === targetName)!;
      const ty = center(target).y;

      if (sources.length === 1) {
        const n = sources[0];
        const sx = center(n).x;
        const sy = n.y;
        const tx = center(target).x;
        const lift = Math.max(30, Math.abs(tx - sx) / 4);
        edges.push({
          kind: "merge",
          arrow: true,
          group: `merge:${n.name}->${targetName}`,
          nodes: [n.name, targetName],
          label: `${n.name} merged into ${targetName}`,
          d: `M ${sx} ${sy} C ${sx} ${sy - lift}, ${tx} ${ty - lift}, ${tx} ${ty}`,
        });
        continue;
      }

      const left = Math.min(...sources.map((s) => s.x));
      const right = Math.max(...sources.map((s) => s.x + NODE_W));
      const targetRight = target.x + NODE_W;
      let railX: number;
      let sourceEdge: (s: PNode) => number;
      let targetEdge: number;

      if (targetRight < left) {
        railX = right + 16;
        sourceEdge = (s) => s.x + NODE_W;
        targetEdge = targetRight;
        extraRight = Math.max(extraRight, railX + 16);
      } else if (target.x > right) {
        railX = left - 16;
        sourceEdge = (s) => s.x;
        targetEdge = target.x;
      } else {
        for (const n of sources) {
          const sx = center(n).x;
          const sy = n.y;
          const tx = center(target).x;
          const lift = Math.max(30, Math.abs(tx - sx) / 4);
          edges.push({
            kind: "merge",
            arrow: true,
            group: `merge:${n.name}->${targetName}`,
            nodes: [n.name, targetName],
            label: `${n.name} merged into ${targetName}`,
            d: `M ${sx} ${sy} C ${sx} ${sy - lift}, ${tx} ${ty - lift}, ${tx} ${ty}`,
          });
        }
        continue;
      }

      const group = `merge-rail:${targetName}`;
      const label = `${sources.length} branches merged into ${targetName}`;
      const ys = sources.map((s) => center(s).y);
      const top = Math.min(ty, ...ys);
      const bottom = Math.max(ty, ...ys);
      edges.push({
        kind: "merge",
        arrow: false,
        group,
        nodes: [targetName],
        label,
        d: `M ${railX} ${top} L ${railX} ${bottom}`,
      });
      for (const s of sources) {
        const sy = center(s).y;
        edges.push({
          kind: "merge",
          arrow: false,
          group,
          nodes: [s.name, targetName],
          label,
          d: `M ${sourceEdge(s)} ${sy} L ${railX} ${sy}`,
        });
      }
      edges.push({
        kind: "merge",
        arrow: true,
        group,
        nodes: [targetName],
        label,
        d: `M ${railX} ${ty} L ${targetEdge} ${ty}`,
      });
    }

    const width =
      Math.max(Math.max(...placed.map((n) => n.x + NODE_W), 0), extraRight) +
      PAD_X;
    const height = order * ROW_GAP + NODE_H + PAD_Y;
    return { nodes: placed, edges, width, height };
  });

  const stats = $derived.by(() => {
    const refs = app.graph?.nodes.length ?? 0;
    const tips = new Set((app.graph?.nodes ?? []).map((n) => n.tip)).size;
    const merges = layout.nodes.reduce((sum, n) => sum + n.mergesInto.length, 0);
    return {
      refs,
      tips,
      merges,
      label: `${refs} ${refs === 1 ? "ref" : "refs"} · ${tips} ${
        tips === 1 ? "tip" : "tips"
      } · ${merges} ${merges === 1 ? "merge" : "merges"}`,
    };
  });

  function aliasText(n: GraphNode): string {
    return n.aliases
      .map((a) =>
        a.tipShort === n.tipShort ? a.name : `${a.name} ${a.tipShort}`,
      )
      .join(" · ");
  }

  const edgeGroups = $derived.by(() => {
    const map = new Map<
      string,
      { group: string; label: string; edges: PEdge[]; nodes: string[] }
    >();
    for (const e of layout.edges) {
      let group = map.get(e.group);
      if (!group) {
        group = { group: e.group, label: e.label, edges: [], nodes: [] };
        map.set(e.group, group);
      }
      group.edges.push(e);
      for (const name of e.nodes) {
        if (!group.nodes.includes(name)) group.nodes.push(name);
      }
    }
    return [...map.values()];
  });

  const focusEdges = $derived.by(() => {
    const f = focus;
    if (!f) return null;
    if (f.kind === "edge") return new Set([f.id]);
    const ids = new Set<string>();
    for (const e of layout.edges) {
      if (e.nodes.includes(f.id)) ids.add(e.group);
    }
    return ids;
  });

  const focusNodes = $derived.by(() => {
    const f = focus;
    if (!f) return null;
    const names = new Set<string>();
    if (f.kind === "node") names.add(f.id);
    for (const e of layout.edges) {
      if (focusEdges?.has(e.group)) {
        for (const name of e.nodes) names.add(name);
      }
    }
    return names;
  });

  const focusLabel = $derived.by(() => {
    const f = focus;
    if (!f) return "";
    if (f.kind === "node") {
      const node = layout.nodes.find((n) => n.name === f.id);
      return node ? `${node.name} · ${node.tipShort}` : "";
    }
    return layout.edges.find((e) => e.group === f.id)?.label ?? "";
  });

  const isPinned = $derived(
    !!pinned &&
      !!focus &&
      pinned.kind === focus.kind &&
      pinned.id === focus.id,
  );

  function togglePin(target: Focus) {
    pinned =
      pinned?.kind === target.kind && pinned.id === target.id ? null : target;
  }

  function clearFocus() {
    hovered = null;
    pinned = null;
  }

  function changeRemote(include: boolean) {
    clearFocus();
    app.setGraphRemote(include);
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape" && (hovered || pinned)) clearFocus();
  }}
/>

<div class="graph-view">
  <div class="head">
    <div class="legend">
      <span class="key"
        ><svg width="26" height="8"
          ><line x1="0" y1="4" x2="26" y2="4" class="fork-line" /></svg
        > branched from</span
      >
      <span class="key"
        ><svg width="26" height="8"
          ><line x1="0" y1="4" x2="26" y2="4" class="merge-line" /></svg
        > merged into</span
      >
      <span class="hint">hover to trace · click to pin</span>
    </div>
    <div class="head-right">
      <span class="label">{stats.label}</span>
      <label class="toggle label">
        <input
          type="checkbox"
          checked={app.graphIncludeRemote}
          onchange={(e) => changeRemote(e.currentTarget.checked)}
        />
        remote
      </label>
    </div>
  </div>

  <div class="canvas main-scrollbar">
    {#if layout.nodes.length === 0}
      <div class="empty">No branches to display</div>
    {:else}
      <div class="stage" style="width:{layout.width}px; height:{layout.height}px">
        <svg
          class="edges"
          width={layout.width}
          height={layout.height}
          viewBox="0 0 {layout.width} {layout.height}"
        >
          <defs>
            <marker
              id="arrow-merge"
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="7"
              markerHeight="7"
              orient="auto-start-reverse"
            >
              <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--purple)" />
            </marker>
          </defs>
          {#each edgeGroups as g (g.group)}
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
            <g
              class="edge-group"
              class:active={focusEdges?.has(g.group)}
              class:dimmed={!!focusEdges && !focusEdges.has(g.group)}
              role="button"
              tabindex="-1"
              aria-label={g.label}
              onmouseenter={() => (hovered = { kind: "edge", id: g.group })}
              onmouseleave={() => (hovered = null)}
              onclick={(e) => {
                e.stopPropagation();
                togglePin({ kind: "edge", id: g.group });
              }}
              onkeydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  togglePin({ kind: "edge", id: g.group });
                }
              }}
            >
              {#each g.edges as e}
                <path d={e.d} class="edge-hit" />
              {/each}
              {#each g.edges as e}
                {#if e.kind === "merge"}
                  {#if e.arrow === false}
                    <path d={e.d} class="edge merge" />
                  {:else}
                    <path
                      d={e.d}
                      class="edge merge"
                      marker-end="url(#arrow-merge)"
                    />
                  {/if}
                {:else}
                  <path d={e.d} class="edge fork" />
                {/if}
              {/each}
            </g>
          {/each}
        </svg>

        {#each layout.nodes as n (n.name)}
          <div
            class="node"
            class:head={n.isHead}
            class:remote={n.isRemote}
            class:active={focusNodes?.has(n.name)}
            class:dimmed={!!focusNodes && !focusNodes.has(n.name)}
            style="left:{n.x}px; top:{n.y}px; width:{NODE_W}px; height:{NODE_H}px"
            role="button"
            tabindex="0"
            aria-label="{n.name}, {n.isHead ? 'current branch' : 'branch'}"
            title="{n.name} · {n.tipShort}{n.aliases.length
              ? ` · ${aliasText(n)}`
              : ''}{n.parent
              ? ` · from ${n.parent}`
              : ''}{n.mergesInto.length
              ? ` · merged into ${n.mergesInto.join(', ')}`
              : ''}"
            onmouseenter={() => (hovered = { kind: "node", id: n.name })}
            onmouseleave={() => (hovered = null)}
            onclick={(e) => {
              e.stopPropagation();
              togglePin({ kind: "node", id: n.name });
            }}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                togglePin({ kind: "node", id: n.name });
              }
            }}
          >
            <div class="node-row">
              <span class="node-icon">{n.isRemote ? "R" : "B"}</span>
              <span class="node-name">{n.name}</span>
              <span class="node-tip mono">{n.tipShort}</span>
              {#if n.isHead}<span class="chip green">HEAD</span>{/if}
            </div>
            {#if n.aliases.length > 0}
              <div class="node-extra" title={aliasText(n)}>+ {aliasText(n)}</div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  {#if focusLabel}
    <div class="trace-bar">
      <span class="label">{focus?.kind === "edge" ? "Trace" : "Branch"}</span>
      <span class="trace-text">{focusLabel}</span>
      <span class="spacer"></span>
      {#if isPinned}<span class="badge blue">pinned</span>{/if}
      <button class="btn ghost sm" onclick={clearFocus}>Clear</button>
    </div>
  {/if}
</div>

<style>
  .graph-view {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 14px;
    background: var(--background-color);
    border-bottom: 1px solid var(--border-color);
    flex: 0 0 auto;
    flex-wrap: wrap;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 16px;
    font-size: 10px;
    color: var(--muted-color);
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }
  .key {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .hint {
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--muted-color);
    opacity: 0.7;
  }
  .head-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .fork-line {
    stroke: var(--primary-color);
    stroke-width: 2;
  }
  .merge-line {
    stroke: var(--purple);
    stroke-width: 2;
    stroke-dasharray: 4 3;
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    cursor: pointer;
  }
  .toggle input {
    accent-color: var(--primary-color);
    margin: 0;
  }
  .canvas {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background:
      radial-gradient(circle at 1px 1px, rgba(124, 130, 142, 0.16) 1px, transparent 0)
        0 0 / 22px 22px;
  }
  .empty {
    padding: 40px 16px;
    text-align: center;
  }
  .stage {
    position: relative;
  }
  .edges {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .edge-group {
    transition: opacity 0.12s ease;
  }
  .edge-group.dimmed .edge {
    opacity: 0.12;
  }
  .edge-group.active .edge {
    stroke-width: 3;
  }
  .edge-hit {
    fill: none;
    stroke: transparent;
    stroke-width: 14;
    pointer-events: stroke;
    cursor: pointer;
  }
  .edge {
    fill: none;
    stroke-width: 2;
    pointer-events: none;
    transition:
      opacity 0.12s ease,
      stroke-width 0.12s ease;
  }
  .edge.fork {
    stroke: var(--primary-color);
  }
  .edge.merge {
    stroke: var(--purple);
    stroke-dasharray: 5 4;
  }
  .node {
    position: absolute;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    padding: 0 8px;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    border-radius: 0;
    font-size: 11px;
    z-index: 1;
    cursor: pointer;
    transition:
      opacity 0.12s ease,
      border-color 0.12s ease;
  }
  .node:hover {
    border-color: var(--primary-color);
  }
  .node.dimmed {
    opacity: 0.25;
  }
  .node.active {
    outline: 1px solid var(--primary-color);
    outline-offset: 2px;
  }
  .node.head {
    border-color: var(--green);
  }
  .node.head:hover {
    border-color: var(--green);
  }
  .node.remote {
    border-style: dashed;
  }
  .node-row {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }
  .node-icon {
    color: var(--muted-color);
    font-size: 10px;
    border: 1px solid var(--border-strong);
    width: 15px;
    height: 15px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
  }
  .node-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .node-tip {
    font-size: 10px;
    color: var(--primary-color);
    flex: 0 0 auto;
  }
  .node-extra {
    padding-left: 22px;
    font-size: 10px;
    line-height: 1.3;
    color: var(--muted-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chip {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    padding: 1px 4px;
    border: 1px solid var(--border-color);
    color: var(--muted-color);
    flex: 0 0 auto;
  }
  .chip.green {
    color: var(--green);
    border-color: #25452f;
    background: #12261a;
  }
  .trace-bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    background: var(--surface-color);
    border-top: 1px solid var(--border-color);
    box-shadow: 0 -12px 24px rgba(0, 0, 0, 0.35);
  }
  .trace-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    color: var(--text-color);
  }
</style>
