# miniterm M2 — Tiling Layout & Drag-Resize Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn miniterm's single pane into a tiling multi-pane terminal: split panes horizontally/vertically, close panes, drag pane borders to resize, and auto-resize each pane's ConPTY grid on any layout change.

**Architecture:** A binary split tree (`layout/tree.rs`) holds `Leaf(PaneId)` and `Split{dir,ratio,a,b}` nodes; recursive rect assignment maps the tree to on-screen pixel rects minus a gutter. `layout/hit.rs` walks the tree to find the split divider under the mouse for drag-resize. `App` (in `app.rs`) owns a `SlotMap<PaneId, Session>`, the tree, and the focused pane; on any relayout it recomputes each leaf's `(rows, cols)` from its rect and calls `Session::resize`. Rendering concatenates every visible pane's quads (each `build_instances` call uses the pane's rect origin) into one `draw_quads` frame.

**Tech Stack:** Rust 2021, winit 0.29/0.30, wgpu 0.19, alacritty_terminal 0.24, portable-pty, swash, slotmap.

**Spec:** docs/superpowers/specs/2026-08-25-miniterm-design.md (§4 architecture, §6 layout/resize/auto-PTY-resize, §7 render scope, §10 testing). This plan implements phasing steps 3–4 (spec §12).

## Global Constraints

- **Toolchain:** stable-x86_64-pc-windows-gnu (pinned in rust-toolchain.toml). MSVC absent. All deps pure Rust. Do NOT change the toolchain.
- **cargo PATH:** cargo is NOT on the bash PATH. Every cargo invocation MUST be prefixed with `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"`.
- **Branch:** continue on `feature/m1-single-terminal` (M1 not yet merged). This plan builds on M1 HEAD `c079aa9`.
- **Font:** CascadiaMono.ttf at assets/font/, FONT_PX = 18.0, measured via `text::metrics::measure`.
- **Idle 0% CPU:** redraw is requested ONLY on input / PtyOutput / resize / drag / split / close — NEVER unconditionally per loop iteration. winit stays in default `Wait` mode. Per-session PTY-output coalescing (the `Arc<AtomicBool>` gate in `Session`) must be preserved for every pane.
- **Threading:** each `Session` owns one blocking PTY reader thread that feeds its `Term` behind a `Mutex` and wakes the main thread via the shared `EventLoopProxy`. Locks are held only around a grid snapshot or `parser.advance`, never across a blocking op. Reader-thread lock uses `.lock().unwrap_or_else(|e| e.into_inner())` (poison recovery) — keep this pattern anywhere a Session is locked off the main thread.
- **Gutter / min-size defaults:** gutter = 4.0 px; min pane size = 2 cols × 1 row (converted to px via cell metrics for clamping).

## Existing interfaces this plan consumes (from M1, verified against current code)

- `text::metrics::CellMetrics { cell_w: f32, cell_h: f32, ascent: f32 }`; `text::metrics::measure(font: &[u8], px: f32) -> CellMetrics`.
- `terminal::session::Session`:
  - `Session::spawn(rows: u16, cols: u16, shell: &str, on_output: impl Fn() + Send + 'static) -> Session`
  - `Session::write(&mut self, bytes: &[u8])`
  - `Session::resize(&mut self, rows: u16, cols: u16)`
  - public fields: `term: Arc<Mutex<Term<EventProxy>>>`, `redraw_pending: Arc<AtomicBool>`.
- `render::grid_draw::{QuadInstance, CellView, build_instances}`:
  - `build_instances(cells: &[Vec<CellView>], m: &CellMetrics, origin: [f32;2], atlas_uv: &dyn Fn(char)->([f32;2],[f32;2])) -> (Vec<QuadInstance>, Vec<QuadInstance>)`
  - `QuadInstance { pos:[f32;2], size:[f32;2], uv_min:[f32;2], uv_max:[f32;2], color:[f32;4] }`
- `render::atlas_gpu::GpuAtlas::uv_for(&mut self, queue: &wgpu::Queue, ch: char) -> ([f32;2],[f32;2])`.
- `render::renderer::Renderer`: `new(&Window)`, `resize(PhysicalSize<u32>)`, `draw_quads(&mut self, bg: &[QuadInstance], glyphs: &[QuadInstance], atlas: &GpuAtlas)`, `queue()`, `device()`, `atlas_bind_group_layout()`.
- `app::grid_to_cells(session: &Session) -> (Vec<Vec<CellView>>, (usize, usize))` — snapshots the term to CellView rows + cursor (line, col). This will be generalized in Task 4.

---

## File Structure

- Create `src/layout/mod.rs` — `pub mod tree; pub mod hit;`
- Create `src/layout/tree.rs` — `PaneId`, `Rect`, `Dir`, `Side`, `Node`, `LayoutTree` + split/close/compute_rects/set_split_ratio. Pure logic, unit-tested.
- Create `src/layout/hit.rs` — `hit_test` returning the split path + orientation under a cursor. Pure logic, unit-tested.
- Modify `src/app.rs` — replace the free `grid_to_cells` with an `App` struct owning the pane map, tree, focus; relayout + frame-building + action methods.
- Modify `src/render/renderer.rs` — add `surface_size(&self) -> (u32, u32)` accessor (needed to compute the root rect). No pipeline change.
- Modify `src/main.rs` — construct `App`; wire keybindings (split/close/focus), mouse (focus click, border drag, cursor icon), window resize → relayout, PtyOutput → redraw.
- Modify `Cargo.toml` — ensure `slotmap` dependency present.

---

### Task 1: Layout tree data model — split, close, rect assignment

**Files:**
- Modify: `Cargo.toml` (add `slotmap` if absent)
- Create: `src/layout/mod.rs`
- Create: `src/layout/tree.rs`
- Modify: `src/main.rs` (add `mod layout;`)
- Test: in `src/layout/tree.rs` under `#[cfg(test)]`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `slotmap::new_key_type! { pub struct PaneId; }`
  - `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }`
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Dir { Horizontal, Vertical }` — `Horizontal` = children side by side (a left, b right, divider is vertical); `Vertical` = children stacked (a top, b bottom, divider is horizontal).
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Side { A, B }`
  - `pub enum Node { Leaf(PaneId), Split { dir: Dir, ratio: f32, a: Box<Node>, b: Box<Node> } }`
  - `pub struct LayoutTree { pub root: Node }`
  - `LayoutTree::new(first: PaneId) -> LayoutTree`
  - `LayoutTree::split(&mut self, target: PaneId, new_pane: PaneId, dir: Dir, ratio: f32) -> bool`
  - `LayoutTree::close(&mut self, target: PaneId) -> bool`
  - `LayoutTree::compute_rects(&self, root: Rect, gutter: f32) -> Vec<(PaneId, Rect)>`
  - `LayoutTree::pane_ids(&self) -> Vec<PaneId>`

- [ ] **Step 1: Ensure `slotmap` is a dependency**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && grep -n slotmap Cargo.toml || echo MISSING`
If MISSING, add under `[dependencies]` in `Cargo.toml`:

```toml
slotmap = "1"
```

- [ ] **Step 2: Create `src/layout/mod.rs`**

```rust
pub mod tree;
pub mod hit;
```

- [ ] **Step 3: Write the failing tests in `src/layout/tree.rs`**

```rust
use slotmap::new_key_type;

new_key_type! {
    pub struct PaneId;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Horizontal, // a | b  (side by side, vertical divider)
    Vertical,   // a / b  (stacked,     horizontal divider)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    A,
    B,
}

pub enum Node {
    Leaf(PaneId),
    Split {
        dir: Dir,
        ratio: f32,
        a: Box<Node>,
        b: Box<Node>,
    },
}

pub struct LayoutTree {
    pub root: Node,
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    fn approx(a: Rect, b: Rect) -> bool {
        (a.x - b.x).abs() < 0.01
            && (a.y - b.y).abs() < 0.01
            && (a.w - b.w).abs() < 0.01
            && (a.h - b.h).abs() < 0.01
    }

    #[test]
    fn single_leaf_fills_root_rect() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let p = sm.insert(());
        let tree = LayoutTree::new(p);
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, 4.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, p);
        assert!(approx(rects[0].1, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }));
    }

    #[test]
    fn horizontal_split_halves_width_minus_gutter() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        assert!(tree.split(a, b, Dir::Horizontal, 0.5));
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 804.0, h: 600.0 }, 4.0);
        // total width 804, gutter 4 => each pane 400 wide.
        let ra = rects.iter().find(|(id, _)| *id == a).unwrap().1;
        let rb = rects.iter().find(|(id, _)| *id == b).unwrap().1;
        assert!(approx(ra, Rect { x: 0.0, y: 0.0, w: 400.0, h: 600.0 }));
        assert!(approx(rb, Rect { x: 404.0, y: 0.0, w: 400.0, h: 600.0 }));
    }

    #[test]
    fn vertical_split_halves_height_minus_gutter() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        assert!(tree.split(a, b, Dir::Vertical, 0.5));
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 800.0, h: 604.0 }, 4.0);
        let ra = rects.iter().find(|(id, _)| *id == a).unwrap().1;
        let rb = rects.iter().find(|(id, _)| *id == b).unwrap().1;
        assert!(approx(ra, Rect { x: 0.0, y: 0.0, w: 800.0, h: 300.0 }));
        assert!(approx(rb, Rect { x: 0.0, y: 304.0, w: 800.0, h: 300.0 }));
    }

    #[test]
    fn close_collapses_sibling_into_parent_rect() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        assert!(tree.close(b));
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, 4.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, a);
        assert!(approx(rects[0].1, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }));
    }

    #[test]
    fn cannot_close_last_pane() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let mut tree = LayoutTree::new(a);
        assert!(!tree.close(a));
    }

    #[test]
    fn pane_ids_lists_all_leaves() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let c = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        tree.split(b, c, Dir::Vertical, 0.5);
        let mut ids = tree.pane_ids();
        ids.sort_by_key(|k| format!("{:?}", k));
        let mut expected = vec![a, b, c];
        expected.sort_by_key(|k| format!("{:?}", k));
        assert_eq!(ids, expected);
    }
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test layout::tree -- --nocapture`
Expected: FAIL (missing `new`, `split`, `close`, `compute_rects`, `pane_ids`).

- [ ] **Step 5: Implement `LayoutTree` methods in `src/layout/tree.rs`**

```rust
impl LayoutTree {
    pub fn new(first: PaneId) -> LayoutTree {
        LayoutTree { root: Node::Leaf(first) }
    }

