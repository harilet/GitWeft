<script lang="ts">
  import { app } from "../store.svelte";
  import { autofocus } from "../actions";
  import Modal from "./Modal.svelte";

  let showCreate = $state(false);
  let tagName = $state("");
  let tagMessage = $state("");
  let tagTarget = $state("");

  async function create() {
    if (!tagName.trim()) return;
    const target = tagTarget.trim() || app.log[0]?.oid || null;
    const ok = await app.createTag(
      tagName.trim(),
      tagMessage.trim() || null,
      target,
    );
    if (ok) {
      showCreate = false;
      tagName = "";
      tagMessage = "";
      tagTarget = "";
    }
  }
</script>

<div class="tags">
  <header class="panel-head">
    <div class="titles">
      <span class="label view-title">Tags</span>
      <span class="label">{app.tags.length} total</span>
    </div>
    <button class="btn sm" onclick={() => (showCreate = true)}>New tag</button>
  </header>

  <div class="content main-scrollbar">
    {#each app.tags as tag (tag.name)}
      <div class="tag">
        <div class="top">
          <span class="mark">T</span>
          <span class="name mono">{tag.name}</span>
          <span class="badge {tag.annotated ? 'yellow' : ''}">
            {tag.annotated ? "annotated" : "lightweight"}
          </span>
          <span class="target mono">{tag.target.slice(0, 7)}</span>
          <span class="spacer"></span>
          <div class="actions">
            <button
              class="btn ghost sm danger-text"
              onclick={async () => {
                if (
                  await app.confirm({
                    title: `Delete tag "${tag.name}"?`,
                    details:
                      "The tag reference is removed from the repository. The commit it pointed to stays in history.",
                    confirmLabel: "Delete",
                    danger: true,
                  })
                )
                  app.deleteTag(tag.name);
              }}>Delete</button
            >
          </div>
        </div>
        {#if tag.message}
          <div class="msg">{tag.message}</div>
        {/if}
      </div>
    {:else}
      <div class="empty">No tags</div>
    {/each}
  </div>
</div>

{#if showCreate}
  <Modal title="Create tag" onclose={() => (showCreate = false)}>
    <label class="field">
      <span>Tag name</span>
      <input class="input" placeholder="v1.0.0" bind:value={tagName} use:autofocus />
    </label>
    <label class="field">
      <span>Message (optional — annotated tag)</span>
      <textarea class="textarea" bind:value={tagMessage}></textarea>
    </label>
    <label class="field">
      <span>Target commit (optional, default HEAD)</span>
      <input class="input mono" placeholder="SHA or ref" bind:value={tagTarget} />
    </label>
    {#snippet footer()}
      <button class="btn" onclick={() => (showCreate = false)}>Cancel</button>
      <button class="btn primary" disabled={!tagName.trim()} onclick={create}>
        Create
      </button>
    {/snippet}
  </Modal>
{/if}

<style>
  .tags {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .titles {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .content {
    flex: 1;
    min-height: 0;
  }
  .tag {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 14px;
    border-bottom: 1px solid rgba(36, 39, 47, 0.7);
    border-left: 2px solid transparent;
  }
  .tag:hover {
    background: var(--hover-color);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .mark {
    color: var(--primary-color);
    border: 1px solid var(--border-strong);
    font-size: 9px;
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .target {
    font-size: 10px;
    color: var(--muted-color);
  }
  .spacer {
    flex: 1;
  }
  .actions {
    flex: 0 0 auto;
  }


  .danger-text:hover {
    color: var(--red);
    border-color: var(--red);
  }
  .msg {
    padding-left: 26px;
    font-size: 11px;
    color: var(--muted-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
