# SCRATCH — working memory

Append dated entries, keep the top current, prune what is obvious from code.

## Current focus

The conflict resolver now has line numbers and search. Every pane (ours/base/
theirs, per-conflict result, whole-file preview) is rendered by the new
`ConflictLines.svelte` component with a line-number gutter showing working-file
positions, and a search bar (`Ctrl/Cmd + F`) matches across all conflict sides
plus the whole file (built result in preview mode, manual text in edit mode)
with prev/next navigation, a match counter, a case toggle and a location hint.
The manual editor gained a scroll-synced line-number gutter; search jumps into
it select and scroll to the match.

## Next up

- Smoke-test the conflict resolver search: type a term present only in one
  side, navigate matches across blocks and the whole file, and check the
  manual editor jump and gutter alignment on a long file.
- Smoke-test auto refresh: commit externally, wait ≤30s and see status/history
  update without refresh; confirm rapid alt-tabbing causes no refresh storms
  and a deleted repo folder only logs once.
- Manual smoke test with a real conflicting merge: resolve per conflict, save,
  commit the merge and verify both parents appear in history.
- Exercise the AI recommendation with Ollama and OpenAI, and confirm that
  dismissing or closing it never changes the repository.
- Smoke-test the AI disable toggle: disable at repo scope and confirm the
  toolbar button, "AI generate" and the conflict AI panel disappear; disable at
  app scope and confirm repos with an enabled override keep their buttons.
- Also exercise delete/modify and binary conflicts, plus Abort merge.
- Pick the next feature from `docs/FEATURES.md` Ideas.

## Known issues / watch-outs

- Auto refresh polls every 30 seconds but pauses while the window is hidden
  (`document.hidden`), so a freshly shown window can be up to one interval
  stale; use refresh (`Ctrl/Cmd + R`) for an immediate update. Poll failures
  (e.g. the repo folder was deleted) log to the error log only once per outage.
- The conflict resolver handles merge/pull conflicts (`RepositoryState::Merge`);
  rebase and cherry-pick conflicts are not covered yet.
- Conflict blocks are parsed from the working-tree markers. Merge, diff3 and
  zdiff3 styles plus CRLF are supported; a file without markers opens in manual
  edit mode.
- Merge commits read `MERGE_HEAD` and clean `MERGE_HEAD` / `MERGE_MSG` /
  `MERGE_MODE`; the commit message is not prefilled from `MERGE_MSG`.
- Dev-only: if Vite reports an error in
  `*.svelte?svelte&type=style&lang.css` whose code frame shows the Svelte
  script/template, `vite-plugin-svelte` missed its virtual CSS cache during HMR.
  The source is fine; restart `npm run tauri dev`. A clean server load and
  `npm run build` both compile the same components correctly.
- AI provider settings (including API keys) live in `localStorage` under
  `gitweft.ai`; they are not encrypted.
- AI conflict recommendations send the conflict text to the configured provider.
  They are text-only, capped at about 120k characters, unavailable for binary
  conflicts, and never applied automatically. The model may leave conflict
  markers; the resolver warns and the user must review before saving.
- AI requests are non-streaming and time out after 180 seconds; tool progress is
  shown as status text, not tokens.
- The assistant's repository tools require a model with tool calling. A model
  that cannot call tools will answer from the conversation without inspecting the
  repository. Conflict recommendations use a plain chat request without tools.
- The tool loop stops after 8 rounds and truncates individual tool results at
  60,000 characters.
- `reqwest` is built with `native-tls`, so the existing `libssl-dev` prerequisite
  is required. `git2` remains built without TLS/SSH.
- Error log lives in `localStorage` (`gitweft.errors`, max 200) and covers toast
  errors plus uncaught `error`/`unhandledrejection`; it does not write a file.
- Version mismatch: `package.json` is `0.1.0`; `src-tauri/tauri.conf.json` is
  `0.1.4`. Check before packaging.
- No CI workflow. Validation is manual: `npm run check`, `npm run build`,
  `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`.
- No frontend lint/format script and no frontend tests; backend has 18 unit tests.
- Fetch/pull/push/clone require system `git` and configured credentials.
- Root `FEATURES.md` and `README.md` predate this flow; `docs/FEATURES.md` is the
  canonical living scope.

## Decisions

- 2026-11-11 — Conflict search is one bar across conflict sides plus the whole
  file, not per-pane inputs. Why: a term can live in unchanged text, in one
  side, or both, and a single ordered match list (blocks in order, then whole
  file) with prev/next keeps context; per-conflict result panes are excluded
  from the match list to avoid double-counting text that duplicates a side.
  Line numbers are working-file positions for conflict sides (from
  `parseConflicts`, which now records `startLine`/`oursStart`/`baseStart`/
  `theirsStart`), the block-start position for per-conflict results, and
  sequential 1..N for the whole-file view.