    /// Split the leaf holding `target`: it becomes child A, `new_pane` becomes B.
    /// Returns false if `target` is not a leaf in the tree.
    pub fn split(&mut self, target: PaneId, new_pane: PaneId, dir: Dir, ratio: f32) -> bool {
        Self::split_node(&mut self.root, target, new_pane, dir, ratio)
    }

    fn split_node(node: &mut Node, target: PaneId, new_pane: PaneId, dir: Dir, ratio: f32) -> bool {
        match node {
            Node::Leaf(id) if *id == target => {
                let a = Node::Leaf(target);
                let b = Node::Leaf(new_pane);
                *node = Node::Split { dir, ratio, a: Box::new(a), b: Box::new(b) };
                true
            }
            Node::Leaf(_) => false,
            Node::Split { a, b, .. } => {
                Self::split_node(a, target, new_pane, dir, ratio)
                    || Self::split_node(b, target, new_pane, dir, ratio)
            }
        }
    }

    /// Remove the leaf holding `target`; its sibling collapses into the parent.
    /// Returns false if `target` is the sole remaining pane or not found.
    pub fn close(&mut self, target: PaneId) -> bool {
        // Root is the target leaf: cannot close the last pane.
        if let Node::Leaf(id) = &self.root {
            return *id != target && false || (*id == target && false); // never close last
        }
        Self::close_node(&mut self.root, target)
    }

    fn close_node(node: &mut Node, target: PaneId) -> bool {
        if let Node::Split { a, b, .. } = node {
            // If either direct child is the target leaf, replace self with the sibling.
            let a_is = matches!(a.as_ref(), Node::Leaf(id) if *id == target);
            let b_is = matches!(b.as_ref(), Node::Leaf(id) if *id == target);
            if a_is {
                let sibling = std::mem::replace(b.as_mut(), Node::Leaf(target));
                *node = sibling;
                return true;
            }
            if b_is {
                let sibling = std::mem::replace(a.as_mut(), Node::Leaf(target));
                *node = sibling;
                return true;
            }
            // Recurse.
            let (a, b) = match node {
                Node::Split { a, b, .. } => (a, b),
                _ => unreachable!(),
            };
            return Self::close_node(a, target) || Self::close_node(b, target);
        }
        false
    }

