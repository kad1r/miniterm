import { describe, expect, it } from "vitest"
import type { PromptRecord } from "../ipc"
import {
  EMPTY_FILTER, applyFilter, enrich, formatDuration, formatTokens, modelName, modelStats,
  projectName, projectsOf, summarize,
} from "./messages"

function rec(over: Partial<PromptRecord> = {}): PromptRecord {
  return {
    sessionId: "s1", title: "Başlık", cwd: "D:\\Dev\\miniterm", branch: "main",
    timestamp: "", timeMs: 0, text: "sidebar fontunu JetBrains Mono yap lütfen", images: 0,
    source: "typed", model: "m", inputTokens: 10, outputTokens: 20, cacheReadTokens: 30,
    cacheWriteTokens: 40, subagentTokens: 5, apiCalls: 1, toolCalls: 0, firstReplyMs: 100,
    durationMs: 1000, models: [], ...over,
  }
}

describe("enrich", () => {
  it("totals every token kind and names the project", () => {
    const e = enrich(rec())
    expect(e.totalTokens).toBe(105)
    expect(e.project).toBe("miniterm")
    expect(e.score?.overall).toBeGreaterThan(0)
  })

  it("leaves what the mod skips unscored", () => {
    expect(enrich(rec({ text: "evet" })).score).toBeNull()
    expect(enrich(rec({ text: "/model opus and more" })).score).toBeNull()
  })
})

describe("applyFilter", () => {
  const entries = [
    enrich(rec({ sessionId: "a", timeMs: 1, text: "eski mesaj burada yazıyor", durationMs: 5 })),
    enrich(rec({ sessionId: "b", timeMs: 2, cwd: "D:\\Other", text: "İSTANBUL hakkında bir soru", durationMs: null })),
    enrich(rec({ sessionId: "a", timeMs: 3, text: "evet", outputTokens: 9999 })),
  ]

  it("sorts newest first by default", () => {
    expect(applyFilter(entries, EMPTY_FILTER).map((e) => e.timeMs)).toEqual([3, 2, 1])
  })

  it("matches the query case-insensitively with Turkish casing", () => {
    expect(applyFilter(entries, { ...EMPTY_FILTER, query: "istanbul" }).map((e) => e.timeMs)).toEqual([2])
  })

  it("narrows by project and by session", () => {
    expect(applyFilter(entries, { ...EMPTY_FILTER, project: "Other" })).toHaveLength(1)
    expect(applyFilter(entries, { ...EMPTY_FILTER, sessionId: "a" })).toHaveLength(2)
  })

  it("puts unknown durations and unscored prompts last", () => {
    expect(applyFilter(entries, { ...EMPTY_FILTER, sort: "duration" }).at(-1)?.timeMs).toBe(2)
    expect(applyFilter(entries, { ...EMPTY_FILTER, sort: "score" }).at(-1)?.timeMs).toBe(3)
    expect(applyFilter(entries, { ...EMPTY_FILTER, sort: "tokens" })[0].timeMs).toBe(3)
  })
})

describe("summarize", () => {
  it("averages scored prompts only and takes the median duration", () => {
    const s = summarize([
      enrich(rec({ durationMs: 100 })),
      enrich(rec({ text: "evet", durationMs: 300 })),
      enrich(rec({ durationMs: 200 })),
      enrich(rec({ durationMs: null })),
    ])
    expect(s.count).toBe(4)
    expect(s.totalTokens).toBe(420)
    expect(s.avgScore).toBe(enrich(rec()).score?.overall)
    expect(s.medianDurationMs).toBe(200)
  })

  it("is empty-safe", () => {
    expect(summarize([])).toEqual({ count: 0, totalTokens: 0, avgScore: null, medianDurationMs: null })
  })
})

describe("formatting", () => {
  it("formats tokens", () => {
    expect([950, 1000, 12_345, 450_000, 4_560_000].map(formatTokens)).toEqual(["950", "1K", "12.3K", "450K", "4.6M"])
  })

  it("formats durations", () => {
    expect([null, 850, 12_400, 192_000, 3_900_000].map(formatDuration)).toEqual(["—", "850ms", "12.4s", "3m 12s", "1h 5m"])
  })

  it("names projects from either slash", () => {
    expect(projectName("C:/a/b/")).toBe("b")
    expect(projectName(null)).toBe("—")
  })

  it("groups projects by count", () => {
    const es = [rec(), rec(), rec({ cwd: "/x/y" })].map(enrich)
    expect(projectsOf(es)).toEqual([{ name: "miniterm", count: 2 }, { name: "y", count: 1 }])
  })
})

const use = (model: string, subagent: boolean, outputTokens: number, calls = 1) => ({
  model, subagent, calls, inputTokens: 0, outputTokens, cacheReadTokens: 0, cacheWriteTokens: 0,
})

describe("models", () => {
  it("names models from their ids", () => {
    expect(modelName("claude-opus-5-5")).toBe("Opus 5.5")
    expect(modelName("claude-3-5-haiku-20241022")).toBe("Haiku 3.5")
    expect(modelName("claude-fable-5-2")).toBe("Fable 5.2")
    expect(modelName("gpt-x")).toBe("Gpt")
  })

  it("totals per model, splits subagent calls and shares tokens", () => {
    const es = [
      rec({ timeMs: 1, model: "opus", models: [use("opus", false, 300, 3), use("haiku", true, 100, 2)] }),
      rec({ timeMs: 5, model: "opus", durationMs: 3000, models: [use("opus", false, 600)] }),
    ].map(enrich)
    const [opus, haiku] = modelStats(es)
    expect(opus).toMatchObject({ model: "opus", prompts: 2, calls: 4, outputTokens: 900, share: 0.9 })
    expect(opus.medianDurationMs).toBe(2000)
    expect([opus.firstUsedMs, opus.lastUsedMs]).toEqual([1, 5])
    expect(haiku).toMatchObject({ prompts: 0, calls: 0, subagentCalls: 2, avgScore: null })
  })

  it("filters prompts that used a model anywhere", () => {
    const es = [
      rec({ timeMs: 1, models: [use("opus", false, 1)] }),
      rec({ timeMs: 2, models: [use("opus", false, 1), use("haiku", true, 1)] }),
    ].map(enrich)
    expect(applyFilter(es, { ...EMPTY_FILTER, model: "haiku" }).map((e) => e.timeMs)).toEqual([2])
  })
})
