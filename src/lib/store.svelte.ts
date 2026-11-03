import { listen } from "@tauri-apps/api/event";
import {
  git,
  type BranchGraph,
  type BranchInfo,
  type CommitInfo,
  type FileStatus,
  type RemoteInfo,
  type RepoInfo,
  type StashInfo,
  type TagInfo,
  type UserConfig,
} from "./api";
import {
  loadRecents,
  persistRecents,
  sortRecents,
  type RecentProject,
} from "./recents";
import {
  createError,
  loadErrors,
  persistErrors,
  type ErrorLogEntry,
} from "./errorlog";
import {
  COMMIT_PROMPT,
  isAiConfigured,
  loadAiSettings,
  normalizeCommitMessage,
  persistAiSettings,
  resolveAiConfig,
  runAiAgent,
  type AiConfig,
  type AiMessage,
  type AiSettings,
  type AiToolCall,
} from "./ai";

export type ViewKey = "changes" | "history" | "branches" | "tags" | "stashes";
export type SettingsTab = "appearance" | "identity" | "ai" | "errors";
export type AiMode = "chat" | "commit";

export interface AiEntry {
  id: number;
  role: "user" | "assistant" | "tool";
  content: string;
  toolName?: string;
  toolArguments?: string;
  toolResult?: string;
  toolState?: "running" | "done" | "error";
}

export interface DiffTarget {
  title: string;
  subtitle: string;
  patch: string;
}

let sessionCounter = 0;

const LOG_PAGE = 300;
const AUTO_FETCH_INTERVAL = 5 * 60 * 1000;
const LAST_FETCH_KEY = "gitweft.lastFetch";

function lastFetchedAt(path: string): number {
  try {
    const raw = localStorage.getItem(LAST_FETCH_KEY);
    if (!raw) return 0;
    const map = JSON.parse(raw);
    return typeof map?.[path] === "number" ? map[path] : 0;
  } catch {
    return 0;
  }
}

function markFetched(path: string) {
  try {
    const raw = localStorage.getItem(LAST_FETCH_KEY);
    const map = raw ? JSON.parse(raw) : {};
    map[path] = Date.now();
    localStorage.setItem(LAST_FETCH_KEY, JSON.stringify(map));
  } catch {
    // ignore storage errors
  }
}

/**
 * Rebuild the visible conversation from the provider history. Tool calls live
 * alongside normal messages there, so this guarantees tool rows survive even if
 * a live progress callback was missed.
 */
function buildAiEntries(
  history: AiMessage[],
  nextId: () => number,
): AiEntry[] {
  const entries: AiEntry[] = [];
  const pending: number[] = [];

  const pushTool = (call: NonNullable<AiMessage["toolCalls"]>[number]) => {
    entries.push({
      id: nextId(),
      role: "tool",
      content: call.name,
      toolName: call.name,
      toolArguments: call.arguments,
      toolResult: "",
      toolState: "done",
    });
    pending.push(entries.length - 1);
  };

  for (const message of history) {
    if (message.role === "user") {
      entries.push({ id: nextId(), role: "user", content: message.content });
    } else if (message.role === "assistant") {
      const calls = message.toolCalls ?? [];
      if (calls.length > 0) {
        if (message.content.trim()) {
          entries.push({
            id: nextId(),
            role: "assistant",
            content: message.content,
          });
        }
        for (const call of calls) pushTool(call);
      } else if (message.content.trim()) {
        entries.push({
          id: nextId(),
          role: "assistant",
          content: message.content,
        });
      }
    } else if (message.role === "tool" && pending.length > 0) {
      const match = pending.findIndex(
        (index) => entries[index].toolName === message.toolName,
      );
      const entryIndex = pending.splice(match >= 0 ? match : 0, 1)[0];
      entries[entryIndex] = {
        ...entries[entryIndex],
        toolResult: message.content,
        toolState: message.content.startsWith("Error:") ? "error" : "done",
      };
    }
  }

  return entries;
}

/**
 * A single open repository. Every backend call is scoped to this repository's
 * path, so multiple sessions can be open (and acted upon) independently.
 */
