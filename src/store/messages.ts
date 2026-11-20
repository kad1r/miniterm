// The message log: prompts from Claude Code transcripts, scored with the same
// heuristic as the prompt-score mod, plus the filtering and figures the view shows.

import type { ModelUsage, PromptRecord } from "../ipc"
import { score, shouldSkip, type Score } from "./promptScore"

export interface Entry extends PromptRecord {
  /** Null for prompts the mod does not score (short, slash commands, "devam"). */
  score: Score | null
  totalTokens: number
  /** Last segment of the working directory. */
  project: string
}

export type SortKey = "newest" | "oldest" | "tokens" | "duration" | "score"

export interface Filter {
  query: string
  project: string | null
  sessionId: string | null
  /** Prompts that used this model anywhere, subagents included. */
  model: string | null
  sort: SortKey
}

export const EMPTY_FILTER: Filter = {
  query: "", project: null, sessionId: null, model: null, sort: "newest",
}

export function projectName(cwd: string | null): string {
  if (!cwd) return "—"
  return cwd.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || cwd
}

export function enrich(p: PromptRecord): Entry {
  return {
    ...p,
    score: shouldSkip(p.text) ? null : score(p.text),
    totalTokens:
      p.inputTokens + p.outputTokens + p.cacheReadTokens + p.cacheWriteTokens + p.subagentTokens,
    project: projectName(p.cwd),
  }
}

const SORTS: Record<SortKey, (a: Entry, b: Entry) => number> = {
  newest: (a, b) => b.timeMs - a.timeMs,
  oldest: (a, b) => a.timeMs - b.timeMs,
  tokens: (a, b) => b.totalTokens - a.totalTokens,
  duration: (a, b) => (b.durationMs ?? -1) - (a.durationMs ?? -1),
  // Unscored prompts sink to the bottom rather than counting as zero.
  score: (a, b) => (b.score?.overall ?? -1) - (a.score?.overall ?? -1),
}

export function applyFilter(entries: readonly Entry[], f: Filter): Entry[] {
  const q = f.query.trim().toLocaleLowerCase("tr-TR")
  const out = entries.filter(
    (e) =>
      (f.project === null || e.project === f.project) &&
      (f.sessionId === null || e.sessionId === f.sessionId) &&
      (f.model === null || e.models.some((m) => m.model === f.model)) &&
      (q === "" ||
        e.text.toLocaleLowerCase("tr-TR").includes(q) ||
        (e.title ?? "").toLocaleLowerCase("tr-TR").includes(q)),
  )
  return out.sort(SORTS[f.sort])
}

/** Projects by prompt count, most used first. */
export function projectsOf(entries: readonly Entry[]): { name: string; count: number }[] {
  const counts = new Map<string, number>()
  for (const e of entries) counts.set(e.project, (counts.get(e.project) ?? 0) + 1)
  return [...counts].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count)
}

export interface Summary {
  count: number
  totalTokens: number
  /** Over scored prompts only; null when none is scored. */
  avgScore: number | null
  /** Median, so one hour-long turn does not drown the rest. */
  medianDurationMs: number | null
}

export function summarize(entries: readonly Entry[]): Summary {
  const scores = entries.flatMap((e) => (e.score ? [e.score.overall] : []))
  const durations = entries
    .flatMap((e) => (e.durationMs === null ? [] : [e.durationMs]))
    .sort((a, b) => a - b)
  const mid = durations.length >> 1
  return {
    count: entries.length,
    totalTokens: entries.reduce((n, e) => n + e.totalTokens, 0),
    avgScore: scores.length ? Math.round(scores.reduce((a, b) => a + b, 0) / scores.length) : null,
    medianDurationMs: durations.length
      ? durations.length % 2
        ? durations[mid]
        : Math.round((durations[mid - 1] + durations[mid]) / 2)
      : null,
  }
}

/** 950 · 12.3K · 4.5M */
export function formatTokens(n: number): string {
  if (n < 1000) return String(n)
  if (n < 1_000_000) return `${trim(n / 1000)}K`
  return `${trim(n / 1_000_000)}M`
}

