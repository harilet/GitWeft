<script lang="ts">
  import { app } from "../store.svelte";

  let message = $state("");

  async function create() {
    const ok = await app.createStash(message.trim() || null);
    if (ok) message = "";
  }
</script>

<div class="stashes">
  <header class="panel-head">
    <div class="titles">
      <span class="label">Stashes</span>
      <span class="label">{app.stashes.length} saved</span>
    </div>
  </header>

  <div class="create">
    <span class="prompt">$</span>
    <input
      class="create-input"
      placeholder="Stash message (optional)"
      bind:value={message}
      onkeydown={(e) => e.key === "Enter" && create()}
    />
    <button class="btn sm" onclick={create}>Stash changes</button>
  </div>

  <div class="content main-scrollbar">
    {#each app.stashes as stash (stash.oid)}
      <div class="stash">
        <span class="index">#{String(stash.index).padStart(2, "0")}</span>
        <span class="msg">{stash.message || "(no message)"}</span>
        <span class="spacer"></span>
        <div class="actions">
          <button class="btn ghost sm" onclick={() => app.applyStash(stash.index)}>
            Apply</button
          >
          <button class="btn ghost sm" onclick={() => app.popStash(stash.index)}>
            Pop</button
          >
          <button
            class="btn ghost sm danger-text"
            onclick={async () => {
              if (
                await app.confirm({
                  title: `Drop stash #${String(stash.index).padStart(2, "0")}?`,
                  details:
                    "The stash entry is permanently deleted and can no longer be applied. This cannot be undone.",
                  confirmLabel: "Drop",
                  danger: true,
                })
              )
                app.dropStash(stash.index);
            }}>Drop</button
          >
        </div>
      </div>
    {:else}
      <div class="empty">No stashes</div>
    {/each}
  </div>
</div>

<style>
  .stashes {
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
  .create {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-bottom: 1px solid var(--border-color);
    background: var(--background-color);
    flex: 0 0 auto;
  }
  .prompt {
    color: var(--primary-color);
    font-size: 12px;
  }
  .create-input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-color);
    font-family: inherit;
    font-size: 12px;
  }
  .create-input::placeholder {
    color: var(--muted-color);
  }
  .content {
    flex: 1;
    min-height: 0;
  }
  .stash {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid rgba(36, 39, 47, 0.7);
    border-left: 2px solid transparent;
  }
  .stash:hover {
    background: var(--hover-color);
    border-left-color: var(--purple);
  }
  .index {
    color: var(--purple);
    font-size: 11px;
    flex: 0 0 auto;
  }
  .msg {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }
  .spacer {
    flex: 1;
  }
  .actions {
    display: flex;
    gap: 2px;
    flex: 0 0 auto;
  }
  .danger-text {
    color: var(--muted-color);
  }
  .danger-text:hover {
    color: var(--red);
    border-color: var(--red);
  }
</style>
