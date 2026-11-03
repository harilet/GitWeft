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
