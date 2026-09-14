import { describe, expect, it } from "vitest"
import pkg from "../../package.json"
import { groupBlocks, inlineSpans, parseChangelog, releaseFor } from "./changelog"
import source from "../../CHANGELOG.md?raw"

const SAMPLE = `# Changelog

Editor-facing preamble that nobody running the app should see.

## 0.4.0 — 2026-09-11

### Change a workspace's grid layout

Right-click a workspace and pick **Change layout**. Every shell keeps
running, scrollback and all.

- The picker follows whether anything changed at all.
- A workspace at the limit can still be reshaped — 1×6, 2×3 and 6×1
  are all on offer.

## 0.3.0 - 2026-09-11

- ConPTY no longer sprays \`[?9001h\` into the pane.
`

describe("parseChangelog", () => {
  it("splits the file into releases, newest first", () => {
    const releases = parseChangelog(SAMPLE)
    expect(releases.map((r) => r.version)).toEqual(["0.4.0", "0.3.0"])
    expect(releases[0]!.date).toBe("2026-09-11")
  })

  it("accepts a plain hyphen as well as an em dash", () => {
    expect(parseChangelog(SAMPLE)[1]!.date).toBe("2026-09-11")
  })

  it("drops the preamble above the first release", () => {
    const text = parseChangelog(SAMPLE)
      .flatMap((r) => r.blocks)
      .map((b) => b.spans.map((s) => s.text).join(""))
      .join(" ")
    expect(text).not.toContain("preamble")
  })

  it("keeps headings, paragraphs and bullets apart", () => {
    const blocks = parseChangelog(SAMPLE)[0]!.blocks
    expect(blocks.map((b) => b.kind)).toEqual(["heading", "para", "bullet", "bullet"])
  })

  it("joins lines that wrap inside one block", () => {
    const blocks = parseChangelog(SAMPLE)[0]!.blocks
    const para = blocks[1]!.spans.map((s) => s.text).join("")
    expect(para).toBe("Right-click a workspace and pick Change layout. Every shell keeps running, scrollback and all.")
    const wrapped = blocks[3]!.spans.map((s) => s.text).join("")
    expect(wrapped).toBe("A workspace at the limit can still be reshaped — 1×6, 2×3 and 6×1 are all on offer.")
  })

  it("finds nothing in an empty file rather than throwing", () => {
    expect(parseChangelog("")).toEqual([])
  })
})

describe("inlineSpans", () => {
  it("marks bold runs and leaves the rest plain", () => {
    expect(inlineSpans("pick **Change layout** now")).toEqual([
      { text: "pick ", style: "plain" },
      { text: "Change layout", style: "bold" },
      { text: " now", style: "plain" },
    ])
  })

  it("marks code runs", () => {
    expect(inlineSpans("`clear` renders wrong")).toEqual([
      { text: "clear", style: "code" },
      { text: " renders wrong", style: "plain" },
    ])
  })

  it("leaves an unclosed marker as written", () => {
    expect(inlineSpans("2 ** 8 is a power")).toEqual([{ text: "2 ** 8 is a power", style: "plain" }])
  })
})

describe("groupBlocks", () => {
  it("collapses adjacent bullets into one list", () => {
    const groups = groupBlocks(parseChangelog(SAMPLE)[0]!.blocks)
    expect(groups.map((g) => g.kind)).toEqual(["heading", "para", "list"])
    expect(groups[2]!.kind === "list" && groups[2]!.items).toHaveLength(2)
  })

  it("starts a new list when a paragraph interrupts the bullets", () => {
    const parsed = parseChangelog("## 1.0.0 — x\n\n- a\n\nbreak\n\n- b\n")
    const groups = groupBlocks(parsed[0]!.blocks)
    expect(groups.map((g) => g.kind)).toEqual(["list", "para", "list"])
  })

  it("returns nothing for a release with no blocks", () => {
    expect(groupBlocks([])).toEqual([])
  })
})

describe("releaseFor", () => {
  const releases = parseChangelog(SAMPLE)

  it("matches the running version", () => {
    expect(releaseFor(releases, "0.3.0")?.version).toBe("0.3.0")
  })

  it("returns null for a version that was never written down", () => {
    expect(releaseFor(releases, "0.5.0")).toBeNull()
  })
})

describe("the shipped CHANGELOG.md", () => {
  const releases = parseChangelog(source)

  it("parses into releases", () => {
    expect(releases.length).toBeGreaterThan(0)
  })

  /** The About tab is blank for a release nobody wrote notes for, so the version
   *  in package.json has to be present before that version is ever cut. */
  it("has an entry for the version in package.json", () => {
    expect(releaseFor(releases, pkg.version)).not.toBeNull()
  })

  it("gives every release a date and some content", () => {
    for (const r of releases) {
      expect(r.date, r.version).not.toBe("")
      expect(r.blocks.length, r.version).toBeGreaterThan(0)
    }
  })
})
