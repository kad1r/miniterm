import { translate, type Locale } from "../i18n/messages"
import type { Folder, Node } from "./types"

export interface Row {
  node: Node
  depth: number
}

export function flatten(tree: Node[], depth = 1): Row[] {
  const rows: Row[] = []
  for (const node of tree) {
    rows.push({ node, depth })
    if (node.kind === "folder" && node.expanded) {
      rows.push(...flatten(node.children, depth + 1))
    }
  }
  return rows
}

/** Count the number of live (spawned) sessions under a node.
 *  A workspace contributes its slot count only if it has an entry in the live
 *  session map; a workspace that was never activated contributes zero.
 *  The `liveSessions` map is keyed by workspace id → number of non-null ids. */
export function countLiveSessions(node: Node, liveSessions: (id: string) => number): number {
  if (node.kind === "workspace") return liveSessions(node.id)
  return node.children.reduce((sum, child) => sum + countLiveSessions(child, liveSessions), 0)
}

export function deletePrompt(
  node: Node,
  liveSessions: (id: string) => number,
  locale: Locale,
): string {
  const n = countLiveSessions(node, liveSessions)
  return n === 0
    ? translate(locale, "sidebar.deleteNoTerminals", { name: node.name })
    : translate(locale, "sidebar.deleteWithTerminals", { name: node.name, count: n })
}

export function newFolder(name: string): Folder {
  return { id: crypto.randomUUID(), kind: "folder", name, expanded: true, children: [] }
}

export const COLLAPSE_KEY = "miniterm.sidebar.collapsed"

/** The slice of `Storage` this module needs. Narrow on purpose: it keeps the
 *  functions testable in the node environment, where `localStorage` is absent. */
export interface CollapseStore {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
}

/** Read the persisted collapse state, defaulting to expanded.
 *  Storage access can throw (private browsing, disabled cookies) and the stored
 *  value can be anything, so every failure resolves to the default rather than
 *  propagating — a bad entry must not stop the sidebar from rendering. */
export function readCollapsed(store: CollapseStore): boolean {
  try {
    return store.getItem(COLLAPSE_KEY) === "true"
  } catch {
    return false
  }
}

/** Persist the collapse state. Failure is silent: this is a cosmetic
 *  preference, and a quota or permission error must not break the toggle. */
export function writeCollapsed(store: CollapseStore, collapsed: boolean): void {
  try {
    store.setItem(COLLAPSE_KEY, String(collapsed))
  } catch {
    // ignored — see doc comment
  }
}
