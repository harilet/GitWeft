export interface ErrorLogEntry {
  id: string;
  time: number;
  message: string;
  repo?: string;
}

const KEY = "gitweft.errors";
const MAX = 200;

let counter = 0;

export function loadErrors(): ErrorLogEntry[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (e): e is ErrorLogEntry =>
          !!e &&
          typeof e.id === "string" &&
          typeof e.time === "number" &&
          typeof e.message === "string",
      )
      .slice(0, MAX);
  } catch {
    return [];
  }
}

export function persistErrors(list: ErrorLogEntry[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(list.slice(0, MAX)));
  } catch {
    // ignore storage errors
  }
}

export function createError(message: string, repo?: string): ErrorLogEntry {
  return {
    id: `${Date.now()}-${++counter}`,
    time: Date.now(),
    message,
    repo,
  };
}

/** The raw message behind an IPC rejection, for the error log. */
export function errorDetail(e: unknown): string {
  return (
    (typeof e === "string" ? e : e instanceof Error ? e.message : String(e))
      .trim() || "Unknown error"
  );
}

/**
 * Human-facing phrasing for common backend failures. The full technical text
 * stays in the error log (Settings → Error log); only certain mappings are
 * replaced, everything else passes through untouched.
 */
export function friendlyError(e: unknown): string {
  const detail = errorDetail(e).toLowerCase();
  if (
    /(authentication failed|could not read username|permission denied|publickey|credential|invalid username or password)/.test(
      detail,
    )
  ) {
    return "The remote rejected this attempt — check your SSH agent or credential helper, then try again.";
  }
  if (
    /(could not resolve host|connection (refused|reset|closed)|network is unreachable|timed? out|temporary failure in name resolution)/.test(
      detail,
    )
  ) {
    return "Could not reach the remote — check your connection, then try again.";
  }
  if (/(not a git repository|repository does not exist)/.test(detail)) {
    return "That folder is not a Git repository.";
  }
  if (/\balready exists\b/.test(detail)) {
    return "That name already exists here — pick another.";
  }
  return errorDetail(e);
}

export function formatTimestamp(ms: number): string {
  return new Date(ms).toLocaleString();
}

export function errorsToText(list: ErrorLogEntry[]): string {
  return list
    .map((e) => {
      const when = formatTimestamp(e.time);
      const where = e.repo ? ` [${e.repo}]` : "";
      return `${when}${where}\n${e.message}`;
    })
    .join("\n\n");
}