- 2026-11-11 — The manual editor keeps a plain `textarea` with a JS-synced
  line-number gutter (fixed 18.6px line height, no wrap), rather than a
  code-editor dependency. Why: keeps the resolver dependency-free; search
  inside it selects and scrolls to the match via `setSelectionRange` instead
  of DOM highlighting.
- 2026-11-11 — The AI off switch is a `disabled` flag inside `AiConfig`
  (app defaults and per-repo override), not a separate list of disabled paths.
  Why: it reuses the existing override model and `resolveAiConfig`, gives an
  app-wide off default for free, and keeps one source of truth. UI affordances
  are hidden when the resolved config is disabled; Settings stays reachable so
  it can be turned back on.
- 2026-11-11 — Auto refresh is a fixed 30-second poll, not a window-focus
  trigger or a filesystem watcher, with no settings toggle. Why: the user
  picked a simple cadence and the refresh button already covers "sooner";
  a Rust `notify`-based watcher remains a future idea.
- 2026-11-11 — The poll refreshes every open session, not just the active one.
  Why: sessions are tabs in one window, snapshots are cheap and stateless, and
  it keeps background tabs and nav-rail counts fresh.
- 2026-11-11 — Poll failures are silent except for one error-log entry per
  outage. Why: a repo unreachable offline or deleted externally must not toast
  every 30 seconds.
- 2026-11-11 — `applySnapshot` closes a working-tree diff whose file no longer
  has that staged/unstaged state, and a conflict resolver whose file is no
  longer conflicted. Why: a stale diff or resolver is misleading; commit diffs
  are immutable and never pruned.
- 2026-11-06 — AI conflict advice is recommendation-only: it uses a plain chat
  request with no tools, returns file contents, and never writes or stages. The
  user must apply the suggestion to the editor and then explicitly save. Why:
  conflict resolution needs human review and a clear accept step.
- 2026-11-06 — Manual conflict resolution is fully user-driven; the resolver
  exposes no AI write path. Why: conflict choices need explicit human review.
- 2026-11-06 — Whole-file side resolution uses index stages 1/2/3 (base/ours/
  theirs) instead of parsing the working file. Why: it handles binary blobs and
  delete/modify conflicts, where one side has no file at all.
- 2026-11-06 — `commit` now includes every `MERGE_HEAD` parent and clears merge
  state after a successful commit. Why: the old single-parent commit finished the
  working tree but produced a non-merge commit and left the merge in progress.
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

### 2026-11-12 — Removed decorative text

- `Welcome.svelte`: dropped the `landing-meta` strip ("01 / Projects",
  "N Recent · Local Repository · Git"), both `panel-head` bars ("02 / Recent" /
  "Resume", "03 / New Session" / "Open · Init · Clone"), the row index
  numbers in the recents and action lists, and the "GitWeft keeps every
  repository…" panel note, plus their now-unused CSS.
- `+page.svelte`: dropped the nav-rail `rail-head` ("Workspace" + padded item
  count) and the `01`–`05` nav-item index numbers, plus their CSS.
- `Modal.svelte` ("Dialog"), `Settings.svelte` ("GitWeft") and
  `AiPanel.svelte` ("GitWeft AI"): removed the non-informative eyebrow labels
  above dialog titles.
- Kept all informative text: counts, section names, statuses, instructions
  ("Resolve before committing", "Working tree", "Read-only"), field labels
  ("Path"/"Head"), the TitleBar wordmark and the Welcome hero headings.
- Verified: `npm run check` 0 errors/0 warnings. Frontend-only change, so
  `npm run build`/`cargo` checks were not re-run.

### 2026-11-11 — Conflict resolver line numbers and search

- `src/lib/conflicts.ts`: `ConflictBlock` gained `startLine`, `oursStart`,
  `baseStart` and `theirsStart` (1-based working-file positions) captured
  during parsing; verified with a tsx script covering merge and diff3 styles
  and a marker-less file.
- New `src/lib/components/ConflictLines.svelte`: numbered, searchable line
  renderer (line-number gutter, per-line match highlighting, active-match
  row highlight, `data-pane`/`data-line` attributes used as scroll targets).
- `ConflictResolver.svelte`: sides, per-conflict results and the whole-file
  preview use `ConflictLines`; a search bar (input, match counter, prev/next,
  case toggle, location hint) drives a derived match list across ours/base/
  theirs and the whole file (`manualText` when editing manually, `resolved`
  otherwise). Enter/Shift+Enter navigate, Ctrl/Cmd+F focuses search, Escape in
  the input clears it instead of closing the modal, and jumping to a whole-file
  match auto-expands the preview. The manual editor has a scroll-synced
  line-number gutter; jumps into it set the selection and scroll to the line.
