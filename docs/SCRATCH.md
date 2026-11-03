# SCRATCH — working memory

Append dated entries, keep the top current, prune what is obvious from code.

## Current focus

Session-memory flow is installed. No app code changed in this pass. Next session
should finish/review the uncommitted nav-count and version-bump edits.

## Next up

- Review/finish the dirty `src/routes/+page.svelte` (Branches nav count shows
  local/remote) and `src-tauri/tauri.conf.json` (`0.1.1` version bump).
- Pick the next feature from `docs/FEATURES.md` Ideas.

## Known issues / watch-outs

- Working tree is dirty: `src/routes/+page.svelte` and
  `src-tauri/tauri.conf.json` have uncommitted edits. Do not overwrite them.
- Version mismatch: `package.json` is `0.1.0`; `src-tauri/tauri.conf.json` is
  being bumped to `0.1.1` in the working tree. Check before packaging.
- No CI workflow. Validation is manual: `npm run check`, `npm run build`,
  `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`.
- No frontend lint/format script and no frontend tests; backend has 6 unit tests.
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

### 2026-11-03 — Installed session-memory docs

- Inspected `README.md`, `package.json`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`, `src/`, `src-tauri/src/` and the existing feature list.
- Created `AGENTS.md`, `docs/FEATURES.md` and `docs/SCRATCH.md`; no app code changed.
- Verified: `npm run check` 0 errors/0 warnings; `npm run build` OK;
  `cargo test` 6 passed; `cargo clippy --all-targets` clean; `cargo fmt --check` clean.
