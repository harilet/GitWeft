<script lang="ts">
  import { tick } from "svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { app } from "../store.svelte";
  import {
    buildResolved,
    defaultChoices,
    parseConflicts,
    type ConflictBlock,
    type ConflictChoice,
  } from "../conflicts";
  import {
    hasConflictMarkers,
    isAiConfigured,
    providerLabel,
    suggestConflictResolution,
  } from "../ai";
  import ConflictLines from "./ConflictLines.svelte";

  interface SearchMatch {
    pane: string;
    line: number;
  }

  const EDITOR_LINE_HEIGHT = 18.6;

  const conflict = $derived(app.conflict);
  const parsed = $derived(parseConflicts(conflict?.content ?? ""));

  let choices = $state<Record<number, ConflictChoice>>({});
  let manual = $state(false);
  let manualText = $state("");
  let showFullFile = $state(false);
  let saving = $state(false);
  let initializedFor = $state<{ path: string; content: string } | null>(null);

  let aiSuggesting = $state(false);
  let aiSuggestion = $state<string | null>(null);
  let aiError = $state<string | null>(null);
  let resultEditor = $state<HTMLTextAreaElement | null>(null);

  let query = $state("");
  let caseSensitive = $state(false);
  let matchIndex = $state(0);
  let searchInput = $state<HTMLInputElement | null>(null);
  let rootEl = $state<HTMLDivElement | null>(null);
  let gutterEl = $state<HTMLDivElement | null>(null);

  const aiSuggestionHasMarkers = $derived(
    aiSuggestion !== null && hasConflictMarkers(aiSuggestion),
  );

  const resolved = $derived(buildResolved(parsed.segments, choices));

  const queryText = $derived(query.trim());
  const manualLines = $derived(manualText.split("\n"));

  const matches = $derived.by(() => {
    const q = queryText;
    if (!q) return [] as SearchMatch[];
    const target = caseSensitive ? q : q.toLowerCase();
    const test = (line: string) =>
      (caseSensitive ? line : line.toLowerCase()).includes(target);
    const out: SearchMatch[] = [];
    for (const block of parsed.blocks) {
      const sides: [string, string[] | null][] = [
        ["ours", block.oursLines],
        ["base", block.baseLines],
        ["theirs", block.theirsLines],
      ];
      for (const [side, lines] of sides) {
        if (!lines) continue;
        for (let i = 0; i < lines.length; i++) {
          if (test(lines[i])) out.push({ pane: `b${block.id}-${side}`, line: i });
        }
      }
    }
    const full = (manual ? manualText : resolved).split("\n");
    for (let i = 0; i < full.length; i++) {
      if (test(full[i])) out.push({ pane: "full", line: i });
    }
    return out;
  });

  const activeMatch = $derived(
    matches.length > 0 && matchIndex < matches.length
      ? matches[matchIndex]
      : null,
  );
  const activePane = $derived(activeMatch?.pane ?? null);
  const activeLine = $derived(activeMatch?.line ?? null);
  const matchCountLabel = $derived(
    queryText
      ? matches.length > 0
        ? `${Math.min(matchIndex + 1, matches.length)}/${matches.length}`
        : "0"
      : "",
  );

  $effect(() => {
    const file = conflict;
    if (!file) {
      query = "";
      matchIndex = 0;
      return;
    }
    if (
      initializedFor?.path === file.path &&
      initializedFor.content === file.content
    )
      return;
    initializedFor = { path: file.path, content: file.content };
    choices = defaultChoices(parsed.blocks);
    const needsManual = !file.binary && parsed.blocks.length === 0;
    manual = needsManual;
    showFullFile = needsManual;
    manualText = file.content;
    query = "";
    matchIndex = 0;
  });

  function setChoice(id: number, choice: ConflictChoice) {
    choices = { ...choices, [id]: choice };
  }

  function choiceLabel(block: ConflictBlock): string {
    const choice = choices[block.id] ?? "ours";
    if (choice === "both") return "Both";
    if (choice === "theirs") return "Theirs";
    if (choice === "base") return "Base";
    return "Ours";
  }

  function blockResult(block: ConflictBlock): string[] {
    const choice = choices[block.id] ?? "ours";
    if (choice === "theirs") return block.theirsLines;
    if (choice === "base") return block.baseLines ?? [];
    if (choice === "both") {
      return [...block.oursLines, ...block.theirsLines];
    }
    return block.oursLines;
  }

  async function toggleManual() {
    if (!manual) {
      manualText = resolved;
      manual = true;
      showFullFile = true;
      await tick();
      resultEditor?.focus();
    } else {
      manual = false;
    }
  }

  async function suggestWithAi() {
    const file = app.conflict;
    if (!file || aiSuggesting) return;

    const config = app.aiConfig;
    if (config.disabled) return;
    if (!isAiConfigured(config)) {
      app.notify("Configure an AI provider in Settings → AI", "error");
      app.openAiSettings();
      return;
    }

    aiSuggesting = true;
    aiError = null;
    try {
      aiSuggestion = await suggestConflictResolution(config, {
        path: file.path,
        content: file.content,
        ours: file.ours,
        theirs: file.theirs,
        base: file.base,
        hasOurs: file.hasOurs,
        hasTheirs: file.hasTheirs,
        binary: file.binary,
      });
    } catch (e) {
      aiError = typeof e === "string" ? e : String(e);
    } finally {
      aiSuggesting = false;
    }
  }

  async function applySuggestion() {
    if (aiSuggestion === null) return;
    manualText = aiSuggestion;
    manual = true;
    showFullFile = true;
    await tick();
    resultEditor?.scrollIntoView({ block: "center", behavior: "smooth" });
    resultEditor?.focus();
    app.notify(
      "AI suggestion loaded into the editor. Review it, then save to resolve.",
      "success",
    );
  }

  async function takeSide(side: "ours" | "theirs" | "base") {
    const file = app.conflict;
    if (!file || saving) return;
    const deleting =
      (side === "ours" && !file.hasOurs) ||
      (side === "theirs" && !file.hasTheirs);
    const label =
      side === "ours" ? "ours" : side === "theirs" ? "theirs" : "the base";
    const prompt = deleting
      ? `Delete ${file.path}? This resolves the conflict by accepting the deletion.`
      : `Replace the whole file with ${label}? Other changes to this file will be lost.`;
    if (!(await confirm(prompt))) return;
    saving = true;
    await app.resolveConflict(file.path, side, null);
    saving = false;
  }

  async function save() {
    const file = app.conflict;
    if (!file || saving) return;
    saving = true;
    await app.resolveConflict(
      file.path,
      null,
      manual ? manualText : resolved,
    );
    saving = false;
  }

  async function abortMerge() {
    if (await confirm("Abort the merge and restore the pre-merge state?"))
      app.abortMerge();
  }

  function paneLabel(pane: string): string {
    if (pane === "full") return "whole file";
    const [block, side] = pane.split("-");
    return `conflict ${Number(block.slice(1)) + 1} · ${side}`;
  }

  function scrollToLine(pane: string, line: number) {
    rootEl
      ?.querySelector(`[data-pane="${pane}"][data-line="${line}"]`)
      ?.scrollIntoView({ block: "center", behavior: "smooth" });
  }

  function focusManualMatch(m: SearchMatch) {
    const el = resultEditor;
    if (!el) return;
    const lines = manualText.split("\n");
    let start = 0;
    for (let k = 0; k < m.line; k++) start += lines[k].length + 1;
    const line = lines[m.line] ?? "";
    const target = caseSensitive ? queryText : queryText.toLowerCase();
    const hay = caseSensitive ? line : line.toLowerCase();
    const rel = hay.indexOf(target);
    if (rel >= 0) el.setSelectionRange(start + rel, start + rel + target.length);
    el.focus({ preventScroll: true });
    el.scrollTop = Math.max(
      0,
      m.line * EDITOR_LINE_HEIGHT - el.clientHeight / 2,
    );
  }

  async function gotoMatch(delta: number) {
    if (matches.length === 0) return;
    matchIndex =
      ((matchIndex + delta) % matches.length + matches.length) % matches.length;
    const m = matches[matchIndex];
    if (m.pane === "full" && !showFullFile) {
      showFullFile = true;
      await tick();
    }
    if (m.pane === "full" && manual) focusManualMatch(m);
    else scrollToLine(m.pane, m.line);
  }

  function toggleCase() {
    caseSensitive = !caseSensitive;
    matchIndex = 0;
  }

  function syncGutter() {
    if (gutterEl && resultEditor) gutterEl.scrollTop = resultEditor.scrollTop;
  }

  function searchKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      gotoMatch(e.shiftKey ? -1 : 1);
    } else if (e.key === "Escape") {
      e.stopPropagation();
      if (query) {
        query = "";
        matchIndex = 0;
      } else {
        searchInput?.blur();
      }
    }
  }

  function onkey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && (e.key === "f" || e.key === "F")) {
      if (conflict && !conflict.binary) {
        e.preventDefault();
        searchInput?.focus();
        searchInput?.select();
      }
      return;
    }
    if (e.key === "Escape" && !saving) app.closeConflict();
  }
