# GitWeft — Feature Scope

Living scope list. Legend: `[x]` shipped, `[~]` in progress, `[ ]` idea.
Keep entries short; move shipped items from `docs/SCRATCH.md` into Shipped.

## Shipped

- [x] Open, initialize and clone repositories; recent projects with pin/unpin,
      remove, and multi-project tabs with independent sessions.
- [x] Working tree: staged/unstaged file lists with status codes, stage/unstage
      one or all, discard with confirmation, and per-file diff viewing.
- [x] Commit staged changes with a message (`Ctrl/Cmd + Enter`), using the
      configured Git identity with empty-message and conflict guards.
- [x] History: paginated commit log (300/page), filter by message/author/SHA,
      ref badges, and full commit diffs.
- [x] Branch graph topology, local and remote branch lists, upstream and
      ahead/behind counts, create/checkout/merge/delete, and remote-branch
      checkout that creates a tracking branch.
- [x] Remotes: add/remove, fetch/pull/push with `--prune`, push upstream setup,
      live progress, and throttled silent auto-fetch on open.
- [x] Tags: list, create lightweight/annotated, and delete.
- [x] Stashes: list, create including untracked files, apply, pop and drop.
- [x] Frameless window and custom title bar, accent-color theming, settings
      dialog (appearance + Git identity local/global), toolbar branch switcher,
      nav rail counts, toasts, spinners and empty states.
- [x] Shortcuts: `Ctrl/Cmd + R` refresh, `Ctrl/Cmd + ,` settings, `Esc` close dialog.
- [x] Persistent error log: toast errors and uncaught runtime errors are saved on
      device (max 200), viewable in Settings → Error log with copy-all and clear.

- [x] Nav rail Branches count shows local/remote totals and the app version is
      `0.1.2` (`src-tauri/tauri.conf.json`).
- [x] AI assistant: Ollama and OpenAI providers with app-level and per-repository
      settings, provider model discovery (installed Ollama models and OpenAI model
      IDs), a read-only chat drawer with collapsible per-call tool activity and
      results, tools for repository info, branch/ref-aware commit history, branch
      comparison, branches, staged/unstaged diffs, and commit-message generation
      from the Changes view.
- [x] Merge conflict resolution: conflicted files are listed separately in
      Changes with a merge banner, a side-by-side ours/base/theirs resolver that
      accepts a choice per conflict (or both), manual editing of the merged
      result, whole-file side selection including delete/modify conflicts,
      binary-conflict handling, abort merge, and merge commits that keep all
      parents. An optional AI recommendation can propose a merged file, but it
      is text-only, never writes or stages, and the user must apply and save it.

## In progress

- [~] None currently.

## Ideas

- [ ] Partial staging (per-hunk / per-line)
- [ ] Amend commit / rebase / cherry-pick / reset
- [ ] Blame view and file history
- [ ] Commit signing (GPG/SSH)
- [ ] Submodule support
- [ ] Distribution targets beyond `deb`

## Non-goals

- Bundling TLS/SSH in `git2`; network operations intentionally use system `git`.
- Server-side rendering or a Node server; the frontend is a static SPA.
- Inventing commit authors; commits use the configured Git signature.
- AI write actions: the assistant may read repository state but never stages,
  unstages, commits, discards, checks out or pushes.
