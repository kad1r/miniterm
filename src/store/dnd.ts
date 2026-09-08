import type { DropTarget } from "./tree"

export type DropZone = "before" | "into" | "after"

export function zoneFor(offsetY: number, height: number, isFolder: boolean): DropZone {
  const y = Math.min(Math.max(offsetY, 0), height)
  if (y < height * 0.25) return "before"
  if (y > height * 0.75) return "after"
  return isFolder ? "into" : "after"
}

export function targetFor(zone: DropZone, nodeId: string): DropTarget {
  if (zone === "into") return { type: "into", folderId: nodeId }
  if (zone === "before") return { type: "before", siblingId: nodeId }
  return { type: "after", siblingId: nodeId }
}