- Also hardened re-initialization: the resolver resets choices/search when the
  conflict content changes, not just on a path change (fixes stale state when
  a resolved file is reopened), and clears the search when the modal closes.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK. No Rust
  changes, so `cargo` checks were not re-run.

### 2026-11-11 — Per-repo AI off switch

- `AiConfig` gained a `disabled` flag (sanitized on load, default `false`), so
  app defaults and each repository override can turn the assistant off;
  existing stored settings load unchanged.
- Settings → AI shows a "Disable the AI assistant" checkbox in both scopes; the
  provider fields gray out while disabled, and the repo no-override notice
  explains when app defaults disable AI.
- While the resolved config is disabled, the toolbar AI button, the Changes
  "AI generate" button and the conflict resolver's AI recommendation panel are
  hidden; `openAi` refuses to open and `sendAi` toasts instead of calling a
  provider. Saving or clearing AI settings closes any open AI drawer whose
  session just became disabled (`syncAiPanels`).
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK. No Rust
  changes, so `cargo` checks were not re-run.

### 2026-11-11 — Auto refresh (30-second poll)

- Added `RepoSession.autoRefresh`: fetches the snapshot without setting `busy`,
  skips while the session is busy, network progress is showing or a poll is
  already in flight, and logs failures to the error log once per consecutive
  outage (never toasts).
- Refactored `refreshAll` and the poll to share `applySnapshot`, which assigns
  the snapshot fields and then prunes stale UI state: working-tree diffs whose
  file lost the staged/unstaged state close, and an open conflict resolver
  closes when its file is no longer conflicted.
- `DiffTarget` gained optional `path`/`staged` (set by `loadWorkingDiff`) so
  pruning can identify working-tree diffs; commit diffs carry no path and are
  never pruned.
- `AppStore` starts one 30-second interval (browser guard, like the existing
  error listeners) that skips ticks while `document.hidden` and calls
  `autoRefresh` on every open session.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK. No Rust
  changes, so `cargo` checks were not re-run.

### 2026-11-06 — AI conflict recommendations

- Added `suggestConflictResolution` in `src/lib/ai.ts`: a plain, tool-free chat
  request that sends the conflict markers (or stage versions for marker-less
  conflicts) and asks for the complete resolved file.
- Added `normalizeSuggestedFile` and `hasConflictMarkers`; suggestions preserve
  the original LF/CRLF style and are capped at about 120k characters.
- `ConflictResolver.svelte` now has an AI recommendation panel with Suggest /
  Regenerate / Dismiss, a read-only preview, and Apply to editor. Applying only
  fills the manual editor; `Save & mark resolved` remains the sole write.
- The model receives no tools and cannot stage, commit or modify anything.
- Each conflict block now shows its own Result preview; the full merged file is
  hidden behind Preview / Edit manually so the resolver stays focused.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK.

### 2026-11-06 — Merge conflict resolver

- Added `get_conflict`, `resolve_conflict` and `abort_merge`; `RepoInfo.merging`
  tracks an in-progress merge. Conflict reads return the working file plus index
  stages 1/2/3 and binary detection.
- Resolution can stage custom merged text or copy a whole stage, including
  deleting the file when the chosen side removed it.
- Fixed merge commits to include every `MERGE_HEAD` parent and clean
  `MERGE_HEAD` / `MERGE_MSG` / `MERGE_MODE`, so committing after a resolved merge
  records a real merge commit and leaves the repository clean.
- Added `src/lib/conflicts.ts`: marker parser for merge/diff3/zdiff3 styles with
  CRLF support, default choices and merged-result builder.
- Changes now separates conflicted files, shows a merge banner with Abort merge,
  and keeps conflicts out of the staged/unstaged lists until resolved.
- Added `ConflictResolver.svelte`: side-by-side ours/base/theirs blocks with
  ours/base/theirs/both choices, manual merged-file editing, whole-file side and
  delete actions, binary conflict handling, and Save & mark resolved.
- Merge/pull now refresh the snapshot even when Git exits with a conflict, so the
  resolver appears without a manual refresh.
- Conflict resolution stays explicit; the AI recommendation flow above was added
  later and never auto-applies or stages.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK;
  `cargo test` 18 passed; `cargo clippy --all-targets` clean; `cargo fmt --check`
  clean.

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
- History tools accept branch names, tags and revision expressions. `compare_branches`
  returns commits reachable from `head` but not `base`, so merge-impact questions no
  longer require a SHA.
- Smoke-tested the Ollama `/api/chat` request shape against the local server with
  a tool-capable model: the first round returned `message.tool_calls` with object
  arguments and the second round accepted our assistant/tool messages and
  produced a final answer.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK;
  `cargo test` 16 passed; `cargo clippy --all-targets` clean; `cargo fmt --check`
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
