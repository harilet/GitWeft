# AGENTS.md — GitWeft

Map for a new agent. GitWeft is a Tauri 2 desktop Git client: Rust backend
(`git2` for local work, system `git` for network work) + SvelteKit 5 SPA frontend.

## New session checklist

1. Read this file (the map).
2. Read `docs/FEATURES.md` for shipped scope, in-progress work, ideas and non-goals.
3. Read `docs/SCRATCH.md` for current focus, next up, known issues and decisions.
4. Inspect code only where the task needs it.

## Verified commands

Run from the repo root unless noted.

```bash
npm install
npm run dev                 # Vite dev server only
npm run tauri dev           # desktop app in development
npm run build               # static frontend -> build/
npm run preview             # preview the frontend build
npm run check               # svelte-kit sync + svelte-check (TS/Svelte diagnostics)
npm run tauri build         # release desktop bundle (deb only)

cd src-tauri && cargo test                 # 16 unit tests in git.rs + ai.rs
cd src-tauri && cargo clippy --all-targets # Rust lint
cd src-tauri && cargo fmt --check          # Rust format check; use cargo fmt to write
```

There is **no separate npm lint or format script** (no ESLint/Prettier config).
Frontend validation is `npm run check`; Rust lint/format are the cargo commands.
There is no CI workflow in the repo — run the checks manually.

Last verified 2026-11-03: `npm run check` 0 errors/0 warnings; `npm run build` OK;
`cargo test` 16 passed; `cargo clippy --all-targets` clean; `cargo fmt --check` clean.

## Architecture map

| Path | Owns |
| --- | --- |
| `src/routes/+page.svelte` | Main shell: toolbar, nav rail, view switching, diff panel, shortcuts. |
| `src/routes/+layout.svelte` | Root layout; applies persisted accent theme on mount. |
| `src/routes/+layout.ts` | SPA mode (`ssr = false`) required by Tauri. |
| `src/lib/api.ts` | Typed `invoke` wrappers (`git.*`) and all IPC data types. |
| `src/lib/store.svelte.ts` | `AppStore` + `RepoSession`: multi-repo state, refresh, all actions, recents, auto-fetch, progress listener. |
| `src/lib/recents.ts` | Recent-project persistence (localStorage, pin/sort, max 30). |
| `src/lib/settings.ts` | Accent presets, validation, persistence and CSS variables. |
| `src/lib/ai.ts` | AI provider config, localStorage settings (app + repo overrides), read-only tool schemas/executors and the agent tool loop. |
| `src/lib/format.ts` | Relative/absolute time formatting and unified-patch parser. |
| `src/lib/actions.ts` | `autofocus` Svelte action. |
| `src/lib/components/Welcome.svelte` | Start screen: recents, open/init/clone dialogs. |
| `src/lib/components/TitleBar.svelte` | Frameless title bar, project tabs, window controls. |
| `src/lib/components/Changes.svelte` | Staged/unstaged lists, stage/unstage/discard, commit composer. |
| `src/lib/components/History.svelte` | Commit list/graph tabs, filter, ref badges, pagination. |
| `src/lib/components/Graph.svelte` | Branch topology layout/rendering. |
| `src/lib/components/Branches.svelte` | Branches/remotes tabs, create/checkout/merge/delete, remote add/remove/network actions. |
| `src/lib/components/BranchSwitcher.svelte` | Toolbar searchable local/remote branch switcher. |
| `src/lib/components/Tags.svelte` | Tag list, create/delete. |
| `src/lib/components/Stashes.svelte` | Stash list, create/apply/pop/drop. |
| `src/lib/components/Settings.svelte` | Appearance, Git identity (local/global), AI provider/scope settings and error-log dialogs. |
| `src/lib/components/AiPanel.svelte` | Read-only AI assistant drawer: chat, tool activity, commit-message generation/insert. |
| `src/lib/components/DiffView.svelte` | Renders a parsed patch with line markers/numbers. |
| `src/lib/components/Modal.svelte` | Shared modal shell, backdrop/Esc handling. |
| `src/app.css` | Global theme and shared component styles. |
| `src-tauri/src/git.rs` | All Git commands, data structs, network CLI runner, unit tests. |
| `src-tauri/src/ai.rs` | Ollama/OpenAI chat and model-list bridge over `reqwest`; normalizes tool calls/messages; unit tests. |
| `src-tauri/src/lib.rs` | Tauri builder, plugins and command registration. |
| `src-tauri/src/main.rs` | Desktop binary entry point. |
| `src-tauri/tauri.conf.json` | Window config (frameless), dev/build hooks, deb bundle. |
| `src-tauri/capabilities/default.json` | Window, dialog and opener permissions. |
| `src-tauri/Cargo.toml` | Rust deps (`git2` without default features, `reqwest` for AI HTTP) and release profile. |
| `README.md` / root `FEATURES.md` | Older overview/detailed feature list. `docs/FEATURES.md` is canonical for scope. |
| `docs/FEATURES.md` / `docs/SCRATCH.md` | Living scope + rolling session memory. |

