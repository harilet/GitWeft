<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "../store.svelte";
  import { autofocus } from "../actions";
  import {
    ACCENTS,
    DEFAULT_ACCENT,
    applyAccent,
    loadAccent,
    normalizeAccent,
    saveAccent,
  } from "../settings";
  import { errorsToText, formatTimestamp } from "../errorlog";
  import {
    AI_DEFAULTS,
    AI_MODEL_SUGGESTIONS,
    listAiModels,
    providerLabel,
    type AiConfig,
    type AiProvider,
  } from "../ai";

  let accent = $state(loadAccent());
  let copied = $state(false);

  let name = $state(app.user?.name ?? "");
  let email = $state(app.user?.email ?? "");
  let global = $state(false);
  let saving = $state(false);

  let aiScope = $state<"app" | "repo">("app");
  let override = $state(false);
  let showKey = $state(false);
  let aiForm = $state<AiConfig>({ ...app.aiSettings.app });
  let models = $state<string[]>([]);
  let modelsLoading = $state(false);
  let modelsError = $state<string | null>(null);
  let modelsKey = "";

  const hasRepo = $derived(!!app.repo);
  const appAi = $derived(app.aiSettings.app);
  const repoAi = $derived(app.repo ? app.aiSettings.repos[app.repo.path] : undefined);
  const modelOptions = $derived(
    Array.from(
      new Set([...models, ...AI_MODEL_SUGGESTIONS[aiForm.provider]]),
    ).sort((a, b) => a.localeCompare(b)),
  );

  async function refreshModels() {
    if (modelsLoading) return;
    modelsLoading = true;
    modelsError = null;
    try {
      models = await listAiModels({
        provider: aiForm.provider,
        baseUrl: aiForm.baseUrl,
        apiKey: aiForm.apiKey,
      });
    } catch (e) {
      models = [];
      modelsError = typeof e === "string" ? e : String(e);
    } finally {
      modelsLoading = false;
    }
  }

  function resetAiForm() {
    const source =
      aiScope === "repo" && app.repo && repoAi ? repoAi : appAi;
    aiForm = { ...source, temperature: source.temperature };
    override = aiScope === "repo" && !!repoAi;
    modelsError = null;
  }

  $effect(() => {
    if (app.settingsTab !== "ai") return;
    aiScope;
    app.repo?.path;
    appAi;
    repoAi;
    resetAiForm();
  });

  $effect(() => {
    if (app.settingsTab !== "ai") return;
    if (aiForm.provider !== "ollama") return;
    const key = `${aiForm.provider}|${aiScope}|${app.repo?.path ?? ""}`;
    if (key === modelsKey) return;
    modelsKey = key;
    void refreshModels();
  });

  function chooseAiScope(scope: "app" | "repo") {
    if (scope === "repo" && !hasRepo) return;
    aiScope = scope;
    resetAiForm();
  }

  function onProviderChange(e: Event) {
    const provider = (e.currentTarget as HTMLSelectElement).value as AiProvider;
    const previous = AI_DEFAULTS[aiForm.provider];
    aiForm = {
      ...aiForm,
      provider,
      baseUrl:
        aiForm.baseUrl === previous.baseUrl
          ? AI_DEFAULTS[provider].baseUrl
          : aiForm.baseUrl,
      model:
        aiForm.model === previous.model
          ? AI_DEFAULTS[provider].model
          : aiForm.model,
      apiKey: "",
    };
    showKey = false;
    models = [];
    modelsError = null;
  }

  function toggleOverride(e: Event) {
    const checked = (e.currentTarget as HTMLInputElement).checked;
    override = checked;
    if (aiScope !== "repo" || !app.repo) return;
    if (checked) {
      app.saveAiConfig("repo", { ...aiForm });
    } else {
      app.clearRepoAiConfig(app.repo.path);
      resetAiForm();
    }
  }

  function saveAi() {
    if (aiScope === "repo" && (!app.repo || !override)) return;
    const temperature = Math.min(
      2,
      Math.max(0, Number(aiForm.temperature) || 0),
    );
    const config: AiConfig = {
      ...aiForm,
      baseUrl: aiForm.baseUrl.trim(),
      model: aiForm.model.trim(),
      apiKey: aiForm.apiKey.trim(),
      temperature,
    };
    aiForm = config;
    app.saveAiConfig(aiScope, config);
    app.notify("AI settings saved", "success");
  }

  function resetAiDefaults() {
    aiForm = { ...AI_DEFAULTS[aiForm.provider] };
    showKey = false;
  }

  onMount(() => applyAccent(accent));

  function choose(value: string) {
    accent = saveAccent(value);
    applyAccent(accent);
  }

  function onColorInput(e: Event) {
    choose((e.currentTarget as HTMLInputElement).value);
  }

  function onHexChange(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const value = normalizeAccent(input.value);
    if (value) choose(value);
    else input.value = accent;
  }

  function resetAccent() {
    choose(DEFAULT_ACCENT);
  }

  async function copyErrors() {
    const text = errorsToText(app.errorLog);
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const area = document.createElement("textarea");
      area.value = text;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  async function saveIdentity() {
    if (!hasRepo) return;
    saving = true;
    await app.saveUserConfig(name, email, global);
    saving = false;
  }

  function close() {
    app.settingsOpen = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") close();
  }
</script>

<svelte:window onkeydown={onkey} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" onclick={close} role="presentation">
  <div
    class="settings"
    role="dialog"
    aria-modal="true"
    aria-label="Settings"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header class="settings-head">
      <div class="head-titles">
        <h3>Settings</h3>
      </div>
      <button class="btn ghost icon" onclick={close} aria-label="Close settings">✕</button>
    </header>

    <div class="settings-body">
      <nav class="settings-nav">
        <button
          class:active={app.settingsTab === "appearance"}
          onclick={() => (app.settingsTab = "appearance")}
        >
          Appearance
        </button>
        <button
          class:active={app.settingsTab === "identity"}
          onclick={() => (app.settingsTab = "identity")}
        >
          Git identity
        </button>
        <button
          class:active={app.settingsTab === "ai"}
          onclick={() => (app.settingsTab = "ai")}
        >
          AI assistant
        </button>
        <button
          class:active={app.settingsTab === "errors"}
          onclick={() => (app.settingsTab = "errors")}
        >
          Error log
          {#if app.errorLog.length}
            <span class="tab-count">{app.errorLog.length}</span>
          {/if}
        </button>
        <div class="spacer"></div>
        <div class="nav-note label">Saved on this device</div>
      </nav>

      <div class="settings-panel main-scrollbar">
        {#if app.settingsTab === "appearance"}
          <section class="section">
            <div class="section-head">
              <span class="label">Accent color</span>
              <span class="label current-accent">{accent}</span>
            </div>
            <p class="desc">
              Used for highlights, selection, links and the active navigation
              item across the app.
            </p>

            <div class="swatches">
              {#each ACCENTS as option (option.value)}
                <button
                  class="swatch"
                  class:active={accent === option.value}
                  title={option.name}
                  onclick={() => choose(option.value)}
                >
                  <span class="swatch-chip" style="background:{option.value}"></span>
                  <span class="swatch-name">{option.name}</span>
                </button>
              {/each}
            </div>

            <div class="field custom-field">
              <span class="label">Custom color</span>
              <div class="custom-row">
                <input
                  class="color-input"
                  type="color"
                  value={accent}
                  oninput={onColorInput}
                  aria-label="Pick accent color"
                />
                <input
                  class="input hex-input"
                  value={accent}
                  spellcheck="false"
                  maxlength="7"
                  onchange={onHexChange}
                  aria-label="Accent color hex value"
                />
              </div>
            </div>

            <div class="preview">
              <span class="label">Preview</span>
              <div class="preview-row">
                <button class="btn primary" tabindex="-1">Primary action</button>
                <button class="btn" tabindex="-1">Secondary</button>
                <button class="btn accent" tabindex="-1">Accent</button>
                <span class="badge blue">branch</span>
                <span class="accent-text">accent · {accent.slice(1)}</span>
              </div>
              <div class="preview-bar">
                <span></span>
              </div>
            </div>
          </section>
        {:else if app.settingsTab === "identity"}
          <section class="section">
            <div class="section-head">
              <span class="label">Git identity</span>
              <span class="label">Commit author</span>
            </div>

            {#if !hasRepo}
              <div class="notice">
                <span class="label">No repository open</span>
                <p class="desc">
                  Open a repository to view or edit its Git identity. Appearance
                  settings are available everywhere.
                </p>
              </div>
            {:else}
              <p class="desc">
                The name and email used as the author of commits in
                <span class="mono">{app.repo?.name}</span>.
              </p>

              <label class="field">
                <span>Name</span>
                <input
                  class="input"
                  placeholder="Jane Developer"
                  bind:value={name}
                  use:autofocus
                />
              </label>
              <label class="field">
                <span>Email</span>
                <input
                  class="input"
                  placeholder="jane@example.com"
                  bind:value={email}
                />
              </label>

              <label class="row scope">
                <input type="checkbox" bind:checked={global} />
                <span>Apply globally (all repositories) instead of this project</span>
              </label>

              <div class="location">
                {global
                  ? "Writing to ~/.gitconfig"
                  : `Writing to ${app.repo?.path ?? ""}/.git/config`}
              </div>
            {/if}
          </section>
        {:else if app.settingsTab === "ai"}
          <section class="section">
            <div class="section-head">
              <span class="label">AI assistant</span>
              <span class="label">Read-only tools</span>
            </div>
            <p class="desc">
              Chat with Ollama or OpenAI and let the assistant inspect commits,
              branches and working-tree diffs. It can never stage, unstage,
              commit or otherwise modify the repository. Repository data read by
              a tool is sent to the configured provider.
            </p>

            <div class="seg ai-scope">
              <button
                class:active={aiScope === "app"}
                onclick={() => chooseAiScope("app")}>App defaults</button
              >
              <button
                class:active={aiScope === "repo"}
                disabled={!hasRepo}
                onclick={() => chooseAiScope("repo")}>This repository</button
              >
            </div>

            {#if aiScope === "repo" && hasRepo}
              <label class="row scope">
                <input type="checkbox" checked={override} onchange={toggleOverride} />
                <span>Override app-level AI settings for {app.repo?.name}</span>
              </label>
              {#if !override}
                <div class="notice">
                  <span class="label">{appAi.disabled ? "AI disabled" : "Using app defaults"}</span>
                  {#if appAi.disabled}
                    <p class="desc">
                      The AI assistant is turned off by the app defaults. Enable
                      the override above and clear the disable option to use it
                      here.
                    </p>
                  {:else}
                    <p class="desc">
                      This repository uses {providerLabel(appAi.provider)} ·
                      {appAi.model}. Enable the override above to choose a
                      different provider, model or key here.
                    </p>
                  {/if}
                </div>
              {/if}
            {/if}

            {#if aiScope === "app" || override}
              <label class="row scope">
                <input type="checkbox" bind:checked={aiForm.disabled} />
                <span>
                  {aiScope === "app"
                    ? "Disable the AI assistant (applies to repositories without an override)"
                    : `Disable the AI assistant for ${app.repo?.name}`}
                </span>
              </label>
            {/if}

            <fieldset
              class="ai-fields"
              disabled={(aiScope === "repo" && !override) || aiForm.disabled}
            >
              <label class="field">
                <span>Provider</span>
                <select
                  class="input"
                  value={aiForm.provider}
                  onchange={onProviderChange}
                >
                  <option value="ollama">Ollama (local)</option>
                  <option value="openai">OpenAI (and compatible APIs)</option>
                </select>
              </label>

              <label class="field">
                <span>Base URL</span>
                <input
                  class="input"
                  bind:value={aiForm.baseUrl}
                  placeholder={AI_DEFAULTS[aiForm.provider].baseUrl}
                />
                <p class="field-hint">
                  {aiForm.provider === "ollama"
                    ? "Ollama server root, e.g. http://localhost:11434."
                    : "Include the API version, e.g. https://api.openai.com/v1."}
                </p>
              </label>

              <label class="field">
                <span>Model</span>
                <div class="model-row">
                  <input
                    class="input"
                    bind:value={aiForm.model}
                    list="ai-models"
                    placeholder={AI_DEFAULTS[aiForm.provider].model}
                  />
                  <button
                    class="btn"
                    type="button"
                    disabled={modelsLoading || !aiForm.baseUrl.trim()}
                    onclick={refreshModels}
                  >
                    {modelsLoading ? "Loading…" : "Refresh"}
                  </button>
                </div>
                <datalist id="ai-models">
                  {#each modelOptions as model (model)}
                    <option value={model}></option>
                  {/each}
                </datalist>
                {#if modelsError}
                  <p class="field-hint error-hint">{modelsError}</p>
                {:else if models.length}
                  <div class="model-chips">
                    {#each models as model (model)}
                      <button
                        class="model-chip"
                        class:active={aiForm.model === model}
                        type="button"
                        onclick={() => (aiForm.model = model)}
                      >
                        {model}
                      </button>
                    {/each}
                  </div>
                  <p class="field-hint">
                    {models.length}
                    {aiForm.provider === "ollama" ? "installed" : "available"} models.
                  </p>
                {:else}
                  <p class="field-hint">
                    {aiForm.provider === "ollama"
                      ? "Refresh to list models installed on the Ollama server."
                      : "Refresh to list models available to this API key."}
                  </p>
                {/if}
              </label>

              <label class="field">
                <span>API key</span>
                <div class="key-row">
                  <input
                    class="input"
                    type={showKey ? "text" : "password"}
                    bind:value={aiForm.apiKey}
                    placeholder={aiForm.provider === "ollama"
                      ? "Not required for local Ollama"
                      : "sk-…"}
                    autocomplete="off"
                    spellcheck="false"
                  />
                  <button
                    class="btn"
                    type="button"
                    onclick={() => (showKey = !showKey)}
                  >
                    {showKey ? "Hide" : "Show"}
                  </button>
                </div>
                <p class="field-hint">
                  Stored locally on this device with the rest of the AI
                  settings.
                </p>
              </label>

              <label class="field">
                <span>Temperature</span>
                <input
                  class="input"
                  type="number"
                  min="0"
                  max="2"
                  step="0.1"
                  bind:value={aiForm.temperature}
                />
                <p class="field-hint">
                  Lower values are more focused (0–2, default 0.2).
                </p>
              </label>
            </fieldset>

            <div class="location">
              The assistant runs with read-only tools: repository info, commit
              log and diffs, branches, staged changes and unstaged changes.
            </div>
          </section>
        {:else}
          <section class="section">
            <div class="section-head">
              <span class="label">Error log</span>
              <span class="label">{app.errorLog.length} saved</span>
            </div>
            <p class="desc">
              Errors shown as toasts are saved here on this device. They stay
              available after the toast disappears and across restarts.
            </p>

            {#if app.errorLog.length === 0}
              <div class="notice">
                <span class="label">No errors logged</span>
                <p class="desc">Anything that fails will appear here.</p>
              </div>
            {:else}
              <ul class="error-list main-scrollbar">
                {#each app.errorLog as entry (entry.id)}
                  <li class="error-entry">
                    <div class="error-meta">
                      <span class="error-time">{formatTimestamp(entry.time)}</span>
                      {#if entry.repo}
                        <span class="error-repo" title={entry.repo}>{entry.repo}</span>
                      {/if}
                    </div>
                    <pre class="error-message">{entry.message}</pre>
                  </li>
                {/each}
              </ul>
            {/if}
          </section>
        {/if}
      </div>
    </div>

    <footer class="settings-foot">
      {#if app.settingsTab === "appearance"}
        <button class="btn" onclick={resetAccent}>Reset to default</button>
        <span class="spacer"></span>
        <button class="btn primary" onclick={close}>Done</button>
      {:else if app.settingsTab === "errors"}
        <button
          class="btn"
          disabled={app.errorLog.length === 0}
          onclick={app.clearErrorLog}
        >
          Clear log
        </button>
        <button
          class="btn"
          disabled={app.errorLog.length === 0}
          onclick={copyErrors}
        >
          {copied ? "Copied" : "Copy all"}
        </button>
        <span class="spacer"></span>
        <button class="btn primary" onclick={close}>Close</button>
      {:else if app.settingsTab === "ai"}
        <button class="btn" onclick={resetAiDefaults}>Reset fields</button>
        <span class="spacer"></span>
        <button class="btn" onclick={close}>Close</button>
        <button
          class="btn primary"
          disabled={aiScope === "repo" && (!hasRepo || !override)}
          onclick={saveAi}
        >
          Save AI settings
        </button>
      {:else}
        <span class="spacer"></span>
        <button class="btn" onclick={close}>Close</button>
        <button
          class="btn primary"
          disabled={!hasRepo || saving}
          onclick={saveIdentity}
        >
          {saving ? "Saving…" : "Save identity"}
        </button>
      {/if}
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(2, 3, 5, 0.78);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 120;
  }
  .settings {
    display: flex;
    flex-direction: column;
    width: min(760px, 94vw);
    height: min(560px, 82vh);
    background: var(--surface-color);
    border: 1px solid var(--border-strong);
    box-shadow: 0 40px 80px rgba(0, 0, 0, 0.6);
  }

  .settings-head {
    display: flex;
    align-items: center;
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
  }
  h3 {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .settings-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .settings-nav {
    display: flex;
    flex-direction: column;
    width: 180px;
    flex: 0 0 auto;
    border-right: 1px solid var(--border-color);
    background: var(--background-color);
    overflow: hidden;
  }
  .settings-nav button {
    display: flex;
    align-items: center;
    padding: 11px 16px;
    border: none;
    border-left: 2px solid transparent;
    background: transparent;
    color: var(--muted-color);
    font-family: inherit;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    text-align: left;
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease;
  }
  .settings-nav button:hover {
    background: var(--hover-color);
    color: var(--text-color);
  }
  .settings-nav button.active {
    background: var(--hover-color);
    border-left-color: var(--primary-color);
    color: var(--text-color);
  }
  .nav-note {
    padding: 12px 16px;
    border-top: 1px solid var(--border-color);
    font-size: 9px;
    line-height: 1.6;
    white-space: normal;
  }
  .tab-count {
    margin-left: auto;
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    font-size: 9px;
    letter-spacing: 0.04em;
    color: var(--muted-color);
  }

  .error-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .error-entry {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    background: var(--background-color);
  }
  .error-meta {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--muted-color);
  }
  .error-time {
    flex: 0 0 auto;
  }
  .error-repo {
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    text-transform: none;
    letter-spacing: 0;
  }
  .error-message {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.6;
    color: #ffdcd7;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .settings-panel {
    flex: 1;
    min-width: 0;
    padding: 20px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .section-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--border-color);
  }
  .current-accent {
    color: var(--primary-color);
    text-transform: none;
    letter-spacing: 0.02em;
  }
  .desc {
    margin: 0;
    font-size: 12px;
    line-height: 1.7;
    color: var(--muted-color);
  }

  .swatches {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }
  .swatch {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 10px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--muted-color);
    font-family: inherit;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    cursor: pointer;
    transition:
      border-color 0.15s ease,
      color 0.15s ease,
      background 0.15s ease;
  }
  .swatch:hover {
    border-color: var(--border-strong);
    color: var(--text-color);
    background: var(--hover-color);
  }
  .swatch.active {
    border-color: var(--text-color);
    color: var(--text-color);
  }
  .swatch-chip {
    width: 12px;
    height: 12px;
    flex: 0 0 auto;
    border: 1px solid rgba(0, 0, 0, 0.4);
  }

  .custom-field {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
  }
  .custom-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .color-input {
    width: 42px;
    height: 38px;
    padding: 3px;
    border: 1px solid var(--border-strong);
    background: var(--background-color);
    cursor: pointer;
  }
  .color-input::-webkit-color-swatch-wrapper {
    padding: 0;
  }
  .color-input::-webkit-color-swatch {
    border: none;
  }
  .hex-input {
    width: 110px;
    flex: 0 0 auto;
    text-transform: lowercase;
  }

  .preview {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--border-color);
    background: var(--background-color);
  }
  .preview-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }
  .accent-text {
    color: var(--primary-color);
    font-size: 11px;
    letter-spacing: 0.04em;
  }
  .preview-bar {
    height: 3px;
    background: var(--border-color);
  }
  .preview-bar span {
    display: block;
    width: 58%;
    height: 100%;
    background: var(--primary-color);
  }

  .notice {
    padding: 14px;
    border: 1px solid var(--border-color);
    background: var(--background-color);
  }
  .notice .desc {
    margin-top: 8px;
  }

  .scope {
    align-items: flex-start;
    gap: 8px !important;
    font-size: 12px;
    color: var(--text-color);
  }
  .scope input {
    margin-top: 2px;
    accent-color: var(--primary-color);
  }
  .location {
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    background: var(--background-color);
    font-size: 10px;
    color: var(--muted-color);
    word-break: break-all;
    line-height: 1.6;
  }

  .ai-scope {
    align-self: flex-start;
  }
  .ai-fields {
    display: flex;
    flex-direction: column;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: none;
  }
  .ai-fields:disabled {
    opacity: 0.55;
  }
  .key-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .key-row .input {
    flex: 1;
    min-width: 0;
  }
  .model-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .model-row .input {
    flex: 1;
    min-width: 0;
  }
  .model-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    max-height: 130px;
    overflow: auto;
    margin-top: 8px;
    padding: 2px;
  }
  .model-chip {
    padding: 5px 8px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--muted-color);
    font-family: var(--font-mono);
    font-size: 10px;
    cursor: pointer;
    transition:
      border-color 0.15s ease,
      color 0.15s ease,
      background 0.15s ease;
  }
  .model-chip:hover {
    border-color: var(--border-strong);
    color: var(--text-color);
    background: var(--hover-color);
  }
  .model-chip.active {
    border-color: var(--primary-color);
    color: var(--primary-color);
    background: var(--hover-color);
  }
  .field-hint {
    margin: 6px 0 0;
    font-size: 10px;
    line-height: 1.6;
    color: var(--muted-color);
  }
  .field-hint.error-hint {
    color: var(--red);
  }

  .settings-foot {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 18px;
    border-top: 1px solid var(--border-color);
    flex: 0 0 auto;
  }
</style>