const trim = (x: number) => (x >= 100 ? Math.round(x).toString() : x.toFixed(1).replace(/\.0$/, ""))

/** 850ms · 12.4s · 3m 12s · 1h 5m */
export function formatDuration(ms: number | null): string {
  if (ms === null) return "—"
  if (ms < 1000) return `${ms}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1).replace(/\.0$/, "")}s`
  const s = Math.round(ms / 1000)
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`
}

/** Calendar date and time in the viewer's zone, e.g. "20.11.2026 16:51". */
export function formatWhen(timeMs: number, locale: string): string {
  const d = new Date(timeMs)
  return `${d.toLocaleDateString(locale)} ${d.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" })}`
}

/** `claude-opus-5-5` → `Opus 5.5`, `claude-3-5-haiku-20241022` → `Haiku 3.5`. */
export function modelName(id: string): string {
  const parts = id.replace(/^claude-/, "").replace(/-\d{8}$/, "").split("-")
  const family = parts.find((p) => /^[a-z]+$/i.test(p))
  const version = parts.filter((p) => /^\d+$/.test(p)).join(".")
  if (!family) return id
  const name = family[0].toUpperCase() + family.slice(1)
  return version ? `${name} ${version}` : name
}

export const usageTotal = (m: ModelUsage) =>
  m.inputTokens + m.outputTokens + m.cacheReadTokens + m.cacheWriteTokens

export interface ModelStat {
  model: string
  /** Prompts whose main conversation ran on this model. */
  prompts: number
  calls: number
  subagentCalls: number
  inputTokens: number
  outputTokens: number
  cacheReadTokens: number
  cacheWriteTokens: number
  totalTokens: number
  /** Of all tokens across every model, 0..1. */
  share: number
  /** Over prompts led by this model. */
  avgScore: number | null
  medianDurationMs: number | null
  firstUsedMs: number
  lastUsedMs: number
}

/** Per-model totals, most tokens first. */
export function modelStats(entries: readonly Entry[]): ModelStat[] {
  const by = new Map<string, ModelStat>()
  const led = new Map<string, Entry[]>()
  const stat = (model: string) => {
    let s = by.get(model)
    if (!s) {
      s = {
        model, prompts: 0, calls: 0, subagentCalls: 0, inputTokens: 0, outputTokens: 0,
        cacheReadTokens: 0, cacheWriteTokens: 0, totalTokens: 0, share: 0, avgScore: null,
        medianDurationMs: null, firstUsedMs: Infinity, lastUsedMs: -Infinity,
      }
      by.set(model, s)
    }
    return s
  }
  for (const e of entries) {
    if (e.model) {
      stat(e.model).prompts += 1
      const list = led.get(e.model)
      if (list) list.push(e)
      else led.set(e.model, [e])
    }
    for (const m of e.models) {
      const s = stat(m.model)
      if (m.subagent) s.subagentCalls += m.calls
      else s.calls += m.calls
      s.inputTokens += m.inputTokens
      s.outputTokens += m.outputTokens
      s.cacheReadTokens += m.cacheReadTokens
      s.cacheWriteTokens += m.cacheWriteTokens
      s.totalTokens += usageTotal(m)
      s.firstUsedMs = Math.min(s.firstUsedMs, e.timeMs)
      s.lastUsedMs = Math.max(s.lastUsedMs, e.timeMs)
    }
  }
  const all = [...by.values()].reduce((n, s) => n + s.totalTokens, 0)
  for (const s of by.values()) {
    s.share = all ? s.totalTokens / all : 0
    const own = summarize(led.get(s.model) ?? [])
    s.avgScore = own.avgScore
    s.medianDurationMs = own.medianDurationMs
    if (!Number.isFinite(s.firstUsedMs)) s.firstUsedMs = s.lastUsedMs = 0
  }
  return [...by.values()].sort((a, b) => b.totalTokens - a.totalTokens)
}
