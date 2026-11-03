<script lang="ts">
  import { onMount, tick } from "svelte";
  import { app } from "../store.svelte";
  import { providerLabel } from "../ai";

  let listEl = $state<HTMLDivElement | null>(null);
  let inputEl = $state<HTMLTextAreaElement | null>(null);

  const title = $derived(
    app.aiMode === "commit" ? "Commit message assistant" : "Repository assistant",
  );

  onMount(() => {
    if (app.aiMode === "commit" && app.aiEntries.length === 0 && app.aiConfigured) {
      void app.sendAi();
      return;
    }
    inputEl?.focus();
  });

  $effect(() => {
    // Keep the newest message in view as the conversation grows.
    app.aiEntries.length;
    app.aiStatus;
    if (!listEl) return;
    void tick().then(() => {
      if (listEl) listEl.scrollTop = listEl.scrollHeight;
    });
  });

  function close() {
    app.closeAi();
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape" && !app.settingsOpen) close();
  }

  function submit() {
    if (!app.aiDraft.trim() || app.aiRunning) return;
    void app.sendAi();
  }

  function onComposerKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" onclick={close} role="presentation">
  <div
    class="ai-panel"
    role="dialog"
    aria-modal="true"
    aria-label="AI assistant"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header class="ai-head">
      <div class="head-titles">
        <span class="label">GitWeft AI</span>
        <h3>{title}</h3>
        <span class="provider">
          {providerLabel(app.aiConfig.provider)} · {app.aiConfig.model || "no model"}
        </span>
      </div>
      <div class="head-actions">
        <button
          class="btn ghost sm"
          title="AI settings"
          onclick={() => app.openAiSettings()}>Settings</button
        >
        <button
          class="btn ghost sm"
          title="Start a new conversation"
          disabled={app.aiRunning || app.aiEntries.length === 0}
          onclick={() => app.resetAi()}>New</button
        >
        <button class="btn ghost icon" aria-label="Close assistant" onclick={close}>✕</button>
      </div>
    </header>

    {#if !app.aiConfigured}
      <div class="config-notice">
        <span class="label">AI not configured</span>
        <p>
          Choose Ollama or OpenAI and set a model in Settings → AI. The assistant
          can read commits and working-tree diffs, but it can never write to your
          repository.
        </p>
        <button class="btn primary sm" onclick={() => app.openAiSettings()}>
          Open AI settings
        </button>
      </div>
    {/if}

    <div class="messages main-scrollbar" bind:this={listEl}>
      {#if app.aiEntries.length === 0 && !app.aiRunning}
        <div class="empty-state">
          <span class="label">Read-only</span>
          <p>
            Ask about recent commits, branches or uncommitted changes. The
            assistant inspects the repository with tools and never stages,
            commits or modifies anything.
          </p>
          {#if app.aiMode === "commit"}
            <p class="hint">
              I’ll inspect the staged diff and draft a commit message for you to
              review.
            </p>
          {/if}
        </div>
      {/if}

      {#each app.aiEntries as entry (entry.id)}
        <article class="msg {entry.role}">
          <div class="msg-head">
            <span class="label">{entry.role === "user" ? "You" : "Assistant"}</span>
            {#if entry.role === "assistant" && app.aiMode === "commit"}
              <button
                class="btn ghost sm"
                onclick={() => app.useAiCommitMessage(entry.content)}
              >
                Use as commit message
              </button>
            {/if}
          </div>
          <div class="msg-body">{entry.content}</div>
        </article>
      {/each}

      {#if app.aiStatus}
        <div class="tool-status">
          <span class="spinner"></span>
          <span class="label">{app.aiStatus}</span>
        </div>
      {/if}
    </div>

    <div class="composer">
      <textarea
        class="textarea"
        bind:this={inputEl}
        bind:value={app.aiDraft}
        placeholder={app.aiMode === "commit"
          ? "Ask for a commit message…"
          : "Ask about this repository…"}
        disabled={!app.aiConfigured}
        onkeydown={onComposerKey}
      ></textarea>
      <div class="composer-row">
        <span class="hint label">Enter to send · Shift + Enter for a new line</span>
        <button
          class="btn primary"
          disabled={!app.aiConfigured || !app.aiDraft.trim() || app.aiRunning}
          onclick={submit}
        >
          {app.aiRunning ? "Working…" : "Send"}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(2, 3, 5, 0.6);
    display: flex;
    justify-content: flex-end;
    z-index: 110;
  }
  .ai-panel {
    display: flex;
    flex-direction: column;
    width: min(520px, 96vw);
    height: 100%;
    background: var(--surface-color);
    border-left: 1px solid var(--border-strong);
    box-shadow: -30px 0 80px rgba(0, 0, 0, 0.55);
  }

  .ai-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 18px;
    border-bottom: 1px solid var(--border-color);
    flex: 0 0 auto;
  }
  .head-titles {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  h3 {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .provider {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--muted-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 0 0 auto;
  }

  .config-notice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 9px;
    margin: 14px 18px 0;
    padding: 12px 14px;
    border: 1px solid #4a3a12;
    background: #2a2410;
  }
  .config-notice p {
    margin: 0;
    font-size: 12px;
    line-height: 1.6;
    color: var(--muted-color);
  }

  .messages {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px;
  }
  .empty-state {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 18px;
    border: 1px dashed var(--border-strong);
    background: var(--background-color);
  }
  .empty-state p {
    margin: 0;
    font-size: 12px;
    line-height: 1.7;
    color: var(--muted-color);
  }
  .empty-state .hint {
    color: var(--text-color);
  }

  .msg {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
    border: 1px solid var(--border-color);
    background: var(--background-color);
  }
  .msg.user {
    border-left: 2px solid var(--primary-color);
  }
  .msg.assistant {
    border-left: 2px solid var(--border-strong);
  }
  .msg-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .msg-body {
    font-size: 12px;
    line-height: 1.7;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--font-mono);
  }

  .tool-status {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 4px 2px;
  }
  .spinner {
    width: 11px;
    height: 11px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--primary-color);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex: 0 0 auto;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .composer {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 18px;
    border-top: 1px solid var(--border-color);
    flex: 0 0 auto;
  }
  .composer .textarea {
    min-height: 76px;
  }
  .composer-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .hint {
    text-transform: none;
    letter-spacing: 0.02em;
    font-size: 9px;
    white-space: normal;
  }
</style>
