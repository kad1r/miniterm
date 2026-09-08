import { toGridTemplate } from "./layout"

export const DIVIDER_PX = 4

export interface Cell {
  index: number
  row: number
  col: number
}

export function cells(rows: number, cols: number): Cell[] {
  const out: Cell[] = []
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      out.push({ index: row * cols + col, row, col })
    }
  }
  return out
}

export function dividerCount(tracks: number): number {
  return Math.max(0, tracks - 1)
}

export function templateWithDividers(sizes: number[], dividerPx = DIVIDER_PX): string {
  return toGridTemplate(sizes).split(" ").join(` ${dividerPx}px `)
}
