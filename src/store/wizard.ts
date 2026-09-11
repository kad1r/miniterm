import type { Workspace } from "./types"
import { equalSizes, type GridLayout } from "./layout"
import { basename } from "./tree"
export { basename } from "./tree"

export interface Draft {
  path: string
  name: string
  aiToolId: string | null
  shellId: string | null
  count: number
  layout: GridLayout
}

export function emptyDraft(): Draft {
  return {
    path: "", name: "", aiToolId: null, shellId: null,
    count: 1, layout: { rows: 1, cols: 1 },
  }
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
