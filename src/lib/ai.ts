import { invoke } from "@tauri-apps/api/core";
import { git, type BranchInfo, type CommitInfo, type FileStatus } from "./api";

export type AiProvider = "ollama" | "openai";
export type AiRole = "system" | "user" | "assistant" | "tool";

export interface AiConfig {
  provider: AiProvider;
  baseUrl: string;
  model: string;
  apiKey: string;
  temperature: number;
}

export interface AiSettings {
  app: AiConfig;
  repos: Record<string, AiConfig>;
}

export interface AiToolCall {
  id: string;
  name: string;
  arguments: string;
}

export interface AiMessage {
  role: AiRole;
  content: string;
  toolCalls?: AiToolCall[];
  toolCallId?: string;
  toolName?: string;
}

export interface AiChatResponse {
  content: string;
  toolCalls: AiToolCall[];
  finishReason: string;
}

export interface AiToolDef {
  type: "function";
  function: {
    name: string;
    description: string;
    parameters: Record<string, unknown>;
  };
}

export const AI_DEFAULTS: Record<AiProvider, AiConfig> = {
  ollama: {
    provider: "ollama",
    baseUrl: "http://localhost:11434",
    model: "llama3.2",
    apiKey: "",
    temperature: 0.2,
  },
  openai: {
    provider: "openai",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    apiKey: "",
    temperature: 0.2,
  },
};

export const AI_MODEL_SUGGESTIONS: Record<AiProvider, string[]> = {
  ollama: ["llama3.2", "llama3.1", "qwen2.5-coder", "mistral", "phi4"],
  openai: ["gpt-4o-mini", "gpt-4o", "gpt-4.1-mini", "gpt-4.1", "o4-mini"],
};

const AI_KEY = "gitweft.ai";
const MAX_TOOL_CHARS = 60_000;
const MAX_TOOL_ROUNDS = 8;

function clampTemperature(value: unknown): number {
  const n = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(n)) return 0.2;
  return Math.min(2, Math.max(0, n));
}

function sanitizeConfig(value: unknown, fallback: AiConfig): AiConfig {
  const raw = (value ?? {}) as Partial<AiConfig>;
  const provider =
    raw.provider === "ollama" || raw.provider === "openai"
      ? raw.provider
      : fallback.provider;
  const defaults = AI_DEFAULTS[provider];
  return {
    provider,
    baseUrl:
      typeof raw.baseUrl === "string" && raw.baseUrl.trim()
        ? raw.baseUrl.trim()
        : defaults.baseUrl,
    model:
      typeof raw.model === "string" && raw.model.trim()
        ? raw.model.trim()
        : defaults.model,
    apiKey: typeof raw.apiKey === "string" ? raw.apiKey : "",
    temperature: clampTemperature(raw.temperature),
  };
}

export function loadAiSettings(): AiSettings {
  try {
    const raw = localStorage.getItem(AI_KEY);
    if (!raw) {
      return { app: { ...AI_DEFAULTS.ollama }, repos: {} };
    }
    const parsed = JSON.parse(raw) as Partial<AiSettings>;
    const app = sanitizeConfig(parsed.app, AI_DEFAULTS.ollama);
    const repos: Record<string, AiConfig> = {};
    if (parsed.repos && typeof parsed.repos === "object") {
      for (const [path, config] of Object.entries(parsed.repos)) {
        repos[path] = sanitizeConfig(config, app);
      }
    }
    return { app, repos };
  } catch {
    return { app: { ...AI_DEFAULTS.ollama }, repos: {} };
  }
}

export function persistAiSettings(settings: AiSettings) {
  try {
    localStorage.setItem(AI_KEY, JSON.stringify(settings));
  } catch {
    // ignore storage errors
  }
}

export function resolveAiConfig(
  settings: AiSettings,
  repoPath: string | null,
): AiConfig {
  if (repoPath && settings.repos[repoPath]) return settings.repos[repoPath];
  return settings.app;
}

export function isAiConfigured(config: AiConfig): boolean {
  if (!config.model.trim() || !config.baseUrl.trim()) return false;
  return config.provider === "ollama" || !!config.apiKey.trim();
}

export function providerLabel(provider: AiProvider): string {
  return provider === "ollama" ? "Ollama" : "OpenAI";
}

const SYSTEM_PROMPT = `You are GitWeft's read-only repository assistant.

Rules:
- You may ONLY inspect the repository using the provided tools. You cannot stage, unstage, commit, discard, checkout, push, or modify anything.
- Never claim that a change was made. If the user asks for a write action, explain that you can only read and that they must perform it in the UI.
- Use tools to gather facts before answering. Do not invent commits, files, or diffs.
- For commit messages, inspect the staged changes first. If nothing is staged, inspect the unstaged changes and say so. Output only the commit message: a concise summary line, then optionally a blank line and a body. Do not wrap it in quotes or code fences.
- Be concise. Use Markdown only when it helps.`;

