import type { Workspace } from "./types"
import { equalSizes, type GridLayout } from "./layout"

export interface Draft {
  path: string
  name: string
  aiToolId: string | null
  shellId: string | null
  count: number
  layout: GridLayout
}

export function basename(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "")
  if (trimmed === "") return path === "" ? "" : path
  const cut = Math.max(trimmed.lastIndexOf("\\"), trimmed.lastIndexOf("/"))
  return cut === -1 ? trimmed : trimmed.slice(cut + 1)
}

export function emptyDraft(): Draft {
  return {
    path: "", name: "", aiToolId: null, shellId: null,
    count: 1, layout: { rows: 1, cols: 1 },
  }
}

export function draftFor(path: string): Draft {
  return { ...emptyDraft(), path, name: basename(path) }
}

export function toWorkspace(draft: Draft): Workspace {
  return {
    id: crypto.randomUUID(),
    kind: "workspace",
    name: draft.name.trim() || basename(draft.path) || "workspace",
    path: draft.path,
    aiToolId: draft.aiToolId,
    shellId: draft.shellId,
    rows: draft.layout.rows,
    cols: draft.layout.cols,
    rowSizes: equalSizes(draft.layout.rows),
    colSizes: equalSizes(draft.layout.cols),
  }
}
