import type { Board, BoardCard, BoardColumn, Workspace } from "./types"

/** The columns a board starts with, as message keys so they are named in the
 *  user's language the first time the board is shown. The ids are fixed: until
 *  the first change nothing is stored, so the default board is rebuilt on every
 *  read and its columns must keep the same identity between reads. */
export const DEFAULT_COLUMNS = [
  { id: "todo", key: "board.todo" },
  { id: "in-progress", key: "board.inProgress" },
  { id: "test", key: "board.test" },
  { id: "done", key: "board.done" },
] as const

/** Accent colours for columns and tags, as keys the stylesheet maps to actual
 *  colours per theme. The first four are what the default columns get, in
 *  order: blue to-do, amber in progress, purple test, green done. */
export const PALETTE = ["blue", "amber", "purple", "green", "red", "teal", "pink", "orange"] as const
export type PaletteColor = (typeof PALETTE)[number]

function isPaletteColor(value: unknown): value is PaletteColor {
  return (PALETTE as readonly unknown[]).includes(value)
}

/** A column's colour: its own if it was ever picked, else one by position. */
export function columnColor(column: Pick<BoardColumn, "color">, index: number): PaletteColor {
  return isPaletteColor(column.color) ? column.color : PALETTE[index % PALETTE.length]
}

export function nextColor(color: PaletteColor): PaletteColor {
  return PALETTE[(PALETTE.indexOf(color) + 1) % PALETTE.length]
}

/** Same name, same colour — everywhere and on every run, with nothing stored.
 *  Case-insensitive so "Bug" and "bug" do not end up in different colours. */
export function tagColor(tag: string): PaletteColor {
  let hash = 0
  for (const ch of tag.toLowerCase()) hash = (hash * 31 + ch.codePointAt(0)!) >>> 0
  return PALETTE[hash % PALETTE.length]
}

export const TAG_MAX_LENGTH = 24

/** Add a typed tag to a card's list: trimmed, inner whitespace collapsed,
 *  capped in length, and ignored if the card already carries it in any case. */
export function addTag(tags: string[], raw: string): string[] {
  const tag = raw.trim().replace(/\s+/g, " ").slice(0, TAG_MAX_LENGTH).trim()
  if (!tag || tags.some((x) => x.toLowerCase() === tag.toLowerCase())) return tags
  return [...tags, tag]
}

/** Every tag in use on the board, alphabetically, one spelling per tag. */
export function allTags(board: Board): string[] {
  const seen = new Map<string, string>()
  for (const c of board.columns) {
    for (const k of c.cards) {
      for (const tag of k.tags) if (!seen.has(tag.toLowerCase())) seen.set(tag.toLowerCase(), tag)
    }
  }
  return [...seen.values()].sort((a, b) => a.localeCompare(b))
}

const ISO_DAY = /^\d{4}-\d{2}-\d{2}$/

/** A date as `YYYY-MM-DD` in the user's own time zone (toISOString would give
 *  the UTC day, which is yesterday or tomorrow for part of every day). */
