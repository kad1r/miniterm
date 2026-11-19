import { describe, expect, it } from "vitest"
import {
  addCard, addColumn, addTag, allTags, boardOf, cardCount, columnColor, defaultBoard, dueState,
  findCard, formatDue, isoDay, moveCard, nextColor, PALETTE, removeCard, removeColumn,
  renameColumn, setColumnColor, shiftColumn, tagColor, updateCard,
} from "./board"
import type { Board } from "./types"

const name = (key: string) => key.toUpperCase()

const card = (id: string, tags: string[] = [], due: string | null = null) =>
  ({ id, title: id, note: "", tags, due })

function sample(): Board {
  return {
    columns: [
      { id: "a", name: "A", cards: [card("1"), card("2"), card("3")] },
      { id: "b", name: "B", cards: [card("4")] },
      { id: "c", name: "C", cards: [] },
    ],
  }
}

const ids = (b: Board) => b.columns.map((c) => c.cards.map((k) => k.id).join(""))

describe("boardOf", () => {
  it("gives a workspace without a board the four default columns", () => {
    const b = boardOf({}, name)
    expect(b.columns.map((c) => c.id)).toEqual(["todo", "in-progress", "test", "done"])
    expect(b.columns[1].name).toBe("BOARD.INPROGRESS")
    expect(cardCount(b)).toBe(0)
  })

  it("keeps default column ids stable between reads", () => {
    expect(boardOf({}, name)).toEqual(defaultBoard(name))
  })

  it("returns the stored board as is", () => {
    expect(boardOf({ board: sample() }, name)).toEqual(sample())
  })

  it("falls back to the default for an unusable stored value", () => {
    expect(boardOf({ board: { columns: [] } }, name).columns).toHaveLength(4)
    expect(boardOf({ board: {} as Board }, name).columns).toHaveLength(4)
  })

  it("repairs a column stored without a card list", () => {
    const broken = { columns: [{ id: "x", name: "X" }] } as unknown as Board
    expect(boardOf({ board: broken }, name).columns[0].cards).toEqual([])
  })
})

describe("stored data from before tags, due dates and colours", () => {
  it("fills in the missing card fields", () => {
    const old = { columns: [{ id: "x", name: "X", cards: [{ id: "1", title: "t", note: "" }] }] }
    expect(boardOf({ board: old as unknown as Board }, name).columns[0].cards[0])
      .toEqual({ id: "1", title: "t", note: "", tags: [], due: null })
  })

  it("drops a due date that is not a calendar day and tags that are not text", () => {
    const odd = { columns: [{ id: "x", name: "X", color: "chartreuse", cards: [
      { id: "1", title: "t", note: "", tags: ["ok", 7, ""], due: "next week" },
    ] }] }
    const column = boardOf({ board: odd as unknown as Board }, name).columns[0]
    expect(column.cards[0].tags).toEqual(["ok"])
    expect(column.cards[0].due).toBeNull()
    expect(column.color).toBeUndefined()
  })
})

describe("colours", () => {
  it("gives the default columns blue, amber, purple, green", () => {
    const b = defaultBoard(name)
    expect(b.columns.map((c, i) => columnColor(c, i))).toEqual(["blue", "amber", "purple", "green"])
  })

  it("prefers a picked colour over the positional one", () => {
    const b = setColumnColor(sample(), "a", "pink")
    expect(columnColor(b.columns[0], 0)).toBe("pink")
    expect(columnColor(b.columns[1], 1)).toBe("amber")
  })

  it("cycles through the whole palette and back", () => {
    let c: (typeof PALETTE)[number] = PALETTE[0]
    for (let i = 0; i < PALETTE.length; i++) c = nextColor(c)
    expect(c).toBe(PALETTE[0])
  })

  it("colours a tag by name, ignoring case", () => {
    expect(tagColor("Bug")).toBe(tagColor("bug"))
    expect(PALETTE).toContain(tagColor("frontend"))
  })
})

describe("tags", () => {
  it("adds a cleaned-up tag", () => {
    expect(addTag([], "  back   end ")).toEqual(["back end"])
  })

  it("ignores blanks and tags the card already has, in any case", () => {
    const tags = ["Bug"]
    expect(addTag(tags, "   ")).toBe(tags)
    expect(addTag(tags, "bug")).toBe(tags)
  })

  it("caps the length", () => {
    expect(addTag([], "x".repeat(60))[0]).toHaveLength(24)
  })

  it("lists each tag on the board once, sorted", () => {
    const b: Board = { columns: [
      { id: "a", name: "A", cards: [card("1", ["ui", "Bug"]), card("2", ["bug"])] },
      { id: "b", name: "B", cards: [card("3", ["api"])] },
    ] }
    expect(allTags(b)).toEqual(["api", "Bug", "ui"])
  })
})