    pub fn compute_rects(&self, root: Rect, gutter: f32) -> Vec<(PaneId, Rect)> {
        let mut out = Vec::new();
        Self::assign(&self.root, root, gutter, &mut out);
        out
    }

    fn assign(node: &Node, rect: Rect, gutter: f32, out: &mut Vec<(PaneId, Rect)>) {
        match node {
            Node::Leaf(id) => out.push((*id, rect)),
            Node::Split { dir, ratio, a, b } => {
                let (ra, rb) = split_rect(rect, *dir, *ratio, gutter);
                Self::assign(a, ra, gutter, out);
                Self::assign(b, rb, gutter, out);
            }
        }
    }

    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        Self::collect_ids(&self.root, &mut out);
        out
    }

    fn collect_ids(node: &Node, out: &mut Vec<PaneId>) {
        match node {
            Node::Leaf(id) => out.push(*id),
            Node::Split { a, b, .. } => {
                Self::collect_ids(a, out);
                Self::collect_ids(b, out);
            }
        }
    }
}

/// Divide `rect` along `dir` by `ratio`, reserving `gutter` px between children.
pub fn split_rect(rect: Rect, dir: Dir, ratio: f32, gutter: f32) -> (Rect, Rect) {
    match dir {
        Dir::Horizontal => {
            let avail = (rect.w - gutter).max(0.0);
            let wa = (avail * ratio).max(0.0);
            let wb = (avail - wa).max(0.0);
            (
                Rect { x: rect.x, y: rect.y, w: wa, h: rect.h },
                Rect { x: rect.x + wa + gutter, y: rect.y, w: wb, h: rect.h },
            )
        }
        Dir::Vertical => {
            let avail = (rect.h - gutter).max(0.0);
            let ha = (avail * ratio).max(0.0);
            let hb = (avail - ha).max(0.0);
            (
                Rect { x: rect.x, y: rect.y, w: rect.w, h: ha },
                Rect { x: rect.x, y: rect.y + ha + gutter, w: rect.w, h: hb },
            )
        }
    }
}
```

Note: the `close` guard above is intentionally explicit that the root leaf cannot be closed. Simplify the `close` root check to just:

```rust
    pub fn close(&mut self, target: PaneId) -> bool {
        if matches!(&self.root, Node::Leaf(_)) {
            return false; // sole pane — refuse
        }
        Self::close_node(&mut self.root, target)
    }
```

Use this simplified form (replace the placeholder-looking version).

- [ ] **Step 6: Run the tests to verify they pass**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test layout::tree -- --nocapture`
Expected: PASS (6 tests).

- [ ] **Step 7: Wire the module into main**

Add `mod layout;` to `src/main.rs` (alongside `mod app; mod render; mod terminal; mod text;`).

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src/layout/mod.rs src/layout/tree.rs src/main.rs
git commit -m "feat: layout split tree with split/close/rect assignment"
```

---

### Task 2: Drag-resize ratio update with min-size clamp

**Files:**
- Modify: `src/layout/tree.rs` (add `set_split_ratio` + a path-locating helper)
- Test: in `src/layout/tree.rs` under `#[cfg(test)]`

**Interfaces:**
- Consumes: `LayoutTree`, `Rect`, `Dir`, `Side` (Task 1).
- Produces:
  - `LayoutTree::set_split_ratio(&mut self, path: &[Side], ratio: f32) -> bool` — set the ratio of the split reached by following `path` from the root; clamps to `[0.0, 1.0]`. Returns false if the path does not lead to a split.
  - `LayoutTree::clamp_ratio_for(root: Rect, dir: Dir, gutter: f32, min_w: f32, min_h: f32, ratio: f32) -> f32` — clamp so both children keep at least `min_w`×`min_h`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod drag_tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn set_split_ratio_updates_root_split() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        // Root split is reached by an empty path.
        assert!(tree.set_split_ratio(&[], 0.25));
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 804.0, h: 600.0 }, 4.0);
        let ra = rects.iter().find(|(id, _)| *id == a).unwrap().1;
        // avail 800 * 0.25 = 200.
        assert!((ra.w - 200.0).abs() < 0.01);
    }

    #[test]
    fn set_split_ratio_follows_path_into_nested_split() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let c = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5); // root split; a=A, b=B
        tree.split(b, c, Dir::Vertical, 0.5);   // b's leaf becomes a nested split at path [B]
        assert!(tree.set_split_ratio(&[Side::B], 0.75));
        // The nested split now favours its A child (b) at 0.75 of the height.
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 804.0, h: 604.0 }, 4.0);
        let rb = rects.iter().find(|(id, _)| *id == b).unwrap().1;
        // nested avail height 600 * 0.75 = 450.
        assert!((rb.h - 450.0).abs() < 0.01);
    }

    #[test]
    fn clamp_keeps_both_children_above_min_width() {
        // root 200 wide, gutter 4 => avail 196, min_w 40.
        // ratio 0.01 would give A=1.96px < 40 => clamp up to 40/196.
        let clamped = LayoutTree::clamp_ratio_for(
            Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 },
            Dir::Horizontal,
            4.0,
            40.0,
            10.0,
            0.01,
        );
        let a_w = 196.0 * clamped;
        let b_w = 196.0 - a_w;
        assert!(a_w >= 40.0 - 0.01);
        assert!(b_w >= 40.0 - 0.01);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test layout::tree -- --nocapture`
Expected: FAIL (`set_split_ratio`, `clamp_ratio_for` missing).

- [ ] **Step 3: Implement**

```rust
impl LayoutTree {
    pub fn set_split_ratio(&mut self, path: &[Side], ratio: f32) -> bool {
        let mut node = &mut self.root;
        for side in path {
            match node {
                Node::Split { a, b, .. } => {
                    node = match side {
                        Side::A => a.as_mut(),
                        Side::B => b.as_mut(),
                    };
                }
                Node::Leaf(_) => return false,
            }
        }
        match node {
            Node::Split { ratio: r, .. } => {
                *r = ratio.clamp(0.0, 1.0);
                true
            }
            Node::Leaf(_) => false,
        }
    }

