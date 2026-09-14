/**
 * The About tab's source of truth is CHANGELOG.md, bundled into the build.
 *
 * Notes for the version you are running should be readable on a machine that is
 * offline, behind a proxy, or looking at a build whose tag was never pushed, so
 * they travel with the binary rather than coming from the GitHub API.
 *
 * That makes the file a build artefact, which is why this parser is deliberately
 * narrow: it understands the four shapes the changelog actually uses and ignores
 * everything else, instead of pulling in a Markdown library to render text we
 * write ourselves.
 */

export type SpanStyle = "plain" | "bold" | "code"

export interface Span {
  text: string
  style: SpanStyle
}

export type BlockKind = "heading" | "para" | "bullet"

export interface Block {
  kind: BlockKind
  spans: Span[]
}

export interface Release {
  version: string
  /** Whatever followed the dash on the heading — not parsed into a Date, because
   *  it is only ever displayed. */
  date: string
  blocks: Block[]
}

/** `## 1.2.3 — 2026-09-11`, with an em dash or a plain hyphen, date optional. */
const RELEASE_HEADING = /^##\s+v?(\d+\.\d+\.\d+[^\s—-]*)\s*(?:[—-]\s*(.*))?$/

/** `**bold**` or `` `code` ``. Anything else is plain text. */
const INLINE = /\*\*([^*]+)\*\*|`([^`]+)`/g

/** Splits one line of text into styled runs. Unmatched `*` and backticks are
 *  left alone: half-typed emphasis should show as the author wrote it, not
 *  swallow the rest of the paragraph. */
export function inlineSpans(text: string): Span[] {
  const spans: Span[] = []
  let last = 0
  for (const m of text.matchAll(INLINE)) {
    const at = m.index
    if (at > last) spans.push({ text: text.slice(last, at), style: "plain" })
    spans.push(
      m[1] !== undefined ? { text: m[1], style: "bold" } : { text: m[2]!, style: "code" },
    )
    last = at + m[0].length
  }
  if (last < text.length) spans.push({ text: text.slice(last), style: "plain" })
  return spans
}

/**
 * Every release in the file, in the order it appears (newest first by convention).
 * The preamble above the first `##` is dropped — it addresses whoever edits the
 * changelog, not whoever reads the About tab.
 */
export function parseChangelog(source: string): Release[] {
  const releases: Release[] = []
  let current: Release | null = null
  // Paragraphs and bullets wrap across lines in the file; `pending` accumulates
  // the run of lines belonging to one block until a blank line or a new marker
  // closes it.
  let pending: { kind: BlockKind; text: string } | null = null

  const flush = () => {
    if (pending && current) current.blocks.push({ kind: pending.kind, spans: inlineSpans(pending.text) })
    pending = null
  }

  for (const raw of source.split(/\r?\n/)) {
    const line = raw.trim()

    const heading = RELEASE_HEADING.exec(line)
    if (heading) {
      flush()
      current = { version: heading[1]!, date: (heading[2] ?? "").trim(), blocks: [] }
      releases.push(current)
      continue
    }

    if (line === "") {
      flush()
      continue
    }
    if (!current) continue

    if (line.startsWith("### ")) {
      flush()
      pending = { kind: "heading", text: line.slice(4).trim() }
      flush()
      continue
    }
    if (line.startsWith("#")) {
      // A deeper or stray heading level the renderer has no style for.
      flush()
      continue
    }
    if (/^[-*]\s/.test(line)) {
      flush()
      pending = { kind: "bullet", text: line.slice(2).trim() }
      continue
    }
    // A continuation of the block above, or the start of a paragraph.
    if (pending) pending.text += " " + line
    else pending = { kind: "para", text: line }
  }
  flush()

  return releases
}

/** The entry for the running build, or null when the version was never written
 *  down — a dev build between releases, most often. */
export function releaseFor(releases: Release[], version: string): Release | null {
  return releases.find((r) => r.version === version) ?? null
}

export type Group =
  | { kind: "list"; items: Block[] }
  | { kind: "heading" | "para"; block: Block }

/**
 * Runs of adjacent bullets, collapsed into one group each.
 *
 * A bullet is parsed on its own because that is how it is written, but four
 * bullets in a row are one list — rendering them as four single-item lists would
 * have a screen reader announce "list, 1 item" four times.
 */
export function groupBlocks(blocks: Block[]): Group[] {
  const groups: Group[] = []
  for (const block of blocks) {
    if (block.kind !== "bullet") {
      groups.push({ kind: block.kind, block })
      continue
    }
    const last = groups.at(-1)
    if (last?.kind === "list") last.items.push(block)
    else groups.push({ kind: "list", items: [block] })
  }
  return groups
}
