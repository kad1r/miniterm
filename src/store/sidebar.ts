import type { Folder, Node } from "./types"
import { countTerminals } from "./tree"

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

export function deletePrompt(node: Node): string {
  const n = countTerminals(node)
  const tail = n === 0 ? "Açık terminal yok." : `${n} terminal kapanacak.`
  return `"${node.name}" silinecek. ${tail} Devam?`
}

export function newFolder(name = "Yeni klasör"): Folder {
  return { id: crypto.randomUUID(), kind: "folder", name, expanded: true, children: [] }
}
