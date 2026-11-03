# SCRATCH — working memory

Append dated entries, keep the top current, prune what is obvious from code.

## Current focus

AI assistant shipped: read-only chat/tool loop over Ollama and OpenAI, app-level
and per-repository provider settings, and commit-message generation from Changes.
Next session should smoke-test against a real Ollama server and an OpenAI key, and
review the tool loop with a non-tool-capable model.

## Next up

- Manual provider smoke test: Ollama (`llama3.2` or another tool-capable model) and
  OpenAI (`gpt-4o-mini`), including a generated commit message.
- Add a model-list refresh if provider model names become hard to type.
- Pick the next feature from `docs/FEATURES.md` Ideas.

## Known issues / watch-outs

- AI provider settings (including API keys) live in `localStorage` under
  `gitweft.ai`; they are not encrypted.
- AI requests are non-streaming and time out after 180 seconds; tool progress is
  shown as status text, not tokens.
- Models must support tool calling. A model that cannot call tools will answer
  from the conversation without inspecting the repository.
- The tool loop stops after 8 rounds and truncates individual tool results at
  60,000 characters.
- `reqwest` is built with `native-tls`, so the existing `libssl-dev` prerequisite
  is required. `git2` remains built without TLS/SSH.
- Error log lives in `localStorage` (`gitweft.errors`, max 200) and covers toast
  errors plus uncaught `error`/`unhandledrejection`; it does not write a file.
- Version mismatch: `package.json` is `0.1.0`; `src-tauri/tauri.conf.json` is
  `0.1.2`. Check before packaging.
- No CI workflow. Validation is manual: `npm run check`, `npm run build`,
  `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`.
- No frontend lint/format script and no frontend tests; backend has 15 unit tests.
- Fetch/pull/push/clone require system `git` and configured credentials.
- Root `FEATURES.md` and `README.md` predate this flow; `docs/FEATURES.md` is the
  canonical living scope.

## Decisions

- 2026-11-03 — AI is read-only by construction: the tool set only reads repository
  info, commits, branches, status and diffs. No write command is exposed to the
  model, regardless of provider.
- 2026-11-03 — AI HTTP calls run in Rust (`reqwest`) instead of the webview. Why:
  avoid CORS differences, keep Ollama/OpenAI message normalization in one place
  and keep API keys out of browser fetch. Provider config still lives in
  `localStorage` like the other client preferences.
- 2026-11-03 — AI settings support app defaults plus a full per-repository
  override keyed by repository path. Why: users may want a local model for one
  project and a hosted model elsewhere without duplicating settings.
- 2026-11-03 — The commit-message draft moved from `Changes.svelte` into the
  repo session so the AI panel can insert a generated message and drafts survive
  view switches.
- 2026-11-03 — Session memory: add `AGENTS.md` (map), `docs/FEATURES.md` (scope)
  and `docs/SCRATCH.md` (rolling notes), read in that order. Why: new sessions
  should start grounded without rereading the whole repo.
- 2026-11-03 — Scope source: keep `docs/FEATURES.md` canonical; leave root
  `FEATURES.md` as the older detailed reference. Why: avoid churn outside the
  requested doc flow while giving agents one current source.

## Session log

### 2026-11-03 — AI assistant with read-only Git tools

- Added `src-tauri/src/ai.rs`: `ai_chat` and `ai_list_models` commands, Ollama
  `/api/chat` + `/api/tags` and OpenAI `/chat/completions` + `/models` bridges,
  message/tool-call normalization, optional bearer auth and focused unit tests.
- Added `src/lib/ai.ts`: provider defaults/validation, app + repo persistence,
  model listing, read-only tool schemas, tool executor and the bounded agent loop.
- Added `AiPanel.svelte`: chat drawer, per-call tool rows with running/done/failed
  state and collapsed argument/result details, provider notice, generated
  commit-message insert, auto-start for commit mode.
- Extended `AppStore`/`RepoSession`: per-repo AI conversations and drafts, AI
  settings save/clear, commit draft moved into the session.
- Settings → AI: provider/model/base URL/API key/temperature with app/repo scope
  and per-repository override. Installed Ollama models are fetched from
  `/api/tags` and shown as clickable chips (plus a Refresh action).
- Changes: "AI generate" button opens the assistant in commit mode; commit draft
  now persists per repo.
- Model listing now reads `/api/tags` (Ollama) or `/models` (OpenAI) instead of
  relying on a static suggestion list.
- Tool rows are rebuilt from the provider history after each turn, so tool calls/results
  still appear if a live progress event is missed.
- Smoke-tested the Ollama `/api/chat` request shape against the local server with
  a tool-capable model: the first round returned `message.tool_calls` with object
  arguments and the second round accepted our assistant/tool messages and
  produced a final answer.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK;
  `cargo test` 15 passed; `cargo clippy --all-targets` clean; `cargo fmt --check`
  clean.

### 2026-11-03 — Fix discard/unstage peel error

- Bug: discard (and unstage) failed with "git_object ... can not be successfully
  peeled into a commit". libgit2 1.9 `git_reset_default` peels the target to a
  commit, but the code passed the HEAD tree.
- Fixed `discard_file`, `unstage_file` and `unstage_all` to resolve
  `head().peel_to_commit()`; added regression tests for discard and unstage.
- Verified: `cargo test` 8 passed; `cargo clippy --all-targets` clean;
  `cargo fmt --check` clean.

### 2026-11-03 — Persistent error log

- Added `src/lib/errorlog.ts` (localStorage load/persist, max 200, text export).
- `AppStore` logs toast errors and uncaught runtime errors with repo context;
  added Settings → Error log tab with copy-all and clear, plus a toast "View log"
  action. Settings tab state moved into the store.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK.

### 2026-11-03 — Installed session-memory docs

- Inspected `README.md`, `package.json`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`, `src/`, `src-tauri/src/` and the existing feature list.
- Created `AGENTS.md`, `docs/FEATURES.md` and `docs/SCRATCH.md`; no app code changed.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK;
  `cargo test` 6 passed; `cargo clippy --all-targets` clean; `cargo fmt --check` clean.