export class RepoSession {
  readonly id: string;
  private app: AppStore;

  readonly path: string;
  repo = $state<RepoInfo | null>(null);
  status = $state<FileStatus[]>([]);
  log = $state<CommitInfo[]>([]);
  logHasMore = $state(false);
  branches = $state<BranchInfo[]>([]);
  graph = $state<BranchGraph | null>(null);
  graphIncludeRemote = $state(false);
  remotes = $state<RemoteInfo[]>([]);
  tags = $state<TagInfo[]>([]);
  stashes = $state<StashInfo[]>([]);
  user = $state<UserConfig | null>(null);

  view = $state<ViewKey>("changes");
  diff = $state<DiffTarget | null>(null);
  selectedCommit = $state<CommitInfo | null>(null);

  commitMessage = $state("");

  aiOpen = $state(false);
  aiMode = $state<AiMode>("chat");
  aiEntries = $state<AiEntry[]>([]);
  aiDraft = $state("");
  aiRunning = $state(false);
  aiStatus = $state<string | null>(null);
  private aiHistory: AiMessage[] = [];
  private aiEntrySeq = 0;

  busy = $state(false);

  constructor(app: AppStore, info: RepoInfo) {
    this.app = app;
    this.id = `s${++sessionCounter}`;
    this.path = info.path;
    this.repo = info;
  }

  get staged(): FileStatus[] {
    return this.status.filter((f) => f.staged);
  }

  get unstaged(): FileStatus[] {
    return this.status.filter((f) => f.unstaged);
  }

  get localBranches(): BranchInfo[] {
    return this.branches.filter((b) => !b.isRemote);
  }

  get remoteBranches(): BranchInfo[] {
    return this.branches.filter((b) => b.isRemote);
  }

  private async run<T>(fn: () => Promise<T>, successMsg?: string): Promise<T | null> {
    this.busy = true;
    try {
      const result = await fn();
      if (successMsg) this.app.notify(successMsg, "success");
      return result;
    } catch (e) {
      this.app.notify(typeof e === "string" ? e : String(e), "error");
      return null;
    } finally {
      this.busy = false;
      this.app.progress = null;
    }
  }

  private async runVoid(fn: () => Promise<void>, successMsg?: string): Promise<boolean> {
    this.busy = true;
    try {
      await fn();
      if (successMsg) this.app.notify(successMsg, "success");
      return true;
    } catch (e) {
      this.app.notify(typeof e === "string" ? e : String(e), "error");
      return false;
    } finally {
      this.busy = false;
      this.app.progress = null;
    }
  }

  async refreshAll() {
    if (!this.repo) return;
    this.busy = true;
    try {
      const s = await git.getSnapshot(
        this.path,
        LOG_PAGE,
        this.graphIncludeRemote,
      );
      this.status = s.status;
      this.log = s.log;
      this.logHasMore = s.log.length >= LOG_PAGE;
      this.branches = s.branches;
      this.graph = s.graph;
      this.remotes = s.remotes;
      this.tags = s.tags;
      this.stashes = s.stashes;
      this.user = s.user;
      this.repo = s.repo;
    } catch (e) {
      this.app.notify(typeof e === "string" ? e : String(e), "error");
    } finally {
      this.busy = false;
    }
  }

  /**
   * Prune stale remote-tracking branches in the background when a project is
   * opened. Throttled and silent: failures (offline, auth, missing remote) are
   * ignored so opening a repository never blocks or reports an error.
   */
  async autoFetch() {
    if (!this.repo || this.remotes.length === 0) return;
    if (Date.now() - lastFetchedAt(this.path) < AUTO_FETCH_INTERVAL) return;
    try {
      await git.fetch(this.path, null);
      markFetched(this.path);
      await Promise.all([this.refreshBranches(), this.refreshGraph()]);
    } catch {
      // Background sync: ignore errors.
    } finally {
      this.app.progress = null;
    }
  }

  async refreshStatus() {
    if (!this.repo) return;
    this.status = await git.getStatus(this.path);
  }