    pub fn clamp_ratio_for(
        root: Rect,
        dir: Dir,
        gutter: f32,
        min_w: f32,
        min_h: f32,
        ratio: f32,
    ) -> f32 {
        let (avail, min_child) = match dir {
            Dir::Horizontal => ((root.w - gutter).max(1.0), min_w),
            Dir::Vertical => ((root.h - gutter).max(1.0), min_h),
        };
        let lo = (min_child / avail).clamp(0.0, 1.0);
        let hi = (1.0 - min_child / avail).clamp(0.0, 1.0);
        if lo > hi {
            0.5
        } else {
            ratio.clamp(lo, hi)
        }
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test layout::tree -- --nocapture`
Expected: PASS (9 tests total in tree.rs).

- [ ] **Step 5: Commit**

```bash
git add src/layout/tree.rs
git commit -m "feat: drag-resize ratio update with min-size clamp"
```

---

### Task 3: Border hit-test for drag-resize

**Files:**
- Create: `src/layout/hit.rs`
- Test: in `src/layout/hit.rs` under `#[cfg(test)]`

**Interfaces:**
- Consumes: `LayoutTree`, `Node`, `Rect`, `Dir`, `Side`, `split_rect` (Task 1).
- Produces:
  - `pub struct SplitHit { pub path: Vec<Side>, pub dir: Dir }`
  - `pub fn hit_test(tree: &LayoutTree, root: Rect, gutter: f32, cursor: (f32, f32), tol: f32) -> Option<SplitHit>` — return the deepest split whose divider strip (the `gutter`-wide band between children, widened by `tol` on each side) contains `cursor`.

- [ ] **Step 1: Write the failing tests in `src/layout/hit.rs`**

```rust
use crate::layout::tree::{Dir, LayoutTree, Node, Rect, Side};

pub struct SplitHit {
    pub path: Vec<Side>,
    pub dir: Dir,
}

pub fn hit_test(
    _tree: &LayoutTree,
    _root: Rect,
    _gutter: f32,
    _cursor: (f32, f32),
    _tol: f32,
) -> Option<SplitHit> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use crate::layout::tree::PaneId;

    #[test]
    fn cursor_on_vertical_divider_hits_horizontal_split() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        let root = Rect { x: 0.0, y: 0.0, w: 804.0, h: 600.0 };
        // Divider band sits at x in [400, 404].
        let hit = hit_test(&tree, root, 4.0, (402.0, 300.0), 3.0).expect("expected a hit");
        assert_eq!(hit.dir, Dir::Horizontal);
        assert!(hit.path.is_empty());
    }

    #[test]
    fn cursor_in_pane_interior_misses() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        let root = Rect { x: 0.0, y: 0.0, w: 804.0, h: 600.0 };
        assert!(hit_test(&tree, root, 4.0, (100.0, 300.0), 3.0).is_none());
    }

    #[test]
    fn nested_split_divider_is_found_with_path() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let c = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5); // root: a | b
        tree.split(b, c, Dir::Vertical, 0.5);   // b becomes b / c at path [B]
        let root = Rect { x: 0.0, y: 0.0, w: 804.0, h: 604.0 };
        // b's subtree occupies x in [404, 804], full height 604.
        // Its vertical split divider band sits at y in [300, 304].
        let hit = hit_test(&tree, root, 4.0, (600.0, 302.0), 3.0).expect("expected nested hit");
        assert_eq!(hit.dir, Dir::Vertical);
        assert_eq!(hit.path, vec![Side::B]);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test layout::hit -- --nocapture`
Expected: FAIL (`unimplemented!`).

- [ ] **Step 3: Implement `hit_test`**

```rust
use crate::layout::tree::split_rect;

pub fn hit_test(
    tree: &LayoutTree,
    root: Rect,
    gutter: f32,
    cursor: (f32, f32),
    tol: f32,
) -> Option<SplitHit> {
    let mut path = Vec::new();
    walk(&tree.root, root, gutter, cursor, tol, &mut path)
}

fn walk(
    node: &Node,
    rect: Rect,
    gutter: f32,
    cursor: (f32, f32),
    tol: f32,
    path: &mut Vec<Side>,
) -> Option<SplitHit> {
    if let Node::Split { dir, ratio, a, b } = node {
        let (ra, rb) = split_rect(rect, *dir, *ratio, gutter);

        // Recurse first so the DEEPEST matching split wins.
        path.push(Side::A);
        if let Some(hit) = walk(a, ra, gutter, cursor, tol, path) {
            return Some(hit);
        }
        path.pop();

        path.push(Side::B);
        if let Some(hit) = walk(b, rb, gutter, cursor, tol, path) {
            return Some(hit);
        }
        path.pop();

        // Divider band lies between ra and rb.
        let (cx, cy) = cursor;
        let on_divider = match dir {
            Dir::Horizontal => {
                let band_min = ra.x + ra.w - tol;
                let band_max = rb.x + tol;
                cx >= band_min && cx <= band_max && cy >= rect.y && cy <= rect.y + rect.h
            }
            Dir::Vertical => {
                let band_min = ra.y + ra.h - tol;
                let band_max = rb.y + tol;
                cy >= band_min && cy <= band_max && cx >= rect.x && cx <= rect.x + rect.w
            }
        };
        if on_divider {
            return Some(SplitHit { path: path.clone(), dir: *dir });
        }
    }
    None
}
```

- [ ] **Step 4: Run to verify pass**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test layout::hit -- --nocapture`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/layout/hit.rs
git commit -m "feat: border hit-test locating the split under the cursor"
```

---

### Task 4: App state — pane map, relayout, multi-pane frame building

**Files:**
- Modify: `src/app.rs` (introduce `App`; keep a `snapshot_cells` helper)
- Modify: `src/render/renderer.rs` (add `surface_size` accessor)
- Modify: `src/main.rs` (construct and use `App`; replace the single-session flow)
- Test: in `src/app.rs` a unit test for `rows_cols_for_rect`

**Interfaces:**
- Consumes: `LayoutTree`, `Rect`, `PaneId` (Task 1); `Session` (M1); `build_instances`, `QuadInstance`, `CellView` (M1); `GpuAtlas` (M1); `CellMetrics` (M1).
- Produces:
  - `pub struct App { pub sessions: SlotMap<PaneId, Session>, pub tree: LayoutTree, pub focus: PaneId, pub metrics: CellMetrics, pub rects: Vec<(PaneId, Rect)>, pub gutter: f32 }`
  - `App::new(root_rect: Rect, metrics: CellMetrics, spawn: impl FnMut(u16,u16)->Session) -> App` — spawns the first pane sized to the root rect.
  - `App::relayout(&mut self, root_rect: Rect)` — recompute rects, resize each pane's session to match its rect.
  - `App::rows_cols_for_rect(rect: Rect, m: &CellMetrics) -> (u16, u16)` — `cols=floor(w/cell_w).max(1)`, `rows=floor(h/cell_h).max(1)`.
  - `App::build_frame(&mut self, queue: &wgpu::Queue, atlas: &mut GpuAtlas) -> (Vec<QuadInstance>, Vec<QuadInstance>)` — concatenate every pane's bg+glyph quads at its rect origin; add the focused pane's block cursor; add a thin focus-border for the focused pane.
  - `pub fn snapshot_cells(session: &Session) -> (Vec<Vec<CellView>>, (usize, usize))` — renamed/kept from M1's `grid_to_cells`.
  - `Renderer::surface_size(&self) -> (u32, u32)`.

- [ ] **Step 1: Add `surface_size` to `Renderer`**

In `src/render/renderer.rs`, add inside `impl Renderer`:

```rust
    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
```

- [ ] **Step 2: Rewrite `src/app.rs`**

Keep the M1 snapshot logic as `snapshot_cells` (identical body to the current `grid_to_cells`, just renamed). Add the `App` struct and methods. Full file:

```rust
use crate::layout::tree::{LayoutTree, PaneId, Rect};
use crate::render::atlas_gpu::GpuAtlas;
use crate::render::grid_draw::{build_instances, CellView, QuadInstance};
use crate::terminal::session::Session;
use crate::text::metrics::CellMetrics;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use slotmap::SlotMap;

/// Snapshot one Term grid into CellView rows + cursor (line, col).
pub fn snapshot_cells(session: &Session) -> (Vec<Vec<CellView>>, (usize, usize)) {
    let term = session.term.lock().unwrap_or_else(|e| e.into_inner());
    let grid = term.grid();
    let actual_lines = term.screen_lines();
    let actual_cols = term.columns();
    let cursor_pt = grid.cursor.point;
    let cursor_line = cursor_pt.line.0.max(0) as usize;
    let cursor_col = cursor_pt.column.0;
    let mut out = Vec::with_capacity(actual_lines);
    for line in 0..actual_lines {
        let mut row = Vec::with_capacity(actual_cols);
        for col in 0..actual_cols {
            let cell = &grid[Point::new(Line(line as i32), Column(col))];
            row.push(CellView {
                ch: cell.c,
                fg: [0.85, 0.85, 0.85],
                bg: [0.05, 0.05, 0.06],
            });
        }
        out.push(row);
    }
    (out, (cursor_line, cursor_col))
}

pub struct App {
    pub sessions: SlotMap<PaneId, Session>,
    pub tree: LayoutTree,
    pub focus: PaneId,
    pub metrics: CellMetrics,
    pub rects: Vec<(PaneId, Rect)>,
    pub gutter: f32,
}

impl App {
    pub fn new(
        root_rect: Rect,
        metrics: CellMetrics,
        mut spawn: impl FnMut(u16, u16) -> Session,
    ) -> App {
        let (rows, cols) = Self::rows_cols_for_rect(root_rect, &metrics);
        let mut sessions: SlotMap<PaneId, Session> = SlotMap::with_key();
        let first = sessions.insert(spawn(rows, cols));
        let tree = LayoutTree::new(first);
        let gutter = 4.0;
        let rects = tree.compute_rects(root_rect, gutter);
        App { sessions, tree, focus: first, metrics, rects, gutter }
    }

    pub fn rows_cols_for_rect(rect: Rect, m: &CellMetrics) -> (u16, u16) {
        let cols = (rect.w / m.cell_w).floor().max(1.0) as u16;
        let rows = (rect.h / m.cell_h).floor().max(1.0) as u16;
        (rows, cols)
    }

    pub fn relayout(&mut self, root_rect: Rect) {
        self.rects = self.tree.compute_rects(root_rect, self.gutter);
        for (id, rect) in &self.rects {
            if let Some(session) = self.sessions.get_mut(*id) {
                let (rows, cols) = Self::rows_cols_for_rect(*rect, &self.metrics);
                session.resize(rows, cols);
            }
        }
    }

    pub fn build_frame(
        &mut self,
        queue: &wgpu::Queue,
        atlas: &mut GpuAtlas,
    ) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
        let mut all_bg: Vec<QuadInstance> = Vec::new();
        let mut all_glyphs: Vec<QuadInstance> = Vec::new();
        let metrics = self.metrics;
        // Clone rects to avoid holding &self.rects across &mut atlas calls.
        let rects = self.rects.clone();
        for (id, rect) in &rects {
            let session = match self.sessions.get(*id) {
                Some(s) => s,
                None => continue,
            };
            let (cells, (cur_line, cur_col)) = snapshot_cells(session);

            // Pre-resolve UVs for this pane's distinct glyphs.
            let mut uv_map: std::collections::HashMap<char, ([f32; 2], [f32; 2])> =
                std::collections::HashMap::new();
            for row in &cells {
                for cell in row {
                    if cell.ch != ' ' && cell.ch != '\0' && !uv_map.contains_key(&cell.ch) {
                        uv_map.insert(cell.ch, atlas.uv_for(queue, cell.ch));
                    }
                }
            }
            let default_uv = ([0.0f32; 2], [0.0f32; 2]);
            let (mut bg, glyphs) = build_instances(
                &cells,
                &metrics,
                [rect.x, rect.y],
                &|ch| uv_map.get(&ch).copied().unwrap_or(default_uv),
            );

            // Cursor block only for the focused pane.
            if *id == self.focus
                && cur_line < cells.len()
                && !cells.is_empty()
                && cur_col < cells[0].len()
            {
                let cx = rect.x + cur_col as f32 * metrics.cell_w;
                let cy = rect.y + cur_line as f32 * metrics.cell_h;
                bg.push(QuadInstance {
                    pos: [cx, cy],
                    size: [metrics.cell_w, metrics.cell_h],
                    uv_min: [0.0, 0.0],
                    uv_max: [0.0, 0.0],
                    color: [0.85, 0.85, 0.85, 1.0],
                });
            }

            all_bg.extend(bg);
            all_glyphs.extend(glyphs);
        }

        // Focus border: four thin quads around the focused pane's rect.
        if let Some((_, frect)) = rects.iter().find(|(id, _)| *id == self.focus) {
            let t = 2.0f32;
            let color = [0.30, 0.55, 0.90, 1.0];
            for q in border_quads(*frect, t, color) {
                all_bg.push(q);
            }
        }

        (all_bg, all_glyphs)
    }
}

fn border_quads(r: Rect, t: f32, color: [f32; 4]) -> [QuadInstance; 4] {
    let mk = |x: f32, y: f32, w: f32, h: f32| QuadInstance {
        pos: [x, y],
        size: [w, h],
        uv_min: [0.0, 0.0],
        uv_max: [0.0, 0.0],
        color,
    };
    [
        mk(r.x, r.y, r.w, t),               // top
        mk(r.x, r.y + r.h - t, r.w, t),     // bottom
        mk(r.x, r.y, t, r.h),               // left
        mk(r.x + r.w - t, r.y, t, r.h),     // right
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::metrics::CellMetrics;

    #[test]
    fn rows_cols_floor_and_clamp() {
        let m = CellMetrics { cell_w: 10.0, cell_h: 20.0, ascent: 15.0 };
        let (rows, cols) = App::rows_cols_for_rect(
            Rect { x: 0.0, y: 0.0, w: 105.0, h: 42.0 },
            &m,
        );
        assert_eq!(cols, 10); // floor(105/10)
        assert_eq!(rows, 2);  // floor(42/20)
        // Tiny rect clamps to at least 1x1.
        let (r2, c2) = App::rows_cols_for_rect(
            Rect { x: 0.0, y: 0.0, w: 3.0, h: 3.0 },
            &m,
        );
        assert_eq!((r2, c2), (1, 1));
    }
}
```

- [ ] **Step 3: Rewire `src/main.rs` to use `App` (single pane still, splits arrive in Task 5)**

Replace the single-`session` flow. Key changes: build the root rect from `renderer.surface_size()`, construct `App::new` with a spawn closure that clones the proxy AND registers each session's `redraw_pending` (see note below), and on `RedrawRequested` call `app.build_frame` then `renderer.draw_quads`. On `Resized` call `renderer.resize` then `app.relayout(root_rect)`. On keyboard input, write to `app.sessions[app.focus]`.

**Root rect:** `let (sw, sh) = renderer.surface_size(); let root_rect = Rect { x: 0.0, y: 0.0, w: sw as f32, h: sh as f32 };`

**Redraw coalescing with many panes:** each `Session` has its own `redraw_pending`. The `on_output` closure passed to `Session::spawn` sends `UserEvent::PtyOutput` on its own 0→1 transition (unchanged from M1 — the gate lives inside `Session`). On `UserEvent::PtyOutput`, call `window.request_redraw()`. On `RedrawRequested`, clear **every** live session's `redraw_pending` before snapshotting:

```rust
for (_, s) in app.sessions.iter() {
    s.redraw_pending.store(false, std::sync::atomic::Ordering::SeqCst);
}
```

(Do this at the top of the `RedrawRequested` arm, before `build_frame`.)

Provide the spawn closure to `App::new` as:

```rust
let proxy = event_loop.create_proxy();
let spawn = |rows: u16, cols: u16| -> Session {
    let p = proxy.clone();
    Session::spawn(rows, cols, "cmd.exe", move || {
        let _ = p.send_event(UserEvent::PtyOutput);
    })
};
let mut app = App::new(root_rect, metrics, spawn);
```

Keep `atlas` and `renderer` as before. `build_frame` needs `renderer.queue()` and `&mut atlas`; resolve the queue borrow before mutably borrowing atlas is not required here because `build_frame` takes `queue` by shared ref and `atlas` by mut ref as separate args — call `let (bg, glyphs) = app.build_frame(renderer.queue(), &mut atlas);` then `renderer.draw_quads(&bg, &glyphs, &atlas);` (the `renderer.queue()` shared borrow ends before `draw_quads`'s `&mut renderer`).

- [ ] **Step 4: Build + test + smoke**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test app:: -- --nocapture` → the `rows_cols_floor_and_clamp` test passes; all prior tests still pass.
Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo build` → clean.
Smoke: run `cargo run` in the background ~8s, terminate, confirm no panic / no wgpu validation error. (Visual: one pane with a focus border — human check.)

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/render/renderer.rs src/main.rs
git commit -m "feat: App owns pane map + multi-pane frame building with focus border"
```

---

### Task 5: Split / close / focus actions + keybindings

**Files:**
- Modify: `src/app.rs` (add action methods)
- Modify: `src/main.rs` (keymap dispatch)

**Interfaces:**
- Consumes: `App`, `LayoutTree`, `Dir`, `PaneId`, `Session` (prior tasks).
- Produces:
  - `App::split_focused(&mut self, dir: Dir, root_rect: Rect, spawn: impl FnOnce(u16,u16)->Session)` — spawn a new pane, insert into the slotmap, `tree.split(focus, new, dir, 0.5)`, `relayout`, set `focus = new`.
  - `App::close_focused(&mut self, root_rect: Rect)` — if more than one pane: drop the focused session (kills its shell), `tree.close(focus)`, `relayout`, set focus to any remaining leaf.
  - `App::focus_next(&mut self)` — cycle focus to the next pane id in `tree.pane_ids()` order.

- [ ] **Step 1: Implement action methods in `src/app.rs`**

```rust
use crate::layout::tree::Dir;

impl App {
    pub fn split_focused(
        &mut self,
        dir: Dir,
        root_rect: Rect,
        spawn: impl FnOnce(u16, u16) -> Session,
    ) {
        // Size the new pane to roughly half the focused rect (relayout corrects it).
        let focus_rect = self
            .rects
            .iter()
            .find(|(id, _)| *id == self.focus)
            .map(|(_, r)| *r)
            .unwrap_or(root_rect);
        let (rows, cols) = Self::rows_cols_for_rect(focus_rect, &self.metrics);
        let new_id = self.sessions.insert(spawn(rows.max(1), cols.max(1)));
        if self.tree.split(self.focus, new_id, dir, 0.5) {
            self.focus = new_id;
            self.relayout(root_rect);
        } else {
            // Split failed (focus not a leaf?) — roll back the orphan session.
            self.sessions.remove(new_id);
        }
    }

    pub fn close_focused(&mut self, root_rect: Rect) {
        if self.sessions.len() <= 1 {
            return; // never close the last pane
        }
        let closing = self.focus;
        if self.tree.close(closing) {
            self.sessions.remove(closing); // drops Session => PTY + reader thread end
            // Focus the first remaining leaf.
            if let Some(next) = self.tree.pane_ids().first().copied() {
                self.focus = next;
            }
            self.relayout(root_rect);
        }
    }

    pub fn focus_next(&mut self) {
        let ids = self.tree.pane_ids();
        if ids.len() <= 1 {
            return;
        }
        let idx = ids.iter().position(|&id| id == self.focus).unwrap_or(0);
        self.focus = ids[(idx + 1) % ids.len()];
    }
}
```

- [ ] **Step 2: Wire keybindings in `src/main.rs`**

In the `KeyboardInput` arm, BEFORE the text/write fallthrough, check for modifier chords. Track modifier state from `WindowEvent::ModifiersChanged` (store a `winit::keyboard::ModifiersState` in a `let mut mods`). Bindings (using logical key + Ctrl+Shift):

- Ctrl+Shift+D → `app.split_focused(Dir::Horizontal, root_rect, spawn_one)` (side-by-side)
- Ctrl+Shift+S → `app.split_focused(Dir::Vertical, root_rect, spawn_one)` (stacked)
- Ctrl+Shift+W → `app.close_focused(root_rect)`
- Ctrl+Shift+Tab (or Ctrl+Shift+O) → `app.focus_next()`

Where `spawn_one` is the same per-call spawn closure shape used in Task 4 (clone the proxy inside). After any of these, `window.request_redraw()` and `return` (do not fall through to writing bytes).

Recompute `root_rect` from `renderer.surface_size()` at the point of use (window size may have changed). Non-binding keys fall through to `app.sessions[app.focus].write(...)` exactly as M1 wrote to the single session — look up the focused session with `if let Some(s) = app.sessions.get_mut(app.focus)`.

Note: modifier detection — winit 0.29/0.30 delivers `WindowEvent::ModifiersChanged(mods)`; store `mods.state()`. Match a chord as: `state.control_key() && state.shift_key()` plus the logical key char/named key. Confirm exact method names against the installed winit version; adjust if the API differs.

- [ ] **Step 3: Build + smoke**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo build` → clean.
Run: `cargo test` → all prior tests still pass (no new unit test for actions — they are integration-tested via smoke + the layout unit tests already cover split/close correctness).
Smoke: `cargo run` background ~8s → no panic. (Human check: Ctrl+Shift+D splits into two shells side by side; Ctrl+Shift+W closes; focus border moves.)

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: split/close/focus pane actions with keybindings"
```

---

### Task 6: Mouse drag-resize + auto PTY resize debounce

**Files:**
- Modify: `src/main.rs` (mouse events, drag state, cursor icon)
- Modify: `src/app.rs` (helpers: `pane_at_point`, `apply_drag`)

**Interfaces:**
- Consumes: `App`, `hit_test`, `SplitHit`, `LayoutTree::set_split_ratio`, `LayoutTree::clamp_ratio_for` (prior tasks).
- Produces:
  - `App::pane_at_point(&self, p: (f32, f32)) -> Option<PaneId>` — the pane whose rect contains `p`.
  - `App::apply_drag(&mut self, hit: &SplitHit, cursor: (f32,f32), root_rect: Rect)` — recompute the dragged split's ratio from the cursor position within the split's own rect, clamped via `clamp_ratio_for`, then update `self.rects` only (visual). The debounced PTY resize is applied by the caller (see Step 3).

- [ ] **Step 1: Implement `pane_at_point` + `apply_drag` in `src/app.rs`**

```rust
use crate::layout::hit::SplitHit;
use crate::layout::tree::{split_rect, Node, Side};

impl App {
    pub fn pane_at_point(&self, p: (f32, f32)) -> Option<PaneId> {
        for (id, r) in &self.rects {
            if p.0 >= r.x && p.0 <= r.x + r.w && p.1 >= r.y && p.1 <= r.y + r.h {
                return Some(*id);
            }
        }
        None
    }

    /// Recompute the dragged split's ratio from the cursor and refresh rects.
    pub fn apply_drag(&mut self, hit: &SplitHit, cursor: (f32, f32), root_rect: Rect) {
        // Walk to the split's own rect following hit.path.
        let mut rect = root_rect;
        let mut node = &self.tree.root;
        for side in &hit.path {
            if let Node::Split { dir, ratio, a, b } = node {
                let (ra, rb) = split_rect(rect, *dir, *ratio, self.gutter);
                match side {
                    Side::A => { rect = ra; node = a; }
                    Side::B => { rect = rb; node = b; }
                }
            }
        }
        // `rect` is now the rect of the split whose ratio we adjust; `hit.dir` its orientation.
        let raw = match hit.dir {
            crate::layout::tree::Dir::Horizontal => {
                let avail = (rect.w - self.gutter).max(1.0);
                ((cursor.0 - rect.x) / avail).clamp(0.0, 1.0)
            }
            crate::layout::tree::Dir::Vertical => {
                let avail = (rect.h - self.gutter).max(1.0);
                ((cursor.1 - rect.y) / avail).clamp(0.0, 1.0)
            }
        };
        let min_w = self.metrics.cell_w * 2.0;
        let min_h = self.metrics.cell_h * 1.0;
        let clamped = LayoutTree::clamp_ratio_for(
            rect, hit.dir, self.gutter, min_w, min_h, raw,
        );
        self.tree.set_split_ratio(&hit.path, clamped);
        self.rects = self.tree.compute_rects(root_rect, self.gutter);
    }
}
```

- [ ] **Step 2: Track mouse state in `src/main.rs`**

Add before the event loop:

```rust
let mut cursor_pos = (0.0f32, 0.0f32);
let mut drag: Option<crate::layout::hit::SplitHit> = None;
let mut last_resize = std::time::Instant::now();
```

- [ ] **Step 3: Handle mouse events**

In the `WindowEvent` match:

- `WindowEvent::CursorMoved { position, .. }` →
  ```rust
  cursor_pos = (position.x as f32, position.y as f32);
  let (sw, sh) = renderer.surface_size();
  let root_rect = Rect { x: 0.0, y: 0.0, w: sw as f32, h: sh as f32 };
  if let Some(hit) = &drag {
      app.apply_drag(hit, cursor_pos, root_rect);
      // Debounce ConPTY resize to ~16ms during a live drag.
      if last_resize.elapsed().as_millis() >= 16 {
          app.relayout(root_rect); // resizes sessions to new rects
          last_resize = std::time::Instant::now();
      }
      window.request_redraw();
  } else {
      // Set the resize cursor when hovering a divider.
      let hovering = crate::layout::hit::hit_test(&app.tree, root_rect, app.gutter, cursor_pos, 3.0);
      let icon = match hovering.as_ref().map(|h| h.dir) {
          Some(crate::layout::tree::Dir::Horizontal) => winit::window::CursorIcon::EwResize,
          Some(crate::layout::tree::Dir::Vertical) => winit::window::CursorIcon::NsResize,
          None => winit::window::CursorIcon::Default,
      };
      window.set_cursor_icon(icon);
  }
  ```
  (Confirm `set_cursor_icon` / `CursorIcon` variant names against the installed winit version; `EwResize`/`NsResize` are the east-west / north-south resize cursors.)

- `WindowEvent::MouseInput { state, button, .. }` with `button == MouseButton::Left` →
  - On `ElementState::Pressed`: compute `root_rect`; `drag = hit_test(&app.tree, root_rect, app.gutter, cursor_pos, 3.0)`. If `drag.is_none()`, treat as a focus click: `if let Some(id) = app.pane_at_point(cursor_pos) { app.focus = id; window.request_redraw(); }`.
  - On `ElementState::Released`: if a drag was active, apply a final authoritative resize: `if drag.is_some() { let root_rect = ...; app.relayout(root_rect); window.request_redraw(); }` then `drag = None`.

- [ ] **Step 4: Build + smoke**

Run: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo build` → clean.
Run: `cargo test` → all tests pass.
Smoke: `cargo run` background ~8s → no panic / no validation error. (Human check: hover a border → cursor changes to ↔/↕; drag resizes both panes and the shells reflow; releasing settles the final grid size; clicking a pane focuses it; idle CPU ~0%.)

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: mouse drag-resize with debounced auto PTY resize and focus click"
```

---

## Self-Review

**Spec coverage (M2 subset, spec §6 + phasing steps 3–4):**
- Split tree (H/V splits, leaf=pane, ratios) — Task 1 ✓ (§6.1)
- New pane splits focused leaf, ratio 0.5 — Task 5 ✓ (§6.1)
- Close pane, sibling collapses up — Tasks 1 (tree) + 5 (action) ✓ (§6.1)
- Drag-resize on split dividers, orientation-aware, min-size clamp — Tasks 2, 3, 6 ✓ (§6.2)
- Cursor changes to ↔/↕ over a divider — Task 6 ✓ (§6.2)
- Rect→grid (`cols=floor(rect_w/cell_w)`), auto Term+PTY resize on relayout — Task 4 (`rows_cols_for_rect`, `relayout`) ✓ (§6.3)
- Live-drag ConPTY resize debounced (~16ms/settle) — Task 6 ✓ (§6.3)
- Window resize → root rect → whole tree relayout — Task 4 (`Resized` → `relayout`) ✓ (§6.3)
- Only active panes drawn, per-pane rect origin — Task 4 (`build_frame`) ✓ (§6.4, §7)
- Focused pane block cursor + visible focus indication — Task 4 (cursor + border) ✓ (§7)
- Idle 0% CPU preserved (per-session coalescing, redraw only on events) — Global Constraints + Task 4 Step 3 ✓ (§5)

Deferred to later milestones by design (NOT in M2 scope): sidebar workspaces, tab groups (M3); config/theme/keybinding customization (M4 — M2 hardcodes the chords); session persistence (M5); scrollback wheel, cursor styles, per-pane scissor clipping, ANSI colour mapping (later). Scissor clipping is intentionally omitted: each pane's grid is sized by `floor(rect/cell)` so glyphs stay within the rect apart from a sub-cell remainder in the gutter; acceptable for M2.

**Placeholder scan:** No "TBD"/"handle edge cases" left. The one prose-only region is Task 5 Step 2 / Task 6 Step 3 (winit event wiring): exact chord bytes, cursor variants, and method calls are named, with an explicit "confirm against installed winit version" caveat since winit 0.29/0.30 modifier + cursor APIs are the volatile surface. Every type and function they call is concrete and defined in an earlier task or M1.

**Type consistency:** `Rect{x,y,w,h}`, `Dir{Horizontal,Vertical}`, `Side{A,B}`, `PaneId` defined in Task 1 and used identically in Tasks 2–6. `SplitHit{path,dir}` from Task 3 consumed in Task 6. `LayoutTree::set_split_ratio(&[Side], f32)` (Task 2) called by `apply_drag` (Task 6). `App::rows_cols_for_rect` returns `(rows, cols)` as `(u16, u16)` matching `Session::resize(rows, cols)`. `build_frame` returns `(Vec<QuadInstance>, Vec<QuadInstance>)` matching `Renderer::draw_quads(bg, glyphs, atlas)`. `snapshot_cells` return shape matches M1's `grid_to_cells` (callers updated).

**API-drift caveat:** winit modifier/cursor APIs (`ModifiersChanged`, `ModifiersState::control_key()/shift_key()`, `set_cursor_icon`, `CursorIcon::EwResize/NsResize`) and alacritty grid `Dimensions` trait are the volatile surfaces; each integration task ends with a build+smoke checkpoint so a signature mismatch fails fast. Adjust to the resolved signatures, preserving behavior.
