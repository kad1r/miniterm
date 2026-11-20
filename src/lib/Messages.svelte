<script lang="ts">
  import { onMount } from "svelte"
  import { promptHistory } from "../ipc"
  import {
    EMPTY_FILTER, applyFilter, enrich, formatDuration, formatTokens, formatWhen, modelName,
    modelStats, projectsOf, summarize, usageTotal, type Entry, type Filter, type SortKey,
  } from "../store/messages"
  import type { Label } from "../store/promptScore"
  import { notify } from "../store/app.svelte"
  import { locale, t } from "../i18n/locale.svelte"
  import type { MessageKey } from "../i18n/messages"

  /** Rows rendered per step: the full history can run into thousands. */
  const PAGE = 150

  let entries = $state.raw<Entry[]>([])
  let loading = $state(true)
  let error = $state<string | null>(null)
  let filter = $state<Filter>({ ...EMPTY_FILTER })
  let shown = $state(PAGE)
  let openKey = $state<string | null>(null)
  let tab = $state<"list" | "models">("list")

  const visible = $derived(applyFilter(entries, filter))
  const summary = $derived(summarize(visible))
  const projects = $derived(projectsOf(entries))
  /** Every model ever used, for the filter. */
  const allModels = $derived(modelStats(entries))
  /** The analysis follows the other filters, so a project's model mix shows too. */
  const modelRows = $derived(modelStats(visible))

  function showModel(model: string) {
    patch({ model })
    tab = "list"
  }

  const day = (ms: number) => (ms ? new Date(ms).toLocaleDateString(locale.current) : "—")
  const sessionTitle = $derived(
    filter.sessionId === null
      ? null
      : (entries.find((e) => e.sessionId === filter.sessionId)?.title ?? filter.sessionId.slice(0, 8)),
  )

  const key = (e: Entry) => `${e.sessionId}:${e.timeMs}`

  async function load() {
    loading = true
    error = null
    try {
      entries = (await promptHistory()).map(enrich)
    } catch (e) {
      error = String(e)
    } finally {
      loading = false
    }
  }

  onMount(() => {
    // Same reason as the board: the hidden terminal must not keep the keys.
    ;(document.activeElement as HTMLElement | null)?.blur()
    void load()
  })

  function patch(p: Partial<Filter>) {
    filter = { ...filter, ...p }
    shown = PAGE
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text)
      notify(t("messages.copied"))
    } catch {
      notify(t("messages.copyFailed"), "error")
    }
  }

  const LABEL_KEY: Record<Label, "messages.poor" | "messages.fair" | "messages.good"> = {
    Poor: "messages.poor",
    Fair: "messages.fair",
    Good: "messages.good",
  }

  const SORTS: Record<SortKey, MessageKey> = {
    newest: "messages.sort.newest",
    oldest: "messages.sort.oldest",
    score: "messages.sort.score",
    tokens: "messages.sort.tokens",
    duration: "messages.sort.duration",
  }
</script>