  async refreshLog() {
    if (!this.repo) return;
    this.log = await git.getLog(this.path, LOG_PAGE);
    this.logHasMore = this.log.length >= LOG_PAGE;
  }

  async loadMoreLog() {
    if (!this.repo) return;
    const next = await this.run(() =>
      git.getLog(this.path, LOG_PAGE, this.log.length),
    );
    if (next !== null) {
      this.log = [...this.log, ...next];
      this.logHasMore = next.length >= LOG_PAGE;
    }
  }

  async refreshBranches() {
    if (!this.repo) return;
    this.branches = await git.listBranches(this.path);
  }

  async refreshGraph() {
    if (!this.repo) return;
    this.graph = await git.getBranchGraph(this.path, this.graphIncludeRemote);
  }

  async setGraphRemote(includeRemote: boolean) {
    this.graphIncludeRemote = includeRemote;
    await this.refreshGraph();
  }

  async refreshRemotes() {
    if (!this.repo) return;
    this.remotes = await git.listRemotes(this.path);
  }

  async refreshTags() {
    if (!this.repo) return;
    this.tags = await git.listTags(this.path);
  }

  async refreshStashes() {
    if (!this.repo) return;
    this.stashes = await git.listStashes(this.path);
  }

  async saveUserConfig(name: string, email: string, global: boolean) {
    const ok = await this.runVoid(
      () => git.setUserConfig(this.path, name, email, global),
      "Git identity saved",
    );
    if (ok) {
      this.user = await git
        .getUserConfig(this.path)
        .catch(() => this.user);
    }
    return ok;
  }

  async loadWorkingDiff(path: string, staged: boolean) {
    const patch = await this.run(() =>
      git.getWorktreeDiff(this.path, path, staged),
    );
    if (patch !== null) {
      this.diff = {
        title: path,
        subtitle: staged ? "Staged changes" : "Working tree changes",
        patch,
      };
    }
  }

  async loadCommit(commit: CommitInfo) {
    this.selectedCommit = commit;
    const patch = await this.run(() =>
      git.getCommitDiff(this.path, commit.oid),
    );
    if (patch !== null) {
      this.diff = {
        title: commit.summary || commit.short,
        subtitle: `${commit.short} · ${commit.author}`,
        patch,
      };
    }
  }

  openAi(mode: AiMode = "chat") {
    if (mode === "commit" && !this.aiRunning) {
      this.aiEntries = [];
      this.aiHistory = [];
      this.aiDraft = COMMIT_PROMPT;
    }
    this.aiOpen = true;
    this.aiMode = mode;
    this.aiStatus = null;
    if (mode === "commit" && !this.aiDraft.trim()) {
      this.aiDraft = COMMIT_PROMPT;
    }
  }

  closeAi() {
    this.aiOpen = false;
    this.aiStatus = null;
  }

  resetAi() {
    if (this.aiRunning) return;
    this.aiEntries = [];
    this.aiHistory = [];
    this.aiDraft = "";
    this.aiStatus = null;
  }

