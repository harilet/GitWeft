# SCRATCH — working memory

Append dated entries, keep the top current, prune what is obvious from code.

## Current focus

Persistent error log shipped (`src/lib/errorlog.ts`, `AppStore`, Settings →
Error log). Next session should finish/review the remaining uncommitted
nav-count and version-bump edits.

## Next up

- Review/finish the dirty `src/routes/+page.svelte` (Branches nav count shows
  local/remote) and `src-tauri/tauri.conf.json` (`0.1.1` version bump).
- Pick the next feature from `docs/FEATURES.md` Ideas.

## Known issues / watch-outs

- Working tree is dirty: `src/routes/+page.svelte`, `src/lib/errorlog.ts`,
  `src/lib/store.svelte.ts`, `src/lib/components/Settings.svelte` and
  `src-tauri/tauri.conf.json` have uncommitted edits. Do not overwrite them.
- Error log lives in `localStorage` (`gitweft.errors`, max 200) and covers toast
  errors plus uncaught `error`/`unhandledrejection`; it does not write a file.
- Version mismatch: `package.json` is `0.1.0`; `src-tauri/tauri.conf.json` is
  being bumped to `0.1.1` in the working tree. Check before packaging.
- No CI workflow. Validation is manual: `npm run check`, `npm run build`,
  `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`.
- No frontend lint/format script and no frontend tests; backend has 8 unit tests.
- Fetch/pull/push/clone require system `git` and configured credentials.
- Root `FEATURES.md` and `README.md` predate this flow; `docs/FEATURES.md` is the
  canonical living scope.

## Decisions

- 2026-11-03 — Session memory: add `AGENTS.md` (map), `docs/FEATURES.md` (scope)
  and `docs/SCRATCH.md` (rolling notes), read in that order. Why: new sessions
  should start grounded without rereading the whole repo.
- 2026-11-03 — Scope source: keep `docs/FEATURES.md` canonical; leave root
  `FEATURES.md` as the older detailed reference. Why: avoid churn outside the
  requested doc flow while giving agents one current source.

## Session log

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