describe("due dates", () => {
  it("classifies against today", () => {
    expect(dueState("2026-11-18", "2026-11-19")).toBe("overdue")
    expect(dueState("2026-11-19", "2026-11-19")).toBe("today")
    expect(dueState("2026-11-21", "2026-11-19")).toBe("soon")
    expect(dueState("2026-11-22", "2026-11-19")).toBe("later")
  })

  it("counts days across a month and a year boundary", () => {
    expect(dueState("2026-12-01", "2026-11-30")).toBe("soon")
    expect(dueState("2027-01-01", "2026-12-31")).toBe("soon")
    expect(dueState("2026-12-31", "2027-01-01")).toBe("overdue")
  })

  it("writes a local date as a calendar day", () => {
    expect(isoDay(new Date(2026, 0, 5, 23, 59))).toBe("2026-01-05")
  })

  it("shows the year only when it differs from the current one", () => {
    expect(formatDue("2026-11-19", "2026-01-01", "en")).not.toContain("2026")
    expect(formatDue("2027-03-02", "2026-11-19", "en")).toContain("2027")
  })
})

describe("cards", () => {
  it("adds to the bottom of the named column", () => {
    expect(ids(addCard(sample(), "b", card("9")))).toEqual(["123", "49", ""])
  })

  it("updates only the named card", () => {
    const b = updateCard(sample(), "2", { title: "new", note: "n" })
    expect(findCard(b, "2")).toEqual({ id: "2", title: "new", note: "n", tags: [], due: null })
    expect(findCard(b, "1")?.title).toBe("1")
  })

  it("removes a card", () => {
    expect(ids(removeCard(sample(), "2"))).toEqual(["13", "4", ""])
  })
})

describe("moveCard", () => {
  it("moves to the bottom of another column", () => {
    expect(ids(moveCard(sample(), "1", "c", null))).toEqual(["23", "4", "1"])
  })

  it("moves above a given card in another column", () => {
    expect(ids(moveCard(sample(), "3", "b", "4"))).toEqual(["12", "34", ""])
  })

  it("reorders downwards within a column", () => {
    expect(ids(moveCard(sample(), "1", "a", "3"))).toEqual(["213", "4", ""])
    expect(ids(moveCard(sample(), "1", "a", null))).toEqual(["231", "4", ""])
  })

  it("reorders upwards within a column", () => {
    expect(ids(moveCard(sample(), "3", "a", "1"))).toEqual(["312", "4", ""])
  })

  it("is a no-op when dropped on itself or just above its neighbour", () => {
    const b = sample()
    expect(moveCard(b, "2", "a", "2")).toBe(b)
    expect(ids(moveCard(b, "2", "a", "3"))).toEqual(["123", "4", ""])
  })

  it("ignores an unknown card or column", () => {
    const b = sample()
    expect(moveCard(b, "nope", "a", null)).toBe(b)
    expect(moveCard(b, "1", "nope", null)).toBe(b)
  })

  it("lands at the bottom when the anchor card is not in the target column", () => {
    expect(ids(moveCard(sample(), "1", "b", "3"))).toEqual(["23", "41", ""])
  })

  it("never loses or duplicates a card", () => {
    expect(cardCount(moveCard(sample(), "2", "c", null))).toBe(4)
  })
})

describe("columns", () => {
  it("adds a column at the right end", () => {
    const b = addColumn(sample(), { id: "d", name: "D", cards: [] })
    expect(b.columns.map((c) => c.id)).toEqual(["a", "b", "c", "d"])
  })

  it("renames a column", () => {
    expect(renameColumn(sample(), "b", "Review").columns[1].name).toBe("Review")
  })

  it("hands a removed column's cards to the column on its left", () => {
    expect(ids(removeColumn(sample(), "b"))).toEqual(["1234", ""])
  })

  it("hands the first column's cards to the column on its right", () => {
    expect(ids(removeColumn(sample(), "a"))).toEqual(["4123", ""])
  })

  it("refuses to remove the last column", () => {
    const one: Board = { columns: [{ id: "a", name: "A", cards: [card("1")] }] }
    expect(removeColumn(one, "a")).toBe(one)
  })

  it("shifts a column and stops at the ends", () => {
    const b = sample()
    expect(shiftColumn(b, "a", 1).columns.map((c) => c.id)).toEqual(["b", "a", "c"])
    expect(shiftColumn(b, "c", -1).columns.map((c) => c.id)).toEqual(["a", "c", "b"])
    expect(shiftColumn(b, "a", -1)).toBe(b)
    expect(shiftColumn(b, "c", 1)).toBe(b)
  })
})
