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
- [x] Auto refresh: every open session re-polls its snapshot every 30 seconds
      while the window is visible (manual refresh stays available for sooner);
      stale working-tree diffs and conflict resolvers close automatically and
      poll failures are logged to the error log once per outage, never toasted.

- [x] Nav rail Branches count shows local/remote totals and the app version is
      `0.1.2` (`src-tauri/tauri.conf.json`).
- [x] AI assistant: Ollama and OpenAI providers with app-level and per-repository
      settings, provider model discovery (installed Ollama models and OpenAI model
      IDs), a read-only chat drawer with collapsible per-call tool activity and
      results, tools for repository info, branch/ref-aware commit history, branch
      comparison, branches, staged/unstaged diffs, and commit-message generation
      from the Changes view.
- [x] AI can be turned off per scope: Settings → AI has a "Disable the AI
      assistant" checkbox for app defaults or a single repository override; while
      disabled, the toolbar AI button, the Changes "AI generate" button and the
      conflict resolver's AI recommendation panel are hidden.
- [x] Merge conflict resolution: conflicted files are listed separately in
      Changes with a merge banner, a side-by-side ours/base/theirs resolver that
      accepts a choice per conflict (or both), manual editing of the merged
      result, whole-file side selection including delete/modify conflicts,
      binary-conflict handling, abort merge, and merge commits that keep all
      parents. An optional AI recommendation can propose a merged file, but it
      is text-only, never writes or stages, and the user must apply and save it.
- [x] Conflict resolver line numbers and search: every pane (ours/base/theirs,
      per-conflict result, whole-file preview) shows file line numbers, and a
      search bar (`Ctrl/Cmd + F`) matches across all conflict sides and the
      whole file with prev/next navigation, match count, case toggle and a
      hint of where the current match is; the manual editor has a synced
      line-number gutter and search jumps select and scroll to the match.
- [x] Themed in-app confirm dialogs replace the native OS dialogs for discard,
      branch delete (merged vs force), abort merge, push, remove remote,
      delete tag and drop stash; each states exactly what is lost, and a
      store-level queue settles concurrent confirms in order.
- [x] The conflict resolver persists choices, manual edits and preview state
      per conflicted file while the merge is open; closing a dirty resolver
      offers Cancel / Keep choices / Discard instead of silently losing work,
      and reopening the file restores the saved state.
- [x] Resolver density: blocks collapse behind their header once a side is
      chosen (header keeps the choice control and a chosen badge, click to
      re-expand), with a "N/M chosen" progress bar in the header bar, and
      the whole-file take/delete actions open from a single "Whole file" 
      dropdown instead of three always-visible buttons.
- [x] Merge confirms with a preview: `get_merge_preview` counts commits that
      would land from the branch into HEAD ahead of the merge, so merging
      never fires without a themed confirm.
- [x] Toolbar regroup: Pull/Push form the network group as a split control —
      Pull plus a caret dropdown listing per-remote Fetch (and "Fetch all
      remotes" with several remotes); with a single remote the caret is a
      direct one-click Fetch. Escape or an outside click closes the menu;
      per-remote fetch also lives in Branches → Remotes, and silent
      auto-fetch still covers opening; a divider separates network from
      app functions. There is no toolbar refresh button: sessions poll
      every 30 seconds while the window is visible, refresh immediately
      when the window is shown again, and `Ctrl/Cmd+R` forces a manual
      refresh.
- [x] Clone progress is mirrored into the Welcome clone modal (live
      `git-progress` line), the dialog locks and disables inputs while
      cloning, and a failed clone keeps the entered URL/destination.
- [x] Backend failures surface as friendly toasts (`friendlyError` mappings
      for credentials, network, not-a-repository, name-taken), while the
      full technical detail is kept in Settings → Error log.
- [x] Dead-end empty states carry the next action: History distinguishes
      "no commits yet" (jumps to Changes) from "no commits match the
      filter" (clears the filter), and empty local branches link to the
      first commit; view titles (History/Branches/Tags/Stashes) get one
      type-size step above the microtype labels.
- [x] Polish pass: one shared `.danger-text` (red at rest) replaces three
      conflicting local definitions, toast colors join the danger/badge
      palette, `prefers-reduced-motion` disables transitions and
      animations, the nav-rail Changes count reflects the rows actually
      displayed, and the commit label drops its trailing space at 0 files.
- [x] Destructive confirms are safe by default: focus lands on Cancel (a
      Tab is needed to reach the destructive button) and Enter only
      confirms benign asks (push, merge); destructive requests require an
      explicit click.
- [x] Accent washes derive from `--primary-color` via `color-mix`, so the
      accent setting now reaches DiffView hunk bars and chosen conflict
      sides, not just buttons and focus states; the smallest letterforms
      sit on a uniform 10px floor.
- [x] Keyboard support: global `:focus-visible` outline, Space as well as
      Enter on list rows, visible-at-rest text row actions (Stage /
      Unstage / Discard / Resolve), `/` focuses the History filter, branch
      switcher supports Escape and arrow/Enter navigation with listbox
      ARIA, `Ctrl/Cmd+1–5` switch views, `Ctrl/Cmd+Shift+S` stages
      everything, and Settings → Shortcuts lists every binding.
- [x] The left-edge stripe idiom is replaced by background/inset-ring
      treatment across file rows, commit rows, branch rows, nav items and
      settings tabs; conflict and AI tool rows carry their state in status
      text and tints instead of edge colors.
- [x] Edge hardening: popping a stash asks for confirmation, the clone
      dialog validates URL shape as you type (https/ssh/git@ or absolute
      path) before enabling Clone, and error toasts stay until dismissed
      (View log / ✕) while success and info toasts keep their timer.
- [x] The diff panel is scoped to the view that opened it: switching views
      (or returning to Changes after a merge/pull conflict) closes the
      panel and the History selection, so a file or commit diff never
      outlives its context.
- [x] Diff files are collapsible: each file header toggles its body
      (chevron, `aria-expanded`), shows green/red +N/−N change counts per
      file, and collapse state resets when another commit is opened.

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