<section class="messages">
  <nav class="tabs">
    <button class:on={tab === "list"} onclick={() => (tab = "list")}>{t("messages.tab.list")}</button>
    <button class:on={tab === "models"} onclick={() => (tab = "models")}>{t("messages.tab.models")}</button>
  </nav>
  <header>
    <div class="stats">
      <div class="stat">
        <span class="k">{t("messages.count")}</span>
        <span class="v">{summary.count}</span>
      </div>
      <div class="stat">
        <span class="k">{t("messages.avgScore")}</span>
        <span class="v">{summary.avgScore ?? "—"}<small>/100</small></span>
      </div>
      <div class="stat">
        <span class="k">{t("messages.totalTokens")}</span>
        <span class="v">{formatTokens(summary.totalTokens)}</span>
      </div>
      <div class="stat">
        <span class="k">{t("messages.medianDuration")}</span>
        <span class="v">{formatDuration(summary.medianDurationMs)}</span>
      </div>
    </div>

    <div class="toolbar">
      <input
        class="search"
        type="search"
        placeholder={t("messages.search")}
        value={filter.query}
        oninput={(e) => patch({ query: e.currentTarget.value })}
      />
      <select
        value={filter.project ?? ""}
        onchange={(e) => patch({ project: e.currentTarget.value || null, sessionId: null })}
        aria-label={t("messages.project")}
      >
        <option value="">{t("messages.allProjects")}</option>
        {#each projects as p (p.name)}
          <option value={p.name}>{p.name} ({p.count})</option>
        {/each}
      </select>
      <select
        value={filter.model ?? ""}
        onchange={(e) => patch({ model: e.currentTarget.value || null })}
        aria-label={t("messages.model")}
      >
        <option value="">{t("messages.allModels")}</option>
        {#each allModels as m (m.model)}
          <option value={m.model}>{modelName(m.model)}</option>
        {/each}
      </select>
      <select
        value={filter.sort}
        onchange={(e) => patch({ sort: e.currentTarget.value as SortKey })}
        aria-label={t("messages.sort")}
      >
        {#each Object.entries(SORTS) as [s, label] (s)}
          <option value={s}>{t(label)}</option>
        {/each}
      </select>
      <button class="ghost" onclick={() => void load()} disabled={loading}>
        {t("messages.refresh")}
      </button>
    </div>

    {#if sessionTitle !== null}
      <div class="chips">
        <button class="chip" onclick={() => patch({ sessionId: null })} title={t("messages.clearSession")}>
          {t("messages.session")}: {sessionTitle} ✕
        </button>
      </div>
    {/if}
  </header>

  <div class="list">
    {#if tab === "models" && !loading && !error}
      {#if modelRows.length === 0}
        <p class="empty">{t("messages.noMatch")}</p>
      {:else}
        <div class="models">
          {#each modelRows as m (m.model)}
            <button class="model" onclick={() => showModel(m.model)} title={t("messages.modelShowPrompts")}>
              <div class="model-head">
                <strong>{modelName(m.model)}</strong>
                <code>{m.model}</code>
                <span class="pct">{(m.share * 100).toFixed(1)}%</span>
              </div>
              <span class="track wide"><span class="fill" style:width="{m.share * 100}%"></span></span>
              <dl class="model-grid">
                <dt>{t("messages.modelPrompts")}</dt><dd>{m.prompts}</dd>
                <dt>{t("messages.modelCalls")}</dt><dd>{m.calls} / {m.subagentCalls}</dd>
                <dt>{t("messages.output")}</dt><dd>{formatTokens(m.outputTokens)}</dd>
                <dt>{t("messages.cacheRead")}</dt><dd>{formatTokens(m.cacheReadTokens)}</dd>
                <dt>{t("messages.cacheWrite")}</dt><dd>{formatTokens(m.cacheWriteTokens)}</dd>
                <dt>{t("messages.total")}</dt><dd>{formatTokens(m.totalTokens)}</dd>
                <dt>{t("messages.avgScore")}</dt><dd>{m.avgScore ?? "—"}</dd>
                <dt>{t("messages.medianDuration")}</dt><dd>{formatDuration(m.medianDurationMs)}</dd>
                <dt>{t("messages.modelUsed")}</dt><dd>{day(m.firstUsedMs)} – {day(m.lastUsedMs)}</dd>
              </dl>
            </button>
          {/each}
          <p class="hint">{t("messages.modelNote")}</p>
        </div>
      {/if}
    {:else if loading && entries.length === 0}
      <p class="empty">{t("messages.loading")}</p>
    {:else if error}
      <p class="empty err">{t("messages.loadFailed", { error })}</p>
    {:else if visible.length === 0}
      <p class="empty">{entries.length === 0 ? t("messages.none") : t("messages.noMatch")}</p>
    {:else}
      {#each visible.slice(0, shown) as e (key(e))}
        {@const open = openKey === key(e)}
        <article class="row" class:open>
          <button class="summary" onclick={() => (openKey = open ? null : key(e))} aria-expanded={open}>
            <span
              class="score {e.score?.label.toLowerCase() ?? 'none'}"
              title={e.score ? t(LABEL_KEY[e.score.label]) : t("messages.unscored")}
            >{e.score?.overall ?? "—"}</span>
            <span class="main">
              <span class="text">{e.text}{e.images ? ` 🖼×${e.images}` : ""}</span>
              <span class="meta">
                <span>{formatWhen(e.timeMs, locale.current)}</span>
                <span class="proj">{e.project}</span>
                {#if e.model}<span>{modelName(e.model)}</span>{/if}
                <span class="title">{e.title ?? e.sessionId.slice(0, 8)}</span>
              </span>
            </span>
            <span class="nums">
              <span title={t("messages.totalTokens")}>{formatTokens(e.totalTokens)} tok</span>
              <span title={t("messages.duration")}>{formatDuration(e.durationMs)}</span>
            </span>
          </button>

          {#if open}
            <div class="detail">
              <pre class="full">{e.text}</pre>
              <div class="actions">
                <button class="ghost" onclick={() => void copy(e.text)}>{t("messages.copy")}</button>
                <button class="ghost" onclick={() => patch({ sessionId: e.sessionId, project: null })}>
                  {t("messages.showSession")}
                </button>
              </div>

              <div class="grid">
                <div class="block">
                  <h4>{t("messages.scoreTitle")}</h4>
                  {#if e.score}
                    {#each [
                      ["messages.clarity", e.score.clarity],
                      ["messages.context", e.score.context],
                      ["messages.specificity", e.score.specificity],
                      ["messages.verifiability", e.score.verifiability],
                    ] as const as [label, v] (label)}
                      <div class="bar">
                        <span>{t(label)}</span>
                        <span class="track"><span class="fill" style:width="{(v / 25) * 100}%"></span></span>
                        <span class="n">{v}/25</span>
                      </div>
                    {/each}
                    <p class="hint">
                      <strong class="score-inline {e.score.label.toLowerCase()}">
                        {e.score.overall}/100 · {t(LABEL_KEY[e.score.label])}
                      </strong>
                      {#if e.score.label !== "Good"}— {e.score.hint}{/if}
                    </p>
                  {:else}
                    <p class="hint">{t("messages.unscoredHint")}</p>
                  {/if}
                </div>

                <div class="block">
                  <h4>{t("messages.tokensTitle")}</h4>
                  <dl>
                    <dt>{t("messages.input")}</dt><dd>{e.inputTokens.toLocaleString(locale.current)}</dd>
                    <dt>{t("messages.output")}</dt><dd>{e.outputTokens.toLocaleString(locale.current)}</dd>
                    <dt>{t("messages.cacheRead")}</dt><dd>{e.cacheReadTokens.toLocaleString(locale.current)}</dd>
                    <dt>{t("messages.cacheWrite")}</dt><dd>{e.cacheWriteTokens.toLocaleString(locale.current)}</dd>
                    <dt>{t("messages.subagents")}</dt><dd>{e.subagentTokens.toLocaleString(locale.current)}</dd>
                    <dt class="sum">{t("messages.total")}</dt><dd class="sum">{e.totalTokens.toLocaleString(locale.current)}</dd>
                  </dl>
                </div>

                <div class="block">
                  <h4>{t("messages.timingTitle")}</h4>
                  <dl>
                    <dt>{t("messages.firstReply")}</dt><dd>{formatDuration(e.firstReplyMs)}</dd>
                    <dt>{t("messages.duration")}</dt><dd>{formatDuration(e.durationMs)}</dd>
                    <dt>{t("messages.apiCalls")}</dt><dd>{e.apiCalls}</dd>
                    <dt>{t("messages.toolCalls")}</dt><dd>{e.toolCalls}</dd>
                  </dl>
                  {#if e.models.length}
                    <h4 class="sub">{t("messages.modelsUsed")}</h4>
                    <dl>
                      {#each e.models as m (m.model + m.subagent)}
                        <dt>{modelName(m.model)}{m.subagent ? ` · ${t("messages.subagent")}` : ""}</dt>
                        <dd>{m.calls}× · {formatTokens(usageTotal(m))}</dd>
                      {/each}
                    </dl>
                  {/if}
                </div>

                <div class="block">
                  <h4>{t("messages.sessionTitle")}</h4>
                  <dl>
                    <dt>{t("messages.title")}</dt><dd>{e.title ?? "—"}</dd>
                    <dt>ID</dt><dd class="mono">{e.sessionId}</dd>
                    <dt>{t("messages.folder")}</dt><dd class="mono">{e.cwd ?? "—"}</dd>
                    <dt>{t("messages.branch")}</dt><dd>{e.branch ?? "—"}</dd>
                    <dt>{t("messages.sent")}</dt><dd>{new Date(e.timeMs).toLocaleString(locale.current)}</dd>
                  </dl>
                </div>
              </div>
            </div>
          {/if}
        </article>
      {/each}
      {#if visible.length > shown}
        <button class="more ghost" onclick={() => (shown += PAGE)}>
          {t("messages.more", { n: visible.length - shown })}
        </button>
      {/if}
    {/if}
  </div>
</section>

<style>
  .messages {
    display: flex;
    flex-direction: column;
    height: 100%;
    font-size: calc(13px * var(--font-scale, 1));
  }
  header {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 16px 12px;
    border-bottom: 1px solid var(--border);
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 8px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
  }
  .stat .k {
    color: var(--text-3);
    font-size: calc(11px * var(--font-scale, 1));
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .stat .v {
    color: var(--text-1);
    font-family: var(--font-mono);
    font-size: calc(18px * var(--font-scale, 1));
    font-variant-numeric: tabular-nums;
  }
  .stat small {
    color: var(--text-3);
    font-size: 0.6em;
  }
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .search {
    flex: 1 1 220px;
    min-width: 0;
  }
  input,
  select {
    padding: 6px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-1);
    font: inherit;
  }
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }
  .ghost {
    padding: 6px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-2);
    font: inherit;
    cursor: pointer;
  }
  .ghost:hover:not(:disabled) {
    color: var(--text-1);
    border-color: var(--text-3);
  }
  .ghost:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .chips {
    display: flex;
  }
  .chip {
    padding: 3px 10px;
    border: 1px solid var(--accent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--text-1);
    font: inherit;
    font-size: calc(12px * var(--font-scale, 1));
    cursor: pointer;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 16px 32px;
  }
  .empty {
    margin: 32px 0;
    color: var(--text-2);
    text-align: center;
  }
  .empty.err {
    color: var(--err);
  }
  .row {
    border-bottom: 1px solid var(--border);
  }
  .row.open {
    background: var(--bg-elevated);
    border-radius: var(--radius);
  }
  .summary {
    display: flex;
    gap: 12px;
    align-items: center;
    width: 100%;
    padding: 10px 8px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .summary:hover .text {
    color: var(--text-1);
  }
  .score {
    flex: none;
    display: grid;
    place-items: center;
    width: 38px;
    height: 26px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: calc(12px * var(--font-scale, 1));
    font-variant-numeric: tabular-nums;
  }
  .score.good,
  .score-inline.good {
    color: var(--ok);
    background: color-mix(in srgb, var(--ok) 14%, transparent);
  }
  .score.fair,
  .score-inline.fair {
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 14%, transparent);
  }
  .score.poor,
  .score-inline.poor {
    color: var(--err);
    background: color-mix(in srgb, var(--err) 14%, transparent);
  }
  .score.none {
    color: var(--text-3);
    background: var(--bg-elevated);
  }
  .score-inline {
    padding: 1px 6px;
    border-radius: 4px;
  }
  .main {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .text {
    display: -webkit-box;
    overflow: hidden;
    color: var(--text-1);
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    white-space: pre-line;
    word-break: break-word;
  }
  .meta {
    display: flex;
    gap: 10px;
    min-width: 0;
    color: var(--text-3);
    font-size: calc(11.5px * var(--font-scale, 1));
    white-space: nowrap;
  }
  .meta .proj {
    color: var(--text-2);
  }
  .meta .title {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .nums {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    color: var(--text-2);
    font-family: var(--font-mono);
    font-size: calc(11.5px * var(--font-scale, 1));
    font-variant-numeric: tabular-nums;
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0 8px 14px 58px;
  }
  .full {
    max-height: 320px;
    margin: 0;
    overflow: auto;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-term);
    color: var(--text-1);
    font-family: var(--font-mono);
    font-size: calc(12px * var(--font-scale, 1));
    white-space: pre-wrap;
    word-break: break-word;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 10px;
  }
  .block {
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-app);
  }
  h4 {
    margin: 0 0 8px;
    color: var(--text-3);
    font-size: calc(11px * var(--font-scale, 1));
    font-weight: 500;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .bar {
    display: grid;
    grid-template-columns: 9em 1fr 3.2em;
    gap: 8px;
    align-items: center;
    margin-bottom: 5px;
    color: var(--text-2);
    font-size: calc(12px * var(--font-scale, 1));
  }
  .track {
    height: 6px;
    overflow: hidden;
    border-radius: 3px;
    background: var(--border);
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .n {
    font-family: var(--font-mono);
    text-align: right;
  }
  .hint {
    margin: 8px 0 0;
    color: var(--text-2);
    font-size: calc(12px * var(--font-scale, 1));
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 0;
    font-size: calc(12px * var(--font-scale, 1));
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
    color: var(--text-1);
    font-family: var(--font-mono);
    text-align: right;
  }
  dd.mono {
    font-size: calc(11px * var(--font-scale, 1));
  }
  .sum {
    padding-top: 4px;
    border-top: 1px solid var(--border);
    font-weight: 600;
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    padding: 10px 14px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--text-2);
    font: inherit;
    cursor: pointer;
  }
  .tabs button.on {
    border-bottom-color: var(--accent);
    color: var(--text-1);
  }
  .models {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 10px;
    padding-top: 8px;
  }
  .models .hint {
    grid-column: 1 / -1;
  }
  .model {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .model:hover {
    border-color: var(--border-strong);
  }
  .model-head {
    display: flex;
    gap: 8px;
    align-items: baseline;
    min-width: 0;
  }
  .model-head strong {
    color: var(--text-1);
    font-size: calc(15px * var(--font-scale, 1));
  }
  .model-head code {
    overflow: hidden;
    color: var(--text-3);
    font-size: calc(11px * var(--font-scale, 1));
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pct {
    margin-left: auto;
    color: var(--accent);
    font-family: var(--font-mono);
  }
  .track.wide {
    display: block;
    height: 8px;
  }
  .model-grid {
    gap: 3px 12px;
  }
  h4.sub {
    margin-top: 12px;
  }
  .more {
    display: block;
    margin: 14px auto 0;
  }
</style>
