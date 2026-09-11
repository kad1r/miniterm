import { describe, expect, it } from "vitest";
import type { Folder, Node, Workspace } from "./types";
import {
  canDrop,
  countTerminals,
  depthOf,
  findNode,
  heightOf,
  isDescendant,
  move,
  remove,
  rename,
  setExpanded,
  updateWorkspace,
} from "./tree";

function folder(id: string, children: Node[] = []): Folder {
  return { id, kind: "folder", name: id, expanded: true, children };
}

function workspace(id: string, rows = 1, cols = 1): Workspace {
  return {
    id,
    kind: "workspace",
    name: id,
    path: `/tmp/${id}`,
    aiToolId: null,
    shellId: null,
    rows,
    cols,
    rowSizes: Array(rows).fill(1 / rows),
    colSizes: Array(cols).fill(1 / cols),
  };
}

/** f1 > f2 > w1 ; kökte ayrıca w2 */
function sample(): Node[] {
  return [folder("f1", [folder("f2", [workspace("w1")])]), workspace("w2")];
}

describe("findNode / depthOf / heightOf", () => {
  it("finds nodes at any depth", () => {
    expect(findNode(sample(), "w1")?.id).toBe("w1");
    expect(findNode(sample(), "nope")).toBeNull();
  });

  it("counts root nodes as depth 1", () => {
    expect(depthOf(sample(), "f1")).toBe(1);
    expect(depthOf(sample(), "f2")).toBe(2);
    expect(depthOf(sample(), "w1")).toBe(3);
    expect(depthOf(sample(), "nope")).toBe(-1);
  });

  it("counts a leaf as height 1", () => {
    expect(heightOf(workspace("x"))).toBe(1);
    expect(heightOf(folder("a", [folder("b", [workspace("c")])]))).toBe(3);
  });
});

describe("isDescendant", () => {
  it("recognises nodes inside a subtree", () => {
    expect(isDescendant(sample(), "f1", "w1")).toBe(true);
    expect(isDescendant(sample(), "f2", "w1")).toBe(true);
    expect(isDescendant(sample(), "f1", "w2")).toBe(false);
  });

  it("does not treat a node as its own descendant", () => {
    expect(isDescendant(sample(), "f1", "f1")).toBe(false);
  });
});

describe("remove", () => {
  it("detaches a nested node and returns it", () => {
    const { tree, removed } = remove(sample(), "w1");
    expect(removed?.id).toBe("w1");
    expect(findNode(tree, "w1")).toBeNull();
    expect(findNode(tree, "f2")).not.toBeNull();
  });

  it("returns null when the id is absent", () => {
    const { removed } = remove(sample(), "nope");
    expect(removed).toBeNull();
  });
});

describe("canDrop", () => {
  it("allows dropping into a folder", () => {
    expect(canDrop(sample(), "w2", { type: "into", folderId: "f1" })).toBe(true);
  });

  it("refuses dropping a node into itself", () => {
    expect(canDrop(sample(), "f1", { type: "into", folderId: "f1" })).toBe(false);
  });

  it("refuses dropping a node into its own subtree", () => {
    expect(canDrop(sample(), "f1", { type: "into", folderId: "f2" })).toBe(false);
  });

  it("refuses dropping into a workspace", () => {
    expect(canDrop(sample(), "w2", { type: "into", folderId: "w1" })).toBe(false);
  });

  it("refuses a drop that would exceed the depth limit", () => {
    // f1 > f2 > f3 > f4 > f5 zaten 5 seviye
    const deep: Node[] = [
      folder("f1", [folder("f2", [folder("f3", [folder("f4", [folder("f5")])])])]),
      folder("g1", [workspace("gw")]),
    ];
    // g1'in yüksekliği 2; f5'in içine düşerse 6. seviyeye taşar
    expect(canDrop(deep, "g1", { type: "into", folderId: "f5" })).toBe(false);
    // Tek başına bir workspace 5. seviyeye sığar
    expect(canDrop(deep, "gw", { type: "into", folderId: "f4" })).toBe(true);
  });

  it("allows reordering next to a sibling", () => {
    expect(canDrop(sample(), "w2", { type: "before", siblingId: "f1" })).toBe(true);
    expect(canDrop(sample(), "f1", { type: "after", siblingId: "w1" })).toBe(false);
  });
});

describe("move", () => {
  it("moves a node into a folder", () => {
    const tree = move(sample(), "w2", { type: "into", folderId: "f2" });
    expect(depthOf(tree, "w2")).toBe(3);
    expect(tree).toHaveLength(1);
  });

  it("reorders siblings", () => {
    const tree = move(sample(), "w2", { type: "before", siblingId: "f1" });
    expect(tree[0].id).toBe("w2");
    expect(tree[1].id).toBe("f1");
  });

  it("is a no-op when the drop is invalid", () => {
    const before = sample();
    const after = move(before, "f1", { type: "into", folderId: "f2" });
    expect(after).toEqual(before);
  });

  it("moves a node back out to the root", () => {
    const tree = move(sample(), "w1", { type: "rootEnd" });
    expect(depthOf(tree, "w1")).toBe(1);
    expect(tree[tree.length - 1].id).toBe("w1");
  });
});

describe("rename / setExpanded / updateWorkspace", () => {
  it("renames without touching siblings", () => {
    const tree = rename(sample(), "w1", "renamed");
    expect(findNode(tree, "w1")?.name).toBe("renamed");
    expect(findNode(tree, "w2")?.name).toBe("w2");
  });

  it("toggles folder expansion", () => {
    const tree = setExpanded(sample(), "f2", false);
    expect((findNode(tree, "f2") as Folder).expanded).toBe(false);
  });

  it("patches workspace fields", () => {
    const tree = updateWorkspace(sample(), "w1", { rows: 2, cols: 3 });
    const w = findNode(tree, "w1") as Workspace;
    expect(w.rows).toBe(2);
    expect(w.cols).toBe(3);
    expect(w.path).toBe("/tmp/w1");
  });

  it("does not mutate the input tree", () => {
    const before = sample();
    rename(before, "w1", "changed");
    expect(findNode(before, "w1")?.name).toBe("w1");
  });
});

describe("countTerminals", () => {
  it("sums rows * cols across a subtree", () => {
    const tree = folder("root", [workspace("a", 2, 3), folder("mid", [workspace("b", 1, 4)])]);
    expect(countTerminals(tree)).toBe(10);
  });
});