Generated/ignored: `node_modules/`, `build/`, `.svelte-kit/`, `src-tauri/target/`,
`src-tauri/gen/`.

## Data model and storage

No database. The app persists only small client-side preferences:

| Key | Shape / purpose |
| --- | --- |
| `gitweft.recents` | JSON array of `{ path, name, pinned, lastOpened }`, max 30; reads legacy `gitman.recents` if absent. |
| `gitweft.settings` | `{ accent: "#rrggbb" }`; invalid values fall back to `#7b8cff`. |
| `gitweft.lastFetch` | Map of repository path -> timestamp used for the 5-minute auto-fetch throttle. |
| `gitweft.ai` | `{ app: AiConfig, repos: { [path]: AiConfig } }`; provider, base URL, model, API key and temperature. Repo entries override app defaults. |

Git identity lives in Git config (`user.name` / `user.email`), read via `git2`,
written via system `git config --local` or `--global`.

IPC structs (camelCase over the wire) in `src/lib/api.ts` / `src-tauri/src/git.rs`:
`RepoInfo`, `FileStatus`, `CommitInfo`, `BranchInfo`, `BranchGraph`/`BranchGraphNode`,
`RemoteInfo`, `TagInfo`, `StashInfo`, `UserConfig`. `Snapshot` bundles repo, status,
log, branches, graph, remotes, tags, stashes and user config for one refresh.

## Product rules / non-negotiables

- **Stateless backend:** every command receives the target repository path. Keep it
  that way so multiple open sessions stay independent and concurrency-safe.
- **Local vs network split:** local operations use `git2`; clone/fetch/pull/push/merge
  shell out to system `git` so SSH agents and credential helpers work. Do not move
  network operations to `git2` without adding TLS/SSH support.
- **No bundled TLS/SSH:** `git2` is built with `default-features = false`; changing
  this affects build size and the system-git integration.
- **Responsiveness:** commands are `#[tauri::command(async)]`; network progress is
  streamed through the `git-progress` event. Keep long work off the UI thread.
- **Safe-by-default destructive actions:** discard is confirmed in the UI; branch
  delete is safe unless the user explicitly forces an unmerged branch.
- **Network defaults:** fetch/pull use `--prune`; push sets upstream only when the
  current branch has none. Auto-fetch is best-effort, throttled to 5 minutes and
  silent on failure.
- **SPA only:** `ssr = false` plus the static adapter; there is no Node server.
- **Commit identity:** use the configured signature; never invent an author.
- **AI is read-only:** the assistant may inspect repository info, commits, branches,
  status and diffs through tools, but must never stage, unstage, commit, discard,
  checkout, push or otherwise write. Do not add write tools to `src/lib/ai.ts`.

## Gotchas

- Prerequisites: Node.js 20+, Rust stable, `git` on `PATH`, and Tauri system
  dependencies. Debian/Ubuntu needs `libwebkit2gtk-4.1-dev`, `build-essential`,
  `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`,
  `libayatana-appindicator3-dev`, `librsvg2-dev`.
- The window is frameless (`decorations: false`). Window controls depend on the
  permissions in `src-tauri/capabilities/default.json`; removing them breaks the UI.
- Network operations fail without system `git` and configured credentials; local
  operations do not need network access.
- AI requires a reachable Ollama server or an OpenAI-compatible endpoint and is
  called from Rust via `reqwest`; settings (including API keys) live in
  `localStorage` under `gitweft.ai`.
- Tests are backend-only and create temporary repositories; run them from
  `src-tauri/`. There are no frontend tests.
- `package.json` version is `0.1.0`; `src-tauri/tauri.conf.json` is `0.1.4` in the
  current working tree. Check the release version before packaging.
- Do not edit generated directories (`build/`, `.svelte-kit/`, `src-tauri/gen/`,
  `src-tauri/target/`).

## After finishing work

Update `docs/FEATURES.md` (move shipped items) and `docs/SCRATCH.md` (current focus,
decisions, dated session log with verification results). Do this at the end of a
session, not per commit.
