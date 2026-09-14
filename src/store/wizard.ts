import type { Workspace } from "./types"
import { equalSizes, type GridLayout } from "./layout"
import { normalizeTools, type PaneTools } from "./panes"
import { basename } from "./tree"
export { basename } from "./tree"

export interface Draft {
  path: string
  name: string
  /** The workspace default — what `paneTools` falls back to, and what a pane
   *  added later gets. */
  aiToolId: string | null
  shellId: string | null
  count: number
  layout: GridLayout
  /** Per-pane override of `aiToolId`, in cell order. May be shorter than the
   *  grid; `toWorkspace` pads it. */
  paneTools: PaneTools
}

export function emptyDraft(): Draft {
  return {
    path: "", name: "", aiToolId: null, shellId: null,
    count: 1, layout: { rows: 1, cols: 1 }, paneTools: [],
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
    paneTools: normalizeTools(
      draft.paneTools,
      draft.layout.rows * draft.layout.cols,
      draft.aiToolId,
    ),
  }
}
