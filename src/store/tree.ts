import type { Folder, Node, Workspace } from "./types";

export const MAX_DEPTH = 5;

export type DropTarget =
  | { type: "into"; folderId: string }
  | { type: "before"; siblingId: string }
  | { type: "after"; siblingId: string }
  | { type: "rootEnd" };

function childrenOf(node: Node): Node[] {
  return node.kind === "folder" ? node.children : [];
}

export function findNode(tree: Node[], id: string): Node | null {
  for (const node of tree) {
    if (node.id === id) return node;
    const hit = findNode(childrenOf(node), id);
    if (hit) return hit;
  }
  return null;
}

export function depthOf(tree: Node[], id: string, current = 1): number {
  for (const node of tree) {
    if (node.id === id) return current;
    const hit = depthOf(childrenOf(node), id, current + 1);
    if (hit !== -1) return hit;
  }
  return -1;
}

export function heightOf(node: Node): number {
  const kids = childrenOf(node);
  if (kids.length === 0) return 1;
  return 1 + Math.max(...kids.map(heightOf));
}

export function isDescendant(tree: Node[], ancestorId: string, candidateId: string): boolean {
  const ancestor = findNode(tree, ancestorId);
  if (!ancestor) return false;
  return findNode(childrenOf(ancestor), candidateId) !== null;
}

export function remove(tree: Node[], id: string): { tree: Node[]; removed: Node | null } {
  let removed: Node | null = null;

  function walk(nodes: Node[]): Node[] {
    const out: Node[] = [];
    for (const node of nodes) {
      if (node.id === id) {
        removed = node;
        continue;
      }
      out.push(node.kind === "folder" ? { ...node, children: walk(node.children) } : node);
    }
    return out;
  }

  return { tree: walk(tree), removed };
}

export function insert(tree: Node[], node: Node, target: DropTarget): Node[] {
  if (target.type === "rootEnd") return [...tree, node];

  function walk(nodes: Node[]): Node[] {
    const out: Node[] = [];
    for (const current of nodes) {
      if (target.type === "before" && current.id === target.siblingId) out.push(node);

      if (target.type === "into" && current.id === target.folderId && current.kind === "folder") {
        out.push({ ...current, children: [...current.children, node], expanded: true });
      } else if (current.kind === "folder") {
        out.push({ ...current, children: walk(current.children) });
      } else {
        out.push(current);
      }

      if (target.type === "after" && current.id === target.siblingId) out.push(node);
    }
    return out;
  }

  return walk(tree);
}

export function canDrop(tree: Node[], nodeId: string, target: DropTarget): boolean {
  const node = findNode(tree, nodeId);
  if (!node) return false;

  let anchorId: string | null = null;
  let insertionDepth: number;

  if (target.type === "rootEnd") {
    insertionDepth = 1;
  } else if (target.type === "into") {
    anchorId = target.folderId;
    const anchor = findNode(tree, anchorId);
    if (!anchor || anchor.kind !== "folder") return false;
    insertionDepth = depthOf(tree, anchorId) + 1;
  } else {
    anchorId = target.siblingId;
    if (!findNode(tree, anchorId)) return false;
    insertionDepth = depthOf(tree, anchorId);
  }

  if (anchorId === nodeId) return false;
  if (anchorId && isDescendant(tree, nodeId, anchorId)) return false;
  if (insertionDepth + heightOf(node) - 1 > MAX_DEPTH) return false;

  return true;
}

export function move(tree: Node[], nodeId: string, target: DropTarget): Node[] {
  if (!canDrop(tree, nodeId, target)) return tree;
  const { tree: without, removed } = remove(tree, nodeId);
  if (!removed) return tree;
  return insert(without, removed, target);
}

function patch(tree: Node[], id: string, fn: (node: Node) => Node): Node[] {
  return tree.map((node) => {
    if (node.id === id) return fn(node);
    if (node.kind === "folder") return { ...node, children: patch(node.children, id, fn) };
    return node;
  });
}

export function rename(tree: Node[], id: string, name: string): Node[] {
  return patch(tree, id, (node) => ({ ...node, name }));
}

export function setExpanded(tree: Node[], id: string, expanded: boolean): Node[] {
  return patch(tree, id, (node) =>
    node.kind === "folder" ? ({ ...node, expanded } as Folder) : node,
  );
}

export function updateWorkspace(tree: Node[], id: string, p: Partial<Workspace>): Node[] {
  return patch(tree, id, (node) =>
    node.kind === "workspace" ? ({ ...node, ...p } as Workspace) : node,
  );
}

export function countTerminals(node: Node): number {
  if (node.kind === "workspace") return node.rows * node.cols;
  return node.children.reduce((sum, child) => sum + countTerminals(child), 0);
}