</script>

<svelte:window onkeydown={onkey} />

{#if conflict}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="overlay"
    role="presentation"
    onclick={() => !saving && app.closeConflict()}
    onkeydown={(e) => e.key === "Escape" && !saving && app.closeConflict()}
  >
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      bind:this={rootEl}
      onclick={(e) => e.stopPropagation()}
    >
      <header class="modal-head">
        <div class="head-titles">
          <span class="label">Resolve conflict</span>
          <h3>{conflict.path}</h3>
        </div>
        <button
          class="btn ghost icon"
          aria-label="Close"
          disabled={saving}
          onclick={() => app.closeConflict()}>✕</button
        >
      </header>

      <div class="modal-body main-scrollbar">
        {#if conflict.binary}
          <div class="binary">
            <span class="label">Binary file</span>
            <p>
              This file contains binary data, so it cannot be merged line by
              line. Choose which version to keep.
            </p>
            <div class="row">
              <button
                class="btn"
                class:danger-text={!conflict.hasOurs}
                disabled={saving}
                onclick={() => takeSide("ours")}
                >{conflict.hasOurs ? "Keep ours" : "Delete (ours)"}</button
              >
              <button
                class="btn"
                class:danger-text={!conflict.hasTheirs}
                disabled={saving}
                onclick={() => takeSide("theirs")}
                >{conflict.hasTheirs ? "Keep theirs" : "Delete (theirs)"}</button
              >
              {#if conflict.hasBase}
                <button
                  class="btn"
                  disabled={saving}
                  onclick={() => takeSide("base")}>Keep base</button
                >
              {/if}
            </div>
            <p class="hint">
              A side marked “delete” means that branch removed the file, so
              choosing it deletes the path.
            </p>
          </div>
        {:else}
          <div class="resolver-bar">
            <span class="label">
              {parsed.blocks.length}
              {parsed.blocks.length === 1 ? "conflict" : "conflicts"}
            </span>
            <span class="spacer"></span>
            <span class="label">Whole file</span>
            <button
              class="btn ghost sm"
              class:danger-text={!conflict.hasOurs}
              disabled={saving}
              onclick={() => takeSide("ours")}
              >{conflict.hasOurs ? "Take ours" : "Delete (ours)"}</button
            >
            <button
              class="btn ghost sm"
              class:danger-text={!conflict.hasTheirs}
              disabled={saving}
              onclick={() => takeSide("theirs")}
              >{conflict.hasTheirs ? "Take theirs" : "Delete (theirs)"}</button
            >
            {#if conflict.hasBase}
              <button
                class="btn ghost sm"
                disabled={saving}
                onclick={() => takeSide("base")}>Take base</button
              >
            {/if}
          </div>

          <div class="search-bar">
            <input
              class="input search-input"
              type="text"
              placeholder="Search file… (Ctrl+F)"
              spellcheck="false"
              title="Search across conflict sides and the whole file"
              bind:this={searchInput}
              bind:value={query}
              oninput={() => (matchIndex = 0)}
              onkeydown={searchKeydown}
            />
            <span class="search-count">{matchCountLabel}</span>
            <button
              class="btn ghost sm"
              title="Previous match (Shift+Enter)"
              disabled={matches.length === 0}
              onclick={() => gotoMatch(-1)}>↑</button
            >
            <button
              class="btn ghost sm"
              title="Next match (Enter)"
              disabled={matches.length === 0}
              onclick={() => gotoMatch(1)}>↓</button
            >
            <button
              class="btn ghost sm search-case"
              class:active={caseSensitive}
              title="Match case"
              onclick={toggleCase}>Aa</button
            >
            {#if activeMatch}
              <span class="search-hint">{paneLabel(activeMatch.pane)}</span>
            {/if}
          </div>

          {#if parsed.blocks.length === 0}
            <div class="note">
              No conflict markers were found in the working file. Edit it
              manually below, or take one side above.
            </div>
          {/if}

          {#if !conflict.hasOurs || !conflict.hasTheirs}
            <div class="note">
              One side deleted this file. Keep the remaining version below, or
              use a Delete action above to accept the deletion.
            </div>
          {/if}

          {#each parsed.blocks as block (block.id)}
            <article class="block">
              <div class="block-head">
                <span class="label"
                  >Conflict {block.id + 1} of {parsed.blocks.length}</span
                >
                <div class="seg">
                  <button
                    class:active={choices[block.id] === "ours"}
                    onclick={() => setChoice(block.id, "ours")}>Ours</button
                  >
                  {#if block.baseLines !== null}
                    <button
                      class:active={choices[block.id] === "base"}
                      onclick={() => setChoice(block.id, "base")}>Base</button
                    >
                  {/if}
                  <button
                    class:active={choices[block.id] === "theirs"}
                    onclick={() => setChoice(block.id, "theirs")}>Theirs</button
                  >
                  <button
                    class:active={choices[block.id] === "both"}
                    onclick={() => setChoice(block.id, "both")}>Both</button
                  >
                </div>
              </div>

              <div
                class="compare"
                class:with-base={block.baseLines !== null}
              >
                <div
                  class="side ours"
                  class:chosen={choices[block.id] === "ours" ||
                    choices[block.id] === "both"}
                >
                  <div class="side-head">
                    <span class="label">Ours</span>
                    <span class="side-label">{block.oursLabel}</span>
                  </div>
                  <ConflictLines
                    lines={block.oursLines}
                    startNo={block.oursStart}
                    paneKey={"b" + block.id + "-ours"}
                    {query}
                    {caseSensitive}
                    {activePane}
                    {activeLine}
                  />
                </div>

                {#if block.baseLines !== null}
                  <div
                    class="side base"
                    class:chosen={choices[block.id] === "base"}
                  >
                    <div class="side-head">
                      <span class="label">Base</span>
                      <span class="side-label">{block.baseLabel}</span>
                    </div>
                    <ConflictLines
                      lines={block.baseLines}
                      startNo={block.baseStart}
                      paneKey={"b" + block.id + "-base"}
                      {query}
                      {caseSensitive}
                      {activePane}
                      {activeLine}
                    />
                  </div>
                {/if}

                <div
                  class="side theirs"
                  class:chosen={choices[block.id] === "theirs" ||
                    choices[block.id] === "both"}
                >
                  <div class="side-head">
                    <span class="label">Theirs</span>
                    <span class="side-label">{block.theirsLabel}</span>
                  </div>
                  <ConflictLines
                    lines={block.theirsLines}
                    startNo={block.theirsStart}
                    paneKey={"b" + block.id + "-theirs"}
                    {query}
                    {caseSensitive}
                    {activePane}
                    {activeLine}
                  />
                </div>
              </div>

              <div class="block-result">
                <div class="block-result-head">
                  <span class="label">Result</span>
                  <span class="badge blue">{choiceLabel(block)}</span>
                </div>
                <ConflictLines
                  lines={blockResult(block)}
                  startNo={block.startLine}
                  paneKey={"b" + block.id + "-result"}
                  {query}
                  {caseSensitive}
                  {activePane}
                  {activeLine}
                />
              </div>
            </article>
          {/each}

          {#if !app.aiDisabled}
            <section class="ai-box">
              <div class="ai-head">
                <div class="ai-titles">
                  <span class="label">AI recommendation</span>
                  {#if app.aiConfigured}
                    <span class="ai-model"
                      >{providerLabel(app.aiConfig.provider)} ·
                      {app.aiConfig.model}</span
                    >
                  {/if}
                </div>
                <div class="row">
                  {#if aiSuggestion !== null}
                    <button
                      class="btn ghost sm"
                      disabled={aiSuggesting}
                      onclick={() => {
                        aiSuggestion = null;
                        aiError = null;
                      }}>Dismiss</button
                    >
                  {/if}
                  <button
                    class="btn ghost sm"
                    disabled={aiSuggesting}
                    onclick={suggestWithAi}
                  >
                    {aiSuggesting
                      ? "Asking the model…"
                      : aiSuggestion !== null
                        ? "Regenerate"
                        : "Suggest with AI"}
                  </button>
                </div>
              </div>

              {#if aiError}
                <div class="ai-error">{aiError}</div>
              {:else if aiSuggesting}
                <p class="ai-hint">
                  Waiting for the model. Nothing is changed until you accept the
                  suggestion and save.
                </p>
              {:else if aiSuggestion !== null}
                {#if aiSuggestionHasMarkers}
                  <div class="note">
                    The suggestion still contains conflict markers. Review it
                    carefully before using it.
                  </div>
                {/if}
                <pre class="ai-preview main-scrollbar">{aiSuggestion}</pre>
                <div class="ai-actions">
                  <button class="btn sm" onclick={applySuggestion}
                    >Apply to editor</button
                  >
                  <span class="ai-note">
                    The AI only recommends. Apply it, edit it if needed, then press
                    Save &amp; mark resolved.
                  </span>
                </div>
              {:else}
                <p class="ai-hint">
                  Ask the configured provider to recommend a merged file. The
                  assistant cannot write, stage or commit; you review the
                  suggestion and decide whether to use it.
                </p>
              {/if}
            </section>
          {/if}

          <section class="result">
            <div class="result-head">
              <div class="result-titles">
                <span class="label">Full merged file</span>
                <span class="result-meta"
                  >{manual ? "manual edit" : "built from choices"}</span
                >
              </div>
              <div class="row">
                <button
                  class="btn ghost sm"
                  onclick={() => (showFullFile = !showFullFile)}
                >
                  {showFullFile ? "Hide" : "Preview"}
                </button>
                <button class="btn ghost sm" onclick={toggleManual}>
                  {manual ? "Use choices" : "Edit manually"}
                </button>
              </div>
            </div>
            {#if showFullFile}
              {#if manual}
                <div class="editor-wrap">
                  <div class="gutter" bind:this={gutterEl} aria-hidden="true">
                    {#each manualLines as _, i (i)}<span>{i + 1}</span>{/each}
                  </div>
                  <textarea
                    class="textarea result-editor"
                    spellcheck="false"
                    bind:this={resultEditor}
                    bind:value={manualText}
                    onscroll={syncGutter}
                  ></textarea>
                </div>
              {:else}
                <ConflictLines
                  class="result-preview"
                  style="max-height: 420px"
                  lines={resolved.split("\n")}
                  startNo={1}
                  paneKey="full"
                  {query}
                  {caseSensitive}
                  {activePane}
                  {activeLine}
                />
              {/if}
            {/if}
          </section>
        {/if}
      </div>

      <footer class="modal-foot">
        {#if app.repo?.merging}
          <button
            class="btn ghost sm danger-text"
            disabled={saving}
            onclick={abortMerge}>Abort merge</button
          >
        {/if}
        <span class="spacer"></span>
        <button
          class="btn sm"
          disabled={saving}
          onclick={() => app.closeConflict()}>Cancel</button
        >
        {#if !conflict.binary}
          <button class="btn primary sm" disabled={saving} onclick={save}>
            {saving ? "Saving…" : "Save & mark resolved"}
          </button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(2, 3, 5, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 90;
  }
  .modal {
    width: min(1120px, 95vw);
    height: min(880px, 92vh);
    display: flex;
    flex-direction: column;
    background: var(--surface-color);
    border: 1px solid var(--border-strong);
    box-shadow: 0 40px 80px rgba(0, 0, 0, 0.6);
  }
  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-color);
    flex: 0 0 auto;
  }
  .head-titles {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  h3 {
    font-size: 15px;
    font-weight: 700;
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .modal-body {
    flex: 1;
    min-height: 0;
    padding: 16px 18px;
  }
  .modal-foot {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--border-color);
    flex: 0 0 auto;
  }

  .resolver-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    padding: 0 0 14px;
  }
  .search-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    padding: 0 0 14px;
  }
  .search-input {
    width: 240px;
    padding: 6px 10px;
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .search-count {
    min-width: 42px;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--muted-color);
  }
  .search-case.active {
    color: var(--text-color);
    border-color: var(--border-strong);
    background: var(--hover-color);
  }
  .search-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--muted-color);
    white-space: nowrap;
  }
  .note {
    padding: 10px 12px;
    margin-bottom: 14px;
    border: 1px solid #4a3a12;
    background: #2a2410;
    color: var(--yellow);
    font-size: 11px;
  }

  .block {
    border: 1px solid var(--border-color);
    background: var(--background-color);
    margin-bottom: 16px;
  }
  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 12px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border-color);
    flex-wrap: wrap;
  }

  .compare {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1px;
    background: var(--border-color);
  }
  .compare.with-base {
    grid-template-columns: 1fr 1fr 1fr;
  }
  .side {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--background-color);
  }
  .side-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 10px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border-color);
  }
  .side-label {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--muted-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .side.ours {
    box-shadow: inset 3px 0 0 var(--primary-color);
  }
  .side.theirs {
    box-shadow: inset 3px 0 0 var(--yellow);
  }
  .side.base {
    box-shadow: inset 3px 0 0 var(--muted-color);
  }
  .side.ours .side-head .label {
    color: var(--primary-color);
  }
  .side.theirs .side-head .label {
    color: var(--yellow);
  }
  .side.chosen {
    background: rgba(123, 140, 255, 0.08);
  }
  .side.theirs.chosen {
    background: rgba(209, 176, 107, 0.08);
  }

  .block-result {
    border-top: 1px solid var(--border-color);
    background: rgba(116, 201, 145, 0.06);
  }
  .block-result-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-color);
  }
  .block-result-head .label {
    color: var(--green);
  }

  .ai-box {
    border: 1px solid var(--border-color);
    background: var(--surface-2);
    margin-bottom: 16px;
  }
  .ai-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-color);
    flex-wrap: wrap;
  }
  .ai-titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .ai-model {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--muted-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ai-hint {
    margin: 0;
    padding: 12px;
    font-size: 11px;
    line-height: 1.6;
    color: var(--muted-color);
  }
  .ai-error {
    padding: 10px 12px;
    background: #3a1d20;
    border-bottom: 1px solid #5c2427;
    color: #ffd0cb;
    font-size: 11px;
    white-space: pre-wrap;
  }
  .ai-preview {
    margin: 0;
    padding: 12px;
    max-height: 260px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .ai-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-top: 1px solid var(--border-color);
    flex-wrap: wrap;
  }
  .ai-note {
    font-size: 10px;
    color: var(--muted-color);
  }

  .result {
    border: 1px solid var(--border-color);
    margin-top: 6px;
  }
  .result-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 12px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border-color);
  }
  .result-titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .result-meta {
    font-size: 10px;
    color: var(--muted-color);
  }
  .editor-wrap {
    display: flex;
    align-items: stretch;
    min-height: 240px;
    max-height: 420px;
    background: var(--background-color);
  }
  .gutter {
    flex: 0 0 auto;
    width: 48px;
    padding: 9px 8px 9px 0;
    overflow: hidden;
    text-align: right;
    background: var(--background-color);
    border-right: 1px solid var(--border-color);
    color: var(--muted-color);
    font-family: var(--font-mono);
    font-size: 10px;
    line-height: 18.6px;
    user-select: none;
  }
  .gutter span {
    display: block;
  }
  .result-editor {
    flex: 1 1 auto;
    min-width: 0;
    width: auto;
    height: 100%;
    min-height: 240px;
    max-height: 420px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 18.6px;
    white-space: pre;
    border: none;
    background: var(--background-color);
  }

  .binary {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border: 1px solid var(--border-color);
  }
  .binary p {
    margin: 0;
    font-size: 12px;
    color: var(--muted-color);
  }
  .binary .hint {
    font-size: 11px;
  }

  .danger-text {
    color: var(--red);
  }
  .danger-text:hover {
    color: #ffd0cb;
    border-color: var(--red);
  }
</style>
