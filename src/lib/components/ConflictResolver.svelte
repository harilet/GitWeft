<script lang="ts">
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { app } from "../store.svelte";
  import {
    buildResolved,
    defaultChoices,
    parseConflicts,
    type ConflictChoice,
  } from "../conflicts";

  const conflict = $derived(app.conflict);
  const parsed = $derived(parseConflicts(conflict?.content ?? ""));

  let choices = $state<Record<number, ConflictChoice>>({});
  let manual = $state(false);
  let manualText = $state("");
  let saving = $state(false);
  let initializedFor = $state<string | null>(null);

  const resolved = $derived(buildResolved(parsed.segments, choices));

  $effect(() => {
    const file = conflict;
    if (!file || initializedFor === file.path) return;
    initializedFor = file.path;
    choices = defaultChoices(parsed.blocks);
    manual = !file.binary && parsed.blocks.length === 0;
    manualText = file.content;
  });

  function setChoice(id: number, choice: ConflictChoice) {
    choices = { ...choices, [id]: choice };
  }

  function toggleManual() {
    if (!manual) {
      manualText = resolved;
      manual = true;
    } else {
      manual = false;
    }
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

  function onkey(e: KeyboardEvent) {
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
                  <pre>{block.oursLines.join("\n") || " "}</pre>
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
                    <pre>{block.baseLines.join("\n") || " "}</pre>
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
                  <pre>{block.theirsLines.join("\n") || " "}</pre>
                </div>
              </div>
            </article>
          {/each}

          <section class="result">
            <div class="result-head">
              <span class="label"
                >Merged result &middot; {manual
                  ? "manual edit"
                  : "built from choices"}</span
              >
              <button class="btn ghost sm" onclick={toggleManual}>
                {manual ? "Use choices" : "Edit manually"}
              </button>
            </div>
            {#if manual}
              <textarea
                class="textarea result-editor"
                spellcheck="false"
                bind:value={manualText}
              ></textarea>
            {:else}
              <pre class="result-preview main-scrollbar">{resolved}</pre>
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
    z-index: 150;
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
  .side pre {
    margin: 0;
    padding: 10px 12px;
    min-height: 44px;
    max-height: 240px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
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
  .result-preview {
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
  .result-editor {
    min-height: 240px;
    max-height: 420px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.55;
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
