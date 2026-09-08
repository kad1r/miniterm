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

export function deletePrompt(node: Node, liveSessions: (id: string) => number): string {
  const n = countLiveSessions(node, liveSessions)
  const tail = n === 0 ? "Açık terminal yok." : `${n} terminal kapanacak.`
  return `"${node.name}" silinecek. ${tail} Devam?`
}

export function newFolder(name = "Yeni klasör"): Folder {
  return { id: crypto.randomUUID(), kind: "folder", name, expanded: true, children: [] }
}