export const AI_TOOLS: AiToolDef[] = [
  {
    type: "function",
    function: {
      name: "get_repository_info",
      description:
        "Get the repository name, current branch, HEAD commit, and whether the working tree is dirty.",
      parameters: { type: "object", properties: {}, required: [] },
    },
  },
  {
    type: "function",
    function: {
      name: "get_status",
      description: "List staged and unstaged files in the working tree with status codes.",
      parameters: { type: "object", properties: {}, required: [] },
    },
  },
  {
    type: "function",
    function: {
      name: "list_commits",
      description: "List recent commits, newest first.",
      parameters: {
        type: "object",
        properties: {
          limit: {
            type: "integer",
            minimum: 1,
            maximum: 100,
            description: "How many commits to return (default 20).",
          },
          skip: {
            type: "integer",
            minimum: 0,
            description: "How many commits to skip for pagination.",
          },
        },
        required: [],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "get_commit",
      description: "Show the full patch for one commit, identified by its full SHA or short SHA.",
      parameters: {
        type: "object",
        properties: {
          oid: { type: "string", description: "Commit SHA (full or short)." },
        },
        required: ["oid"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "get_staged_diff",
      description: "Show the unified diff of all staged changes.",
      parameters: { type: "object", properties: {}, required: [] },
    },
  },
  {
    type: "function",
    function: {
      name: "get_unstaged_diff",
      description: "Show the unified diff of all unstaged working-tree changes.",
      parameters: { type: "object", properties: {}, required: [] },
    },
  },
  {
    type: "function",
    function: {
      name: "get_file_diff",
      description: "Show the unified diff for a single file, staged or unstaged.",
      parameters: {
        type: "object",
        properties: {
          path: { type: "string", description: "Repository-relative file path." },
          staged: {
            type: "boolean",
            description: "True for the staged index diff, false for the working tree.",
          },
        },
        required: ["path", "staged"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "list_branches",
      description: "List local and remote branches with upstream and ahead/behind counts.",
      parameters: { type: "object", properties: {}, required: [] },
    },
  },
];

const TOOL_LABELS: Record<string, string> = {
  get_repository_info: "Reading repository info",
  get_status: "Reading working tree status",
  list_commits: "Browsing commits",
  get_commit: "Reading commit",
  get_staged_diff: "Reading staged changes",
  get_unstaged_diff: "Reading unstaged changes",
  get_file_diff: "Reading file diff",
  list_branches: "Reading branches",
};

function truncate(text: string, max = MAX_TOOL_CHARS): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max)}\n… (truncated ${text.length - max} characters)`;
}

function parseArgs(raw: string): Record<string, unknown> {
  if (!raw.trim()) return {};
  const value = JSON.parse(raw);
  return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

function clampInt(value: unknown, min: number, max: number, fallback: number): number {
  const n = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, Math.trunc(n)));
}

function formatCommit(commit: CommitInfo): string {
  const date = new Date(commit.time * 1000).toISOString();
  const body = commit.message.trim();
  return `${commit.short} (${commit.oid})\n  ${commit.summary || "(no summary)"}\n  ${commit.author} <${commit.email}> · ${date}\n  ${body.replace(/\n/g, "\n  ")}`;
}

function formatStatus(files: FileStatus[]): string {
  if (files.length === 0) return "Working tree is clean.";
  return files
    .map((file) => {
      const scope = [
        file.staged ? "staged" : null,
        file.unstaged ? "unstaged" : null,
      ]
        .filter(Boolean)
        .join(", ");
      return `${file.path} [${scope || "unchanged"}] index=${file.index} worktree=${file.worktree}`;
    })
    .join("\n");
}

function formatBranch(branch: BranchInfo): string {
  const parts = [branch.isRemote ? "remote" : "local"];
  if (branch.isHead) parts.push("HEAD");
  if (branch.merged) parts.push("merged");
  if (branch.upstream) parts.push(`upstream=${branch.upstream}`);
  parts.push(`ahead=${branch.ahead}`, `behind=${branch.behind}`);
  return `${branch.name} (${parts.join(", ")})`;
}

async function executeTool(
  repository: string,
  name: string,
  rawArgs: string,
): Promise<string> {
  let args: Record<string, unknown>;
  try {
    args = parseArgs(rawArgs);
  } catch {
    return "Error: tool arguments were not valid JSON.";
  }

  switch (name) {
    case "get_repository_info": {
      const info = await git.getRepositoryInfo(repository);
      return [
        `name: ${info.name}`,
        `path: ${info.path}`,
        `branch: ${info.branch}`,
        `head: ${info.head || "(unborn)"}`,
        `dirty: ${info.dirty}`,
        `detached: ${info.detached}`,
      ].join("\n");
    }
    case "get_status": {
      const files = await git.getStatus(repository);
      return formatStatus(files);
    }
    case "list_commits": {
      const limit = clampInt(args.limit, 1, 100, 20);
      const skip = clampInt(args.skip, 0, 100_000, 0);
      const commits = await git.getLog(repository, limit, skip);
      return commits.length
        ? commits.map(formatCommit).join("\n\n")
        : "No commits found.";
    }
    case "get_commit": {
      const oid = typeof args.oid === "string" ? args.oid.trim() : "";
      if (!oid) return "Error: oid is required.";
      const patch = await git.getCommitDiff(repository, oid);
      return patch.trim()
        ? truncate(patch)
        : `Commit ${oid} has an empty diff.`;
    }
    case "get_staged_diff": {
      const patch = await git.getWorktreeDiff(repository, null, true);
      return patch.trim() ? truncate(patch) : "No staged changes.";
    }
    case "get_unstaged_diff": {
      const patch = await git.getWorktreeDiff(repository, null, false);
      return patch.trim() ? truncate(patch) : "No unstaged changes.";
    }
    case "get_file_diff": {
      const path = typeof args.path === "string" ? args.path.trim() : "";
      if (!path) return "Error: path is required.";
      const staged = args.staged === true;
      const patch = await git.getWorktreeDiff(repository, path, staged);
      if (!patch.trim()) {
        return `No ${staged ? "staged" : "unstaged"} changes for ${path}.`;
      }
      return truncate(`File: ${path}\n\n${patch}`);
    }
    case "list_branches": {
      const branches = await git.listBranches(repository);
      return branches.length
        ? branches.map(formatBranch).join("\n")
        : "No branches found.";
    }
    default:
      return `Error: unknown tool "${name}".`;
  }
}

async function chat(config: AiConfig, messages: AiMessage[]): Promise<AiChatResponse> {
  return invokeAiChat({
    provider: config.provider,
    baseUrl: config.baseUrl,
    apiKey: config.apiKey,
    model: config.model,
    temperature: config.temperature,
    messages,
    tools: AI_TOOLS,
  });
}

function invokeAiChat(request: {
  provider: AiProvider;
  baseUrl: string;
  apiKey: string;
  model: string;
  temperature: number;
  messages: AiMessage[];
  tools: AiToolDef[];
}): Promise<AiChatResponse> {
  return invoke<AiChatResponse>("ai_chat", { request });
}

export interface AiRunResult {
  text: string;
  history: AiMessage[];
}

export function listAiModels(
  config: Pick<AiConfig, "provider" | "baseUrl" | "apiKey">,
): Promise<string[]> {
  return invoke<string[]>("ai_list_models", {
    request: {
      provider: config.provider,
      baseUrl: config.baseUrl.trim(),
      apiKey: config.apiKey,
    },
  });
}

export async function runAiAgent(
  config: AiConfig,
  repository: string,
  history: AiMessage[],
  onStatus?: (status: string) => void,
): Promise<AiRunResult> {
  const messages: AiMessage[] = [...history];

  for (let round = 0; round < MAX_TOOL_ROUNDS; round++) {
    onStatus?.(round === 0 ? "Thinking…" : "Reviewing tool results…");
    const response = await chat(config, [
      { role: "system", content: SYSTEM_PROMPT },
      ...messages,
    ]);

    if (response.toolCalls.length === 0) {
      messages.push({ role: "assistant", content: response.content });
      return { text: response.content.trim(), history: messages };
    }

    messages.push({
      role: "assistant",
      content: response.content,
      toolCalls: response.toolCalls,
    });

    for (const call of response.toolCalls) {
      onStatus?.(TOOL_LABELS[call.name] ?? `Running ${call.name}`);
      let result: string;
      try {
        result = await executeTool(repository, call.name, call.arguments);
      } catch (e) {
        result = `Error: ${typeof e === "string" ? e : String(e)}`;
      }
      messages.push({
        role: "tool",
        content: result,
        toolCallId: call.id,
        toolName: call.name,
      });
    }
  }

  throw new Error("The model requested too many tool calls. Try a more specific question.");
}

export const COMMIT_PROMPT =
  "Write a commit message for the staged changes in this repository. Inspect the staged diff first. If nothing is staged, inspect the unstaged changes and tell me that nothing is staged.";

export function normalizeCommitMessage(text: string): string {
  let value = text.trim();
  const fenced = value.match(/^```[a-zA-Z0-9_-]*\s*\n?([\s\S]*?)\n?```$/);
  if (fenced) value = fenced[1].trim();
  value = value.replace(/^commit message:\s*/i, "").trim();
  if (
    value.length > 1 &&
    ((value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'")))
  ) {
    value = value.slice(1, -1).trim();
  }
  return value;
}