export function isoDay(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0")
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

function dayNumber(iso: string): number {
  const [y, m, d] = iso.split("-").map(Number)
  return Math.round(Date.UTC(y, m - 1, d) / 86_400_000)
}

export type DueState = "overdue" | "today" | "soon" | "later"

/** How urgent a due date is on a given day; "soon" is within the next two days. */
export function dueState(due: string, today: string): DueState {
  const days = dayNumber(due) - dayNumber(today)
  if (days < 0) return "overdue"
  if (days === 0) return "today"
  return days <= 2 ? "soon" : "later"
}

/** "19 Nov", plus the year only when it is not the current one. */
export function formatDue(due: string, today: string, locale: string): string {
  const [y, m, d] = due.split("-").map(Number)
  const sameYear = due.slice(0, 4) === today.slice(0, 4)
  return new Intl.DateTimeFormat(locale, {
    day: "numeric",
    month: "short",
    ...(sameYear ? {} : { year: "numeric" }),
  }).format(new Date(y, m - 1, d))
}

export function defaultBoard(name: (key: (typeof DEFAULT_COLUMNS)[number]["key"]) => string): Board {
  return { columns: DEFAULT_COLUMNS.map((c) => ({ id: c.id, name: name(c.key), cards: [] })) }
}

/** The workspace's board, or the default one when it has none (or the stored
 *  value is not usable — the backend passes it through without checking). */
export function boardOf(
  ws: Pick<Workspace, "board">,
  name: Parameters<typeof defaultBoard>[0],
): Board {
  const stored = ws.board
  if (!stored || !Array.isArray(stored.columns) || stored.columns.length === 0) {
    return defaultBoard(name)
  }
  return {
    columns: stored.columns.map((c) => ({
      id: String(c.id),
      name: String(c.name ?? ""),
      ...(isPaletteColor(c.color) ? { color: c.color } : {}),
      // Cards saved before tags and due dates existed have neither field.
      cards: (Array.isArray(c.cards) ? c.cards : []).map((k) => ({
        id: String(k.id),
        title: String(k.title ?? ""),
        note: String(k.note ?? ""),
        tags: Array.isArray(k.tags) ? k.tags.filter((x) => typeof x === "string" && x !== "") : [],
        due: typeof k.due === "string" && ISO_DAY.test(k.due) ? k.due : null,
      })),
    })),
  }
}

function mapColumn(board: Board, id: string, fn: (c: BoardColumn) => BoardColumn): Board {
  return { columns: board.columns.map((c) => (c.id === id ? fn(c) : c)) }
}

export function findCard(board: Board, cardId: string): BoardCard | null {
  for (const c of board.columns) {
    const hit = c.cards.find((k) => k.id === cardId)
    if (hit) return hit
  }
  return null
}

export function cardCount(board: Board): number {
  return board.columns.reduce((n, c) => n + c.cards.length, 0)
}

export function addCard(board: Board, columnId: string, card: BoardCard): Board {
  return mapColumn(board, columnId, (c) => ({ ...c, cards: [...c.cards, card] }))
}

export function updateCard(
  board: Board,
  cardId: string,
  patch: Partial<Omit<BoardCard, "id">>,
): Board {
  return {
    columns: board.columns.map((c) => ({
      ...c,
      cards: c.cards.map((k) => (k.id === cardId ? { ...k, ...patch } : k)),
    })),
  }
}

export function removeCard(board: Board, cardId: string): Board {
  return {
    columns: board.columns.map((c) => ({ ...c, cards: c.cards.filter((k) => k.id !== cardId) })),
  }
}

/** Put the card in `toColumnId`, just above `beforeCardId`, or at the bottom
 *  when that is null (or no longer in the column). Unknown card or column, or
 *  a drop onto itself, leaves the board as it was. */
export function moveCard(
  board: Board,
  cardId: string,
  toColumnId: string,
  beforeCardId: string | null,
): Board {
  const card = findCard(board, cardId)
  if (!card || cardId === beforeCardId) return board
  if (!board.columns.some((c) => c.id === toColumnId)) return board
  return mapColumn(removeCard(board, cardId), toColumnId, (c) => {
    const at = beforeCardId === null ? -1 : c.cards.findIndex((k) => k.id === beforeCardId)
    const cards = [...c.cards]
    cards.splice(at === -1 ? cards.length : at, 0, card)
    return { ...c, cards }
  })
}

export function addColumn(board: Board, column: BoardColumn): Board {
  return { columns: [...board.columns, column] }
}

export function setColumnColor(board: Board, id: string, color: PaletteColor): Board {
  return mapColumn(board, id, (c) => ({ ...c, color }))
}

export function renameColumn(board: Board, id: string, name: string): Board {
  return mapColumn(board, id, (c) => ({ ...c, name }))
}

/** Remove a column without losing work: its cards go to the bottom of the
 *  column on its left (on its right for the first one). The last remaining
 *  column cannot be removed. */
export function removeColumn(board: Board, id: string): Board {
  const at = board.columns.findIndex((c) => c.id === id)
  if (at === -1 || board.columns.length === 1) return board
  const heir = board.columns[at === 0 ? 1 : at - 1].id
  const orphans = board.columns[at].cards
  return {
    columns: board.columns
      .filter((c) => c.id !== id)
      .map((c) => (c.id === heir ? { ...c, cards: [...c.cards, ...orphans] } : c)),
  }
}

/** Shift a column one place left (-1) or right (1); a no-op at either end. */
export function shiftColumn(board: Board, id: string, by: -1 | 1): Board {
  const at = board.columns.findIndex((c) => c.id === id)
  const to = at + by
  if (at === -1 || to < 0 || to >= board.columns.length) return board
  const columns = [...board.columns]
  ;[columns[at], columns[to]] = [columns[to], columns[at]]
  return { columns }
}
