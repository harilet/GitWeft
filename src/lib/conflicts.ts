export type ConflictChoice = "ours" | "theirs" | "base" | "both";

export interface ConflictBlock {
  id: number;
  startLine: number;
  oursStart: number;
  baseStart: number;
  theirsStart: number;
  oursLines: string[];
  baseLines: string[] | null;
  theirsLines: string[];
  oursLabel: string;
  theirsLabel: string;
  baseLabel: string;
}

export type ConflictSegment =
  | { kind: "text"; lines: string[] }
  | { kind: "conflict"; block: ConflictBlock };

export interface ParsedConflict {
  segments: ConflictSegment[];
  blocks: ConflictBlock[];
}

const START = /^<{7,}(?: (.*?))?\r?$/;
const BASE = /^\|{7,}(?: (.*?))?\r?$/;
const SEP = /^={7,}\r?$/;
const END = /^>{7,}(?: (.*?))?\r?$/;

function markerLabel(match: RegExpExecArray | null, fallback: string): string {
  const value = match?.[1]?.trim();
  return value || fallback;
}

/**
 * Split a working-tree file into plain text and conflict blocks. Both the
 * default (merge) and diff3 marker styles are supported.
 */
export function parseConflicts(content: string): ParsedConflict {
  const lines = content.split("\n");
  const segments: ConflictSegment[] = [];
  const blocks: ConflictBlock[] = [];

  let text: string[] = [];
  let i = 0;

  const flush = () => {
    if (text.length > 0) {
      segments.push({ kind: "text", lines: text });
      text = [];
    }
  };

  while (i < lines.length) {
    const start = START.exec(lines[i]);
    if (!start) {
      text.push(lines[i]);
      i++;
      continue;
    }

    flush();
    const block: ConflictBlock = {
      id: blocks.length,
      startLine: i + 1,
      oursStart: 0,
      baseStart: 0,
      theirsStart: 0,
      oursLines: [],
      baseLines: null,
      theirsLines: [],
      oursLabel: markerLabel(start, "ours"),
      theirsLabel: "theirs",
      baseLabel: "base",
    };

    i++;
    block.oursStart = i + 1;
    while (
      i < lines.length &&
      !BASE.test(lines[i]) &&
      !SEP.test(lines[i]) &&
      !END.test(lines[i])
    ) {
      block.oursLines.push(lines[i]);
      i++;
    }

    if (i < lines.length && BASE.test(lines[i])) {
      block.baseLabel = markerLabel(BASE.exec(lines[i]), "base");
      block.baseLines = [];
      i++;
      block.baseStart = i + 1;
      while (i < lines.length && !SEP.test(lines[i]) && !END.test(lines[i])) {
        block.baseLines.push(lines[i]);
        i++;
      }
    }

    if (i < lines.length && SEP.test(lines[i])) {
      i++;
    }

    block.theirsStart = i + 1;
    while (i < lines.length && !END.test(lines[i])) {
      block.theirsLines.push(lines[i]);
      i++;
    }

    if (i < lines.length && END.test(lines[i])) {
      block.theirsLabel = markerLabel(END.exec(lines[i]), "theirs");
      i++;
    }

    blocks.push(block);
    segments.push({ kind: "conflict", block });
  }

  flush();
  return { segments, blocks };
}

export function defaultChoices(
  blocks: ConflictBlock[],
): Record<number, ConflictChoice> {
  return Object.fromEntries(blocks.map((block) => [block.id, "ours"]));
}

export function buildResolved(
  segments: ConflictSegment[],
  choices: Record<number, ConflictChoice>,
): string {
  const out: string[] = [];
  for (const segment of segments) {
    if (segment.kind === "text") {
      out.push(...segment.lines);
      continue;
    }
    const block = segment.block;
    const choice = choices[block.id] ?? "ours";
    if (choice === "ours") {
      out.push(...block.oursLines);
    } else if (choice === "theirs") {
      out.push(...block.theirsLines);
    } else if (choice === "base") {
      out.push(...(block.baseLines ?? []));
    } else if (choice === "both") {
      out.push(...block.oursLines, ...block.theirsLines);
    }
  }
  return out.join("\n");
}
