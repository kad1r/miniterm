import { equalSizes, layoutsFor, MAX_TERMINALS, type GridLayout } from "./layout";
import type { Workspace } from "./types";

/** The four workspace fields that together describe the terminal grid. */
export type GridPatch = Pick<Workspace, "rows" | "cols" | "rowSizes" | "colSizes">;

export function terminalCount(ws: Pick<Workspace, "rows" | "cols">): number {
  return ws.rows * ws.cols;
}

/** The layout a workspace should land on when a terminal is added: the same
 *  orientation it already has, when that shape exists for the new count — a 2×1
 *  column grows into a 3×1, not a 1×3. A lone pane has no orientation yet, so it
 *  splits side by side like every other terminal does. */
export function defaultLayoutFor(ws: Pick<Workspace, "rows" | "cols">, count: number): GridLayout {
  const options = layoutsFor(count);
  if (options.length === 0) return { rows: ws.rows, cols: ws.cols };
  const column = ws.cols === 1 && ws.rows > 1;
  const sameShape = options.find((l) => (column ? l.cols === 1 : l.rows === ws.rows));
  return sameShape ?? options[0];
}

/** Track sizes are dragged by hand, so a relayout keeps the axis whose track
 *  count did not change and only redistributes the one that grew. Copying the
 *  kept array matters: the caller feeds this straight into the config, and a
 *  shared reference would let a later divider drag mutate the old value too. */
export function relayout(ws: GridPatch, next: GridLayout): GridPatch {
  return {
    rows: next.rows,
    cols: next.cols,
    rowSizes: ws.rows === next.rows ? [...ws.rowSizes] : equalSizes(next.rows),
    colSizes: ws.cols === next.cols ? [...ws.colSizes] : equalSizes(next.cols),
  };
}

/** Counts the dialog may offer. Shrinking would kill running shells, which is a
 *  different action than the one the user asked for, so the floor is today's
 *  count and the ceiling is the grid limit. */
export function addableCounts(ws: Pick<Workspace, "rows" | "cols">): number[] {
  const current = terminalCount(ws);
  const out: number[] = [];
  for (let n = current; n <= MAX_TERMINALS; n++) out.push(n);
  return out;
}