  async sendAi() {
    const prompt = this.aiDraft.trim();
    if (!prompt || this.aiRunning) return;

    const config = this.app.aiConfig;
    if (!isAiConfigured(config)) {
      this.app.notify("Configure an AI provider in Settings → AI", "error");
      this.app.openAiSettings();
      return;
    }

    this.aiDraft = "";
    this.aiRunning = true;
    this.aiStatus = "Thinking…";
    this.aiEntries = [
      ...this.aiEntries,
      { id: ++this.aiEntrySeq, role: "user", content: prompt },
    ];

    const history: AiMessage[] = [
      ...this.aiHistory,
      { role: "user", content: prompt },
    ];
    const toolEntries = new Map<AiToolCall, number>();
    try {
      const result = await runAiAgent(
        config,
        this.path,
        history,
        (status) => {
          this.aiStatus = status;
        },
        (event) => {
          if (event.phase === "start") {
            const id = ++this.aiEntrySeq;
            toolEntries.set(event.call, id);
            this.aiEntries = [
              ...this.aiEntries,
              {
                id,
                role: "tool",
                content: event.call.name,
                toolName: event.call.name,
                toolArguments: event.call.arguments,
                toolState: "running",
              },
            ];
            return;
          }
          const id = toolEntries.get(event.call);
          if (id === undefined) return;
          const state = event.phase === "error" ? "error" : "done";
          this.aiEntries = this.aiEntries.map((entry) =>
            entry.id === id
              ? {
                  ...entry,
                  toolState: state,
                  toolResult: event.result ?? "",
                }
              : entry,
          );
        },
      );
      this.aiHistory = result.history;
      const rebuilt = buildAiEntries(
        result.history,
        () => ++this.aiEntrySeq,
      );
      this.aiEntries = result.text
        ? rebuilt
        : [
            ...rebuilt,
            {
              id: ++this.aiEntrySeq,
              role: "assistant",
              content: "The model returned an empty response.",
            },
          ];
    } catch (e) {
      const text = typeof e === "string" ? e : String(e);
      this.app.notify(text, "error");
      this.aiEntries = [
        ...this.aiEntries,
        { id: ++this.aiEntrySeq, role: "assistant", content: `⚠ ${text}` },
      ];
    } finally {
      this.aiRunning = false;
      this.aiStatus = null;
    }
  }

  useAiCommitMessage(text: string) {
    const message = normalizeCommitMessage(text);
    if (!message) return;
    this.commitMessage = message;
    this.aiOpen = false;
    this.app.notify("Commit message inserted", "success");
  }

  async stage(path: string) {
    await this.run(() => git.stageFile(this.path, path));
    await this.refreshStatus();
  }

  async unstage(path: string) {
    await this.run(() => git.unstageFile(this.path, path));
    await this.refreshStatus();
  }

  async stageAll() {
    await this.run(() => git.stageAll(this.path));
    await this.refreshStatus();
  }

  async unstageAll() {
    await this.run(() => git.unstageAll(this.path));
    await this.refreshStatus();
  }

  async discard(path: string) {
    await this.run(
      () => git.discardFile(this.path, path),
      `Discarded changes in ${path}`,
    );
    await this.refreshStatus();
  }

  async commit(message: string, name?: string, email?: string) {
    const oid = await this.run(
      () => git.commit(this.path, message, name, email),
      "Commit created",
    );
    if (oid) {
      this.commitMessage = "";
      await this.refreshAll();
      return true;
    }
    return false;
  }

  async createBranch(name: string, checkout: boolean) {
    const ok = await this.runVoid(
      () => git.createBranch(this.path, name, checkout),
      `Branch ${name} created`,
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async deleteBranch(name: string, force: boolean) {
    const ok = await this.runVoid(
      () => git.deleteBranch(this.path, name, force),
      `Branch ${name} deleted`,
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async checkout(name: string) {
    const ok = await this.runVoid(
      () => git.checkoutBranch(this.path, name),
      `Switched to ${name}`,
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async merge(name: string) {
    const res = await this.run(() => git.mergeBranch(this.path, name));
    if (res !== null) {
      this.app.notify(res || `Merged ${name}`, "success");
      await this.refreshAll();
    }
    return res !== null;
  }

  async addRemote(name: string, url: string) {
    const ok = await this.runVoid(
      () => git.addRemote(this.path, name, url),
      `Remote ${name} added`,
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async removeRemote(name: string) {
    const ok = await this.runVoid(
      () => git.removeRemote(this.path, name),
      `Remote ${name} removed`,
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async fetch(remote: string | null) {
    const res = await this.run(() => git.fetch(this.path, remote));
    if (res !== null) {
      markFetched(this.path);
      this.app.notify(res || "Fetch complete", "success");
      await this.refreshAll();
    }
  }

  async pull(remote: string | null, branch: string | null = null) {
    const res = await this.run(() => git.pull(this.path, remote, branch));
    if (res !== null) {
      markFetched(this.path);
      this.app.notify(res || "Pull complete", "success");
      await this.refreshAll();
    }
  }

  async push(remote: string | null, branch: string | null, setUpstream: boolean) {
    const res = await this.run(() =>
      git.push(this.path, remote, branch, setUpstream),
    );
    if (res !== null) {
      this.app.notify(res || "Push complete", "success");
      await this.refreshAll();
    }
  }

  async createTag(name: string, message: string | null, target: string | null) {
    const ok = await this.runVoid(
      () => git.createTag(this.path, name, message, target),
      `Tag ${name} created`,
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async deleteTag(name: string) {
    const ok = await this.runVoid(
      () => git.deleteTag(this.path, name),
      `Tag ${name} deleted`,
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async createStash(message: string | null) {
    const ok = await this.run(
      () => git.createStash(this.path, message),
      "Changes stashed",
    );
    if (ok !== null) await this.refreshAll();
    return ok !== null;
  }

  async applyStash(index: number) {
    const ok = await this.runVoid(
      () => git.applyStash(this.path, index),
      "Stash applied",
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async popStash(index: number) {
    const ok = await this.runVoid(
      () => git.popStash(this.path, index),
      "Stash popped",
    );
    if (ok) await this.refreshAll();
    return ok;
  }

  async dropStash(index: number) {
    const ok = await this.runVoid(
      () => git.dropStash(this.path, index),
      "Stash dropped",
    );
    if (ok) await this.refreshAll();
    return ok;
  }
}

class AppStore {
  sessions = $state<RepoSession[]>([]);
  activeId = $state<string | null>(null);
  recents = $state<RecentProject[]>(sortRecents(loadRecents()));
  aiSettings = $state<AiSettings>(loadAiSettings());

  settingsOpen = $state(false);
  settingsTab = $state<SettingsTab>("appearance");

  errorLog = $state<ErrorLogEntry[]>(loadErrors());

  toast = $state<{ kind: "info" | "error" | "success"; text: string } | null>(
    null,
  );
  progress = $state<string | null>(null);
  private globalBusy = $state(false);
  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    void listen<{ operation: string; line: string }>(
      "git-progress",
      (event) => {
        this.progress = event.payload.line;
      },
    ).catch(() => {
      // Progress events are best-effort.
    });

    if (typeof window !== "undefined") {
      window.addEventListener("error", (event) => {
        this.logError(event.message || "Unknown error");
      });
      window.addEventListener("unhandledrejection", (event) => {
        const reason = event.reason;
        this.logError(
          typeof reason === "string"
            ? reason
            : (reason?.message ?? String(reason)),
        );
      });
    }
  }

  get active(): RepoSession | null {
    return this.sessions.find((s) => s.id === this.activeId) ?? null;
  }

  get busy(): boolean {
    return this.globalBusy || this.sessions.some((s) => s.busy);
  }

  // --- Active-session proxies (keep the component API stable) ---

  get repo() {
    return this.active?.repo ?? null;
  }
  get status() {
    return this.active?.status ?? [];
  }
  get staged() {
    return this.active?.staged ?? [];
  }
  get unstaged() {
    return this.active?.unstaged ?? [];
  }
  get log() {
    return this.active?.log ?? [];
  }
  get logHasMore() {
    return this.active?.logHasMore ?? false;
  }
  get branches() {
    return this.active?.branches ?? [];
  }
  get graph() {
    return this.active?.graph ?? null;
  }
  get graphIncludeRemote() {
    return this.active?.graphIncludeRemote ?? false;
  }
  get localBranches() {
    return this.active?.localBranches ?? [];
  }
  get remoteBranches() {
    return this.active?.remoteBranches ?? [];
  }
  get remotes() {
    return this.active?.remotes ?? [];
  }
  get tags() {
    return this.active?.tags ?? [];
  }
  get stashes() {
    return this.active?.stashes ?? [];
  }
  get user() {
    return this.active?.user ?? null;
  }
  get view(): ViewKey {
    return this.active?.view ?? "changes";
  }
  set view(v: ViewKey) {
    if (this.active) this.active.view = v;
  }
  get diff(): DiffTarget | null {
    return this.active?.diff ?? null;
  }
  set diff(d: DiffTarget | null) {
    if (this.active) this.active.diff = d;
  }
  get selectedCommit() {
    return this.active?.selectedCommit ?? null;
  }
  get commitMessage(): string {
    return this.active?.commitMessage ?? "";
  }
  set commitMessage(value: string) {
    if (this.active) this.active.commitMessage = value;
  }
  get aiOpen(): boolean {
    return this.active?.aiOpen ?? false;
  }
  get aiMode(): AiMode {
    return this.active?.aiMode ?? "chat";
  }
  get aiEntries(): AiEntry[] {
    return this.active?.aiEntries ?? [];
  }
  get aiDraft(): string {
    return this.active?.aiDraft ?? "";
  }
  set aiDraft(value: string) {
    if (this.active) this.active.aiDraft = value;
  }
  get aiRunning(): boolean {
    return this.active?.aiRunning ?? false;
  }
  get aiStatus(): string | null {
    return this.active?.aiStatus ?? null;
  }
  get aiConfig(): AiConfig {
    return resolveAiConfig(this.aiSettings, this.active?.path ?? null);
  }
  get aiConfigured(): boolean {
    return isAiConfigured(this.aiConfig);
  }

  refreshAll() {
    return this.active?.refreshAll();
  }
  loadMoreLog() {
    return this.active?.loadMoreLog();
  }
  saveUserConfig(name: string, email: string, global: boolean) {
    return this.active?.saveUserConfig(name, email, global);
  }
  loadWorkingDiff(path: string, staged: boolean) {
    return this.active?.loadWorkingDiff(path, staged);
  }
  loadCommit(commit: CommitInfo) {
    return this.active?.loadCommit(commit);
  }
  openAi(mode: AiMode = "chat") {
    return this.active?.openAi(mode);
  }
  closeAi() {
    return this.active?.closeAi();
  }
  resetAi() {
    return this.active?.resetAi();
  }
  sendAi() {
    return this.active?.sendAi();
  }
  useAiCommitMessage(text: string) {
    return this.active?.useAiCommitMessage(text);
  }
  setGraphRemote(includeRemote: boolean) {
    return this.active?.setGraphRemote(includeRemote);
  }
  stage(path: string) {
    return this.active?.stage(path);
  }
  unstage(path: string) {
    return this.active?.unstage(path);
  }
  stageAll() {
    return this.active?.stageAll();
  }
  unstageAll() {
    return this.active?.unstageAll();
  }
  discard(path: string) {
    return this.active?.discard(path);
  }
  commit(message: string) {
    return this.active?.commit(message);
  }
  createBranch(name: string, checkout: boolean) {
    return this.active?.createBranch(name, checkout);
  }
  deleteBranch(name: string, force: boolean) {
    return this.active?.deleteBranch(name, force);
  }
  checkout(name: string) {
    return this.active?.checkout(name);
  }
  merge(name: string) {
    return this.active?.merge(name);
  }
  addRemote(name: string, url: string) {
    return this.active?.addRemote(name, url);
  }
  removeRemote(name: string) {
    return this.active?.removeRemote(name);
  }
  fetch(remote: string | null) {
    return this.active?.fetch(remote);
  }
  pull(remote: string | null, branch: string | null = null) {
    return this.active?.pull(remote, branch);
  }
  push(remote: string | null, branch: string | null, setUpstream: boolean) {
    return this.active?.push(remote, branch, setUpstream);
  }
  createTag(name: string, message: string | null, target: string | null) {
    return this.active?.createTag(name, message, target);
  }
  deleteTag(name: string) {
    return this.active?.deleteTag(name);
  }
  createStash(message: string | null) {
    return this.active?.createStash(message);
  }
  applyStash(index: number) {
    return this.active?.applyStash(index);
  }
  popStash(index: number) {
    return this.active?.popStash(index);
  }
  dropStash(index: number) {
    return this.active?.dropStash(index);
  }

  // --- Session lifecycle ---

  private async runGlobal<T>(fn: () => Promise<T>): Promise<T | null> {
    this.globalBusy = true;
    try {
      return await fn();
    } catch (e) {
      this.notify(typeof e === "string" ? e : String(e), "error");
      return null;
    } finally {
      this.globalBusy = false;
      this.progress = null;
    }
  }

  private async addSession(info: RepoInfo) {
    const existing = this.sessions.find((s) => s.path === info.path);
    if (existing) {
      this.activeId = existing.id;
      await existing.refreshAll();
      void existing.autoFetch();
      this.touchRecent(info.path, info.name);
      this.notify(`Switched to ${info.name}`, "success");
      return;
    }
    const session = new RepoSession(this, info);
    this.sessions = [...this.sessions, session];
    this.activeId = session.id;
    await session.refreshAll();
    void session.autoFetch();
    this.touchRecent(info.path, info.name);
    this.notify(`Opened ${info.name}`, "success");
  }

  closeSession(id: string) {
    const idx = this.sessions.findIndex((s) => s.id === id);
    if (idx < 0) return;
    const remaining = this.sessions.filter((s) => s.id !== id);
    if (this.activeId === id) {
      const next = remaining[Math.min(idx, remaining.length - 1)];
      this.activeId = next?.id ?? null;
    }
    this.sessions = remaining;
  }

  closeRepo() {
    if (this.activeId) this.closeSession(this.activeId);
  }

  async openRepo(path: string) {
    const info = await this.runGlobal(() => git.openRepository(path));
    if (info) await this.addSession(info);
  }

  async initRepo(path: string) {
    const info = await this.runGlobal(() => git.initRepository(path));
    if (info) await this.addSession(info);
  }

  async cloneRepo(url: string, destination: string) {
    const info = await this.runGlobal(() =>
      git.cloneRepository(url, destination),
    );
    if (info) {
      markFetched(info.path);
      await this.addSession(info);
    }
  }

  notify(text: string, kind: "info" | "error" | "success" = "info") {
    if (kind === "error") this.logError(text);
    this.toast = { kind, text };
    if (this.toastTimer) clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => (this.toast = null), 4500);
  }

  logError(text: string) {
    const message = text.trim() || "Unknown error";
    const entry = createError(message, this.active?.repo?.path);
    this.errorLog = [entry, ...this.errorLog].slice(0, 200);
    persistErrors(this.errorLog);
  }

  clearErrorLog() {
    this.errorLog = [];
    persistErrors(this.errorLog);
  }

  openErrorLog() {
    this.settingsTab = "errors";
    this.settingsOpen = true;
  }

  openAiSettings() {
    this.settingsTab = "ai";
    this.settingsOpen = true;
  }

  saveAiConfig(scope: "app" | "repo", config: AiConfig) {
    const next: AiSettings = {
      app: this.aiSettings.app,
      repos: { ...this.aiSettings.repos },
    };
    if (scope === "app") {
      next.app = { ...config };
    } else if (this.active) {
      next.repos[this.active.path] = { ...config };
    }
    this.aiSettings = next;
    persistAiSettings(next);
  }

  clearRepoAiConfig(path: string) {
    const repos = { ...this.aiSettings.repos };
    delete repos[path];
    const next: AiSettings = { app: this.aiSettings.app, repos };
    this.aiSettings = next;
    persistAiSettings(next);
  }

  private touchRecent(path: string, name: string) {
    const existing = this.recents.find((r) => r.path === path);
    const entry: RecentProject = {
      path,
      name,
      pinned: existing?.pinned ?? false,
      lastOpened: Date.now(),
    };
    const next = this.recents.filter((r) => r.path !== path);
    next.push(entry);
    this.recents = sortRecents(next).slice(0, 30);
    persistRecents(this.recents);
  }

  togglePin(path: string) {
    this.recents = sortRecents(
      this.recents.map((r) => (r.path === path ? { ...r, pinned: !r.pinned } : r)),
    );
    persistRecents(this.recents);
  }

  removeRecent(path: string) {
    this.recents = this.recents.filter((r) => r.path !== path);
    persistRecents(this.recents);
  }
}

export const app = new AppStore();
