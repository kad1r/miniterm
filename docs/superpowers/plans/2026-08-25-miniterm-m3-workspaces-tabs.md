# miniterm M3 — Workspaces + Tabs (sidebar) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a left sidebar of named workspaces (groups), each holding one or more tabs, each tab holding its own tiling split layout of terminals — created/switched/renamed/deleted via mouse and keyboard, in-memory only.

**Architecture:** Refactor the flat `App` (which owns one split tree of panes) into a three-level model `App → Vec<Workspace> → Vec<Tab> → (LayoutTree + SlotMap<PaneId,Session>)`. The window is divided into a fixed-width left **sidebar**, a **tab bar** across the top of the remaining area, and the **pane area** (the active tab's split layout). Sidebar and tab bar are drawn with the existing wgpu quad+glyph pipeline (no new UI framework). Only the active tab's panes are rendered; inactive tabs keep their PTY reader threads alive but are not drawn. Idle-0%-CPU is preserved: redraws remain strictly event-driven.

**Tech Stack:** Rust 2021, winit 0.29, wgpu 0.19, alacritty_terminal 0.24, portable-pty, swash 0.1, slotmap, bytemuck.

**Spec:** docs/superpowers/specs/2026-08-25-miniterm-design.md (sections 4.1 data model, 6.4 render scope, 7 rendering, 8 input). This plan implements the spec's M3 milestone ("Sidebar workspaces + tab groups"), in-memory (persistence is M5).

## Global Constraints

- Toolchain: **stable-x86_64-pc-windows-gnu**. Do NOT change. cargo is NOT on the bash PATH — every cargo command MUST be prefixed with `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"`.
- Idle 0% CPU is sacred: `window.request_redraw()` may be called ONLY on genuine damage events (keyboard/mouse input, `UserEvent::PtyOutput`, resize, split/close, workspace/tab switch, rename edit). winit stays in `Wait` mode. Never request a redraw from `RedrawRequested` or `about_to_wait`. Hover that only changes the cursor icon must NOT request a redraw.
- Per-session `Arc<AtomicBool>` redraw coalescing is preserved. At the top of `RedrawRequested`, clear `redraw_pending` for the sessions being drawn (the active tab's sessions).
- Reader-thread / term lock recovery: always `.lock().unwrap_or_else(|e| e.into_inner())` (poison recovery). Never plain `.unwrap()` on the term mutex in non-test code.
- Font: resolved at runtime via `text::font_source::resolve_font` (already implemented). `FONT_PX = 18.0`. Metrics via `text::metrics::measure`. Do NOT hardcode a font path.
- Shell: `"cmd.exe"` (unchanged from M1/M2).
- gutter = 4.0px. Minimum pane size: 2 cols × 1 row.
- New panes get a stable border color from the existing golden-ratio `pane_color` generator. Preserve the M2 per-pane left/top boundary borders.
- No new heavy dependencies. Everything stays pure-Rust (gnu target links cleanly).
- Verify each task with `cargo build` (clean), `cargo test` (all green), and where noted a smoke `cargo run` (build first, then run the exe under `timeout` since the window blocks; exit code 143 = SIGTERM from timeout = OK, look for absence of `panicked`/`Validation Error` in stderr).

---

## File Structure

- `src/app.rs` → converted to a directory module `src/app/mod.rs` (the `App` wrapper: workspaces, active index, chrome geometry, chrome rendering, input routing, rename state).
- `src/app/tab.rs` — NEW. The `Tab` struct: owns one split layout (`sessions`, `tree`, `focus`, `rects`, `colors`, `color_seed`, `gutter`, `metrics`) and all pane operations moved verbatim from the current `App` (relayout, build_frame, split_focused, close_focused, focus_next, pane_at_point, apply_drag, snapshot_cells, border/color helpers).
- `src/app/workspace.rs` — NEW. `Workspace { name, tabs, active_tab }` + `Tab` construction helpers and workspace/tab navigation logic that is pure enough to unit-test.
- `src/render/text_draw.rs` — NEW. `build_text(...)` lays out an arbitrary `&str` into glyph `QuadInstance`s using the atlas + metrics (for sidebar/tab labels).
- `src/render/mod.rs` — add `pub mod text_draw;`.
- `src/main.rs` — rewire the event loop: chrome-aware root rect, chrome rendering in `RedrawRequested`, region-routed mouse, new keyboard chords, rename edit-mode routing.

Layout constants (define in `src/app/mod.rs`): `SIDEBAR_W = 180.0`, `TAB_BAR_H = 28.0`, `ROW_H = 24.0` (sidebar row height), chrome padding `PAD = 6.0`.

---

## Task 1: Extract `Tab` from `App` (behavior-preserving refactor)

Move all pane-owning state and methods out of `App` into a new `Tab` struct, leaving `App` as a thin wrapper that owns exactly one `Tab`. No behavior change: the app still shows one group of panes, all existing chords/mouse work, and the full existing test suite stays green.

**Files:**
- Delete-and-recreate: `src/app.rs` → `src/app/mod.rs`
- Create: `src/app/tab.rs`
- Modify: `src/main.rs` (route pane access through `app.active_tab_mut()` / `app.active_tab()`)

**Interfaces:**
- Consumes: `LayoutTree`, `Node`, `Dir`, `Side`, `Rect`, `PaneId`, `split_rect` (from `layout::tree`); `SplitHit`, `hit_test` (from `layout::hit`); `build_instances`, `CellView`, `GlyphInfo`, `QuadInstance` (from `render::grid_draw`); `GpuAtlas::uv_for` (from `render::atlas_gpu`); `Session` (from `terminal::session`); `CellMetrics` (from `text::metrics`).
- Produces (used by Tasks 2, 4, 5):
  - `pub struct Tab { pub sessions: SlotMap<PaneId, Session>, pub tree: LayoutTree, pub focus: PaneId, pub metrics: CellMetrics, pub rects: Vec<(PaneId, Rect)>, pub gutter: f32, pub colors: HashMap<PaneId,[f32;4]>, color_seed: u32 }`
  - `impl Tab`:
    - `pub fn new(root_rect: Rect, metrics: CellMetrics, gutter: f32, spawn: impl FnMut(u16,u16)->Session) -> Tab`
    - `pub fn rows_cols_for_rect(rect: Rect, m: &CellMetrics) -> (u16,u16)` (assoc fn)
    - `pub fn relayout(&mut self, root_rect: Rect)`
    - `pub fn build_frame(&mut self, queue: &wgpu::Queue, atlas: &mut GpuAtlas) -> (Vec<QuadInstance>, Vec<QuadInstance>)`
    - `pub fn split_focused(&mut self, dir: Dir, root_rect: Rect, spawn: impl FnOnce(u16,u16)->Session)`
    - `pub fn close_focused(&mut self, root_rect: Rect) -> bool` (returns true if a pane was closed; refuses when only one pane remains and returns false)
    - `pub fn focus_next(&mut self)`
    - `pub fn pane_at_point(&self, p: (f32,f32)) -> Option<PaneId>`
    - `pub fn apply_drag(&mut self, hit: &SplitHit, cursor: (f32,f32), root_rect: Rect)`
  - `pub fn snapshot_cells(session: &Session) -> (Vec<Vec<CellView>>, (usize,usize))` (free fn, moved verbatim)
  - `pub struct App { tab: Tab, pub metrics: CellMetrics, pub gutter: f32 }` with:
    - `pub fn new(root_rect: Rect, metrics: CellMetrics, spawn: impl FnMut(u16,u16)->Session) -> App`
    - `pub fn active_tab(&self) -> &Tab`
    - `pub fn active_tab_mut(&mut self) -> &mut Tab`

- [ ] **Step 1: Convert `app.rs` to a module directory**

Run:
```bash
cd "D:/Development/Cursor Apps/miniterm"
mkdir -p src/app
git mv src/app.rs src/app/mod.rs
```
(If `git mv` is unavailable, `mv src/app.rs src/app/mod.rs`.)

- [ ] **Step 2: Create `src/app/tab.rs` — move pane state + methods verbatim**

Cut the following out of `src/app/mod.rs` and paste into `src/app/tab.rs`, renaming `App` → `Tab` and adjusting only what the new signatures require:
- the `snapshot_cells` free function (unchanged),
- the entire struct that is currently `App` and its `impl` block, renamed to `Tab`,
- the free helpers `border_quads` is already gone; keep `pane_color` + `hsv_to_rgb` (move them here),
- the `#[cfg(test)] mod tests` with `rows_cols_floor_and_clamp` (move here; update `use super::*;`).

Required signature changes while moving:
- `Tab::new` gains an explicit `gutter: f32` parameter (instead of hardcoding `4.0`), and takes `metrics` by value. Body:
```rust
pub fn new(
    root_rect: Rect,
    metrics: CellMetrics,
    gutter: f32,
    mut spawn: impl FnMut(u16, u16) -> Session,
) -> Tab {
    let (rows, cols) = Self::rows_cols_for_rect(root_rect, &metrics);
    let mut sessions: SlotMap<PaneId, Session> = SlotMap::with_key();
    let first = sessions.insert(spawn(rows, cols));
    let tree = LayoutTree::new(first);
    let rects = tree.compute_rects(root_rect, gutter);
    let mut colors = std::collections::HashMap::new();
    colors.insert(first, pane_color(0));
    Tab { sessions, tree, focus: first, metrics, rects, gutter, colors, color_seed: 1 }
}
```
- `close_focused` returns `bool` and no longer contains the "never close the last pane" early-return based on `sessions.len()`. Instead it relies on `tree.close` returning false for the sole pane. Body:
```rust
pub fn close_focused(&mut self, root_rect: Rect) -> bool {
    let closing = self.focus;
    if self.tree.close(closing) {
        self.sessions.remove(closing); // drops Session => PTY + reader thread end
        self.colors.remove(&closing);
        if let Some(next) = self.tree.pane_ids().first().copied() {
            self.focus = next;
        }
        self.relayout(root_rect);
        true
    } else {
        false
    }
}
```
All other method bodies (`rows_cols_for_rect`, `relayout`, `build_frame`, `split_focused`, `focus_next`, `pane_at_point`, `apply_drag`) move **verbatim** — only the enclosing type name changes from `App` to `Tab`. Keep the M2 per-pane left/top border loop inside `build_frame` exactly as is.

Add at the top of `tab.rs`:
```rust
use crate::layout::hit::SplitHit;
use crate::layout::tree::{split_rect, Dir, LayoutTree, Node, PaneId, Rect, Side};
use crate::render::atlas_gpu::GpuAtlas;
use crate::render::grid_draw::{build_instances, CellView, GlyphInfo, QuadInstance};
use crate::terminal::session::Session;
use crate::text::metrics::CellMetrics;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use slotmap::SlotMap;
```

- [ ] **Step 3: Rewrite `src/app/mod.rs` as the thin `App` wrapper**

```rust
mod tab;
pub use tab::{snapshot_cells, Tab};

use crate::layout::tree::Rect;
use crate::terminal::session::Session;
use crate::text::metrics::CellMetrics;

pub struct App {
    tab: Tab,
    pub metrics: CellMetrics,
    pub gutter: f32,
}

impl App {
    pub fn new(
        root_rect: Rect,
        metrics: CellMetrics,
        spawn: impl FnMut(u16, u16) -> Session,
    ) -> App {
        let gutter = 4.0;
        let tab = Tab::new(root_rect, metrics, gutter, spawn);
        App { tab, metrics, gutter }
    }

    pub fn active_tab(&self) -> &Tab {
        &self.tab
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tab
    }
}
```

- [ ] **Step 4: Update `src/main.rs` call sites**

Replace every `app.<field/method>` that referred to pane state with a `Tab` access. Concretely:
- `app.sessions.get_mut(app.focus)` → 
```rust
let tab = app.active_tab_mut();
if let Some(session) = tab.sessions.get_mut(tab.focus) { session.write(b); }
```
- `app.relayout(root_rect)` → `app.active_tab_mut().relayout(root_rect);`
- `app.build_frame(queue, &mut atlas)` → `app.active_tab_mut().build_frame(queue, &mut atlas)`
- `app.split_focused(dir, root_rect, spawn_one)` → `app.active_tab_mut().split_focused(dir, root_rect, spawn_one)`
- `app.close_focused(root_rect)` → `app.active_tab_mut().close_focused(root_rect);` (ignore the returned bool for now)
- `app.focus_next()` → `app.active_tab_mut().focus_next()`
- `app.pane_at_point(cursor_pos)` → `app.active_tab().pane_at_point(cursor_pos)`
- `app.apply_drag(hit, cursor_pos, root_rect)` → `app.active_tab_mut().apply_drag(hit, cursor_pos, root_rect)`
- Redraw-pending clear loop: `for (_, s) in app.sessions.iter()` → `for (_, s) in app.active_tab().sessions.iter()`
- Hover/press hit_test uses `&app.tree`, `app.gutter` → `&app.active_tab().tree`, `app.active_tab().gutter`
- Focus click `app.focus = id` → `app.active_tab_mut().focus = id`

- [ ] **Step 5: Build and run the full test suite**

Run:
```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test 2>&1 | tail -8
```
Expected: all previously-passing tests PASS (the moved `rows_cols_floor_and_clamp` now lives under `app::tab::tests`). Build clean apart from the pre-existing `view`/`sampler` dead-code warning in `atlas_gpu.rs`.

- [ ] **Step 6: Smoke run**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo build 2>&1 | tail -2 && timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation" ; echo DONE
```
Expected: no `panic`/`Validation` lines before `DONE`.

- [ ] **Step 7: Commit**

```bash
git add src/app/mod.rs src/app/tab.rs src/main.rs && git commit -m "refactor: extract Tab from App (behavior-preserving)"
```

---

## Task 2: Workspace + Tab model and navigation actions

Replace the single `tab: Tab` in `App` with a `Vec<Workspace>` (each a `Vec<Tab>`), plus in-memory navigation actions. Still no chrome rendering; the active tab draws in the full window rect as before. All actions unit-tested.

**Files:**
- Create: `src/app/workspace.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/main.rs` (spawn closure plumbing so `App` can create new tabs/panes on demand)

**Interfaces:**
- Consumes: `Tab` (Task 1), `Rect`, `CellMetrics`, `Session`.
- Produces (used by Tasks 4, 5, 6):
  - `pub struct Workspace { pub name: String, pub tabs: Vec<Tab>, pub active_tab: usize }`
  - `App` new shape:
    - `pub struct App { pub workspaces: Vec<Workspace>, pub active: usize, pub metrics: CellMetrics, pub gutter: f32, root_rect: Rect }`
    - `pub fn new(root_rect, metrics, spawn) -> App` (creates one workspace "1" with one tab)
    - `pub fn active_ws(&self) -> &Workspace` / `active_ws_mut`
    - `pub fn active_tab(&self) -> &Tab` / `active_tab_mut` (active workspace's active tab)
    - `pub fn set_root_rect(&mut self, r: Rect)` (stores the pane-area rect; used by relayout on switch)
    - `pub fn new_workspace(&mut self, spawn: impl FnMut(u16,u16)->Session)` — append a workspace with one tab, make it active
    - `pub fn next_workspace(&mut self)` / `pub fn prev_workspace(&mut self)` — wrap-around; no-op if `len()<=1`
    - `pub fn switch_workspace(&mut self, idx: usize)` — bounds-checked; relayouts the newly active tab to `root_rect`
    - `pub fn new_tab(&mut self, spawn: impl FnMut(u16,u16)->Session)` — append tab to active workspace, make it active, relayout
    - `pub fn next_tab(&mut self)` / `pub fn prev_tab(&mut self)` — wrap-around within active workspace
    - `pub fn switch_tab(&mut self, idx: usize)` — bounds-checked; relayouts

Behavior notes:
- On any workspace/tab switch, call `relayout(root_rect)` on the newly-active tab so its panes match the current window size (the window may have resized while it was hidden).
- `new_workspace`/`new_tab` build their first `Tab` via `Tab::new(root_rect, metrics, gutter, spawn)`.

- [ ] **Step 1: Write failing tests for navigation**

Create `src/app/workspace.rs` with a test module. Because `Tab::new` spawns a real `Session` (a `cmd.exe` PTY), tests exercise navigation through `App` using a **test spawn closure** that still calls the real `Session::spawn` (cheap enough — M1's `echo_reaches_the_grid` already spawns cmd). Keep pane counts to 1 per tab to stay fast.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::tree::Rect;
    use crate::text::metrics::CellMetrics;
    use crate::terminal::session::Session;

    fn spawn_stub(rows: u16, cols: u16) -> Session {
        Session::spawn(rows.max(1), cols.max(1), "cmd.exe", || {})
    }

    fn test_app() -> App {
        let m = CellMetrics { cell_w: 10.0, cell_h: 20.0, ascent: 15.0 };
        App::new(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, m, spawn_stub)
    }

    #[test]
    fn starts_with_one_workspace_one_tab() {
        let app = test_app();
        assert_eq!(app.workspaces.len(), 1);
        assert_eq!(app.active, 0);
        assert_eq!(app.active_ws().tabs.len(), 1);
        assert_eq!(app.active_ws().active_tab, 0);
    }

    #[test]
    fn new_workspace_appends_and_activates() {
        let mut app = test_app();
        app.new_workspace(spawn_stub);
        assert_eq!(app.workspaces.len(), 2);
        assert_eq!(app.active, 1);
    }

    #[test]
    fn next_prev_workspace_wraps() {
        let mut app = test_app();
        app.new_workspace(spawn_stub); // active=1, len=2
        app.next_workspace();
        assert_eq!(app.active, 0);
        app.prev_workspace();
        assert_eq!(app.active, 1);
    }

    #[test]
    fn new_tab_appends_within_active_workspace() {
        let mut app = test_app();
        app.new_tab(spawn_stub);
        assert_eq!(app.active_ws().tabs.len(), 2);
        assert_eq!(app.active_ws().active_tab, 1);
    }

    #[test]
    fn tabs_are_isolated_per_workspace() {
        let mut app = test_app();
        app.new_tab(spawn_stub); // ws0 has 2 tabs
        app.new_workspace(spawn_stub); // ws1 has 1 tab
        assert_eq!(app.workspaces[0].tabs.len(), 2);
        assert_eq!(app.workspaces[1].tabs.len(), 1);
    }

    #[test]
    fn switch_tab_bounds_checked() {
        let mut app = test_app();
        app.switch_tab(99); // out of range -> no-op
        assert_eq!(app.active_ws().active_tab, 0);
    }
}
```

- [ ] **Step 2: Run tests, verify they fail to compile (App shape not updated yet)**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test app::workspace 2>&1 | tail -15
```
Expected: compile errors (App fields/methods missing).

- [ ] **Step 3: Implement `Workspace` and the new `App`**

In `src/app/workspace.rs`:
```rust
use crate::app::Tab;
use crate::layout::tree::Rect;
use crate::terminal::session::Session;
use crate::text::metrics::CellMetrics;

pub struct Workspace {
    pub name: String,
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
}

pub struct App {
    pub workspaces: Vec<Workspace>,
    pub active: usize,
    pub metrics: CellMetrics,
    pub gutter: f32,
    root_rect: Rect,
}

impl App {
    pub fn new(
        root_rect: Rect,
        metrics: CellMetrics,
        spawn: impl FnMut(u16, u16) -> Session,
    ) -> App {
        let gutter = 4.0;
        let tab = Tab::new(root_rect, metrics, gutter, spawn);
        let ws = Workspace { name: "1".to_string(), tabs: vec![tab], active_tab: 0 };
        App { workspaces: vec![ws], active: 0, metrics, gutter, root_rect }
    }

    pub fn set_root_rect(&mut self, r: Rect) { self.root_rect = r; }

    pub fn active_ws(&self) -> &Workspace { &self.workspaces[self.active] }
    pub fn active_ws_mut(&mut self) -> &mut Workspace { &mut self.workspaces[self.active] }

    pub fn active_tab(&self) -> &Tab {
        let ws = self.active_ws();
        &ws.tabs[ws.active_tab]
    }
    pub fn active_tab_mut(&mut self) -> &mut Tab {
        let a = self.active;
        let ws = &mut self.workspaces[a];
        let t = ws.active_tab;
        &mut ws.tabs[t]
    }

    pub fn new_workspace(&mut self, spawn: impl FnMut(u16, u16) -> Session) {
        let tab = Tab::new(self.root_rect, self.metrics, self.gutter, spawn);
        let name = format!("{}", self.workspaces.len() + 1);
        self.workspaces.push(Workspace { name, tabs: vec![tab], active_tab: 0 });
        self.active = self.workspaces.len() - 1;
    }

    pub fn next_workspace(&mut self) {
        if self.workspaces.len() <= 1 { return; }
        self.active = (self.active + 1) % self.workspaces.len();
        self.relayout_active();
    }
    pub fn prev_workspace(&mut self) {
        if self.workspaces.len() <= 1 { return; }
        self.active = (self.active + self.workspaces.len() - 1) % self.workspaces.len();
        self.relayout_active();
    }
    pub fn switch_workspace(&mut self, idx: usize) {
        if idx < self.workspaces.len() {
            self.active = idx;
            self.relayout_active();
        }
    }

    pub fn new_tab(&mut self, spawn: impl FnMut(u16, u16) -> Session) {
        let tab = Tab::new(self.root_rect, self.metrics, self.gutter, spawn);
        let ws = self.active_ws_mut();
        ws.tabs.push(tab);
        ws.active_tab = ws.tabs.len() - 1;
    }
    pub fn next_tab(&mut self) {
        let n = self.active_ws().tabs.len();
        if n <= 1 { return; }
        let ws = self.active_ws_mut();
        ws.active_tab = (ws.active_tab + 1) % n;
        self.relayout_active();
    }
    pub fn prev_tab(&mut self) {
        let n = self.active_ws().tabs.len();
        if n <= 1 { return; }
        let ws = self.active_ws_mut();
        ws.active_tab = (ws.active_tab + n - 1) % n;
        self.relayout_active();
    }
    pub fn switch_tab(&mut self, idx: usize) {
        if idx < self.active_ws().tabs.len() {
            self.active_ws_mut().active_tab = idx;
            self.relayout_active();
        }
    }

    fn relayout_active(&mut self) {
        let r = self.root_rect;
        self.active_tab_mut().relayout(r);
    }
}
```

- [ ] **Step 4: Replace `App` in `src/app/mod.rs`**

`src/app/mod.rs` becomes:
```rust
mod tab;
mod workspace;
pub use tab::{snapshot_cells, Tab};
pub use workspace::{App, Workspace};
```
(Delete the old `App` struct/impl that Task 1 put in `mod.rs`.)

- [ ] **Step 5: Update `src/main.rs`**

The initial `App::new(root_rect, metrics, spawn)` call is unchanged. The pane-access sites from Task 1 (`app.active_tab()` / `app.active_tab_mut()`) still work. Add one line after the window's initial size is known and after each `Resized`: `app.set_root_rect(root_rect);` so `App` always knows the current pane-area rect. (In Task 4 this becomes the chrome-adjusted rect; for now it is the full window rect.)

- [ ] **Step 6: Run tests**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test 2>&1 | tail -12
```
Expected: all green including the 6 new `app::workspace::tests`.

- [ ] **Step 7: Commit**

```bash
git add src/app/mod.rs src/app/workspace.rs src/main.rs && git commit -m "feat: workspace/tab data model with in-memory navigation actions"
```

---

## Task 3: `build_text` — lay out arbitrary strings into glyph quads

A pure helper to render UI label strings (workspace names, tab titles, the rename caret) using the same atlas + metrics as terminal cells. Left-to-right advance by `cell_w`; skips spaces; positions each glyph at the baseline exactly like `build_instances`.

**Files:**
- Create: `src/render/text_draw.rs`
- Modify: `src/render/mod.rs` (add `pub mod text_draw;`)

**Interfaces:**
- Consumes: `QuadInstance`, `GlyphInfo` (from `render::grid_draw`), `CellMetrics`.
- Produces (used by Task 4):
  - `pub fn build_text(text: &str, m: &CellMetrics, origin: [f32;2], color: [f32;3], atlas_uv: &dyn Fn(char) -> GlyphInfo) -> Vec<QuadInstance>`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_draw::GlyphInfo;
    use crate::text::metrics::CellMetrics;

    #[test]
    fn lays_out_left_to_right_and_skips_spaces() {
        let m = CellMetrics { cell_w: 10.0, cell_h: 20.0, ascent: 15.0 };
        let uv = |_c: char| GlyphInfo {
            uv_min: [0.0, 0.0],
            uv_max: [0.5, 0.5],
            px_size: [8.0, 12.0],
            offset: [1.0, 10.0],
        };
        let quads = build_text("A B", &m, [100.0, 50.0], [1.0, 1.0, 1.0], &uv);
        // 'A' and 'B' produce quads; the space does not.
        assert_eq!(quads.len(), 2);
        // 'A' at column 0: x + left = 100 + 1, y + ascent - top = 50 + 15 - 10.
        assert_eq!(quads[0].pos, [101.0, 55.0]);
        assert_eq!(quads[0].size, [8.0, 12.0]);
        // 'B' at column 2 (space consumed a column): x = 100 + 2*10 + 1 = 121.
        assert_eq!(quads[1].pos, [121.0, 55.0]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test render::text_draw 2>&1 | tail -10
```
Expected: FAIL (function not defined).

- [ ] **Step 3: Implement `build_text`**

```rust
use crate::render::grid_draw::{GlyphInfo, QuadInstance};
use crate::text::metrics::CellMetrics;

/// Lay out `text` left-to-right at `origin` (top-left of the first cell),
/// advancing one `cell_w` per character. Spaces and NULs produce no quad.
/// Glyphs are sized to their bitmap and placed at the baseline, matching
/// `grid_draw::build_instances`.
pub fn build_text(
    text: &str,
    m: &CellMetrics,
    origin: [f32; 2],
    color: [f32; 3],
    atlas_uv: &dyn Fn(char) -> GlyphInfo,
) -> Vec<QuadInstance> {
    let mut out = Vec::new();
    for (i, ch) in text.chars().enumerate() {
        if ch == ' ' || ch == '\0' {
            continue;
        }
        let x = origin[0] + i as f32 * m.cell_w;
        let y = origin[1];
        let g = atlas_uv(ch);
        out.push(QuadInstance {
            pos: [x + g.offset[0], y + m.ascent - g.offset[1]],
            size: g.px_size,
            uv_min: g.uv_min,
            uv_max: g.uv_max,
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    out
}
```

- [ ] **Step 4: Register module and run test**

Add `pub mod text_draw;` to `src/render/mod.rs`. Then:
```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test render::text_draw 2>&1 | tail -6
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/render/text_draw.rs src/render/mod.rs && git commit -m "feat: build_text helper for UI label glyph layout"
```

---

## Task 4: Chrome geometry + sidebar/tab-bar rendering

Reserve the sidebar and tab bar from the window, make the pane area the remaining rect, and draw both chrome regions with the quad pipeline. `App` gains a `build_frame` that composes chrome quads + the active tab's frame.

**Files:**
- Modify: `src/app/mod.rs` (chrome constants + geometry helpers)
- Modify: `src/app/workspace.rs` (`App::build_frame`, `App::pane_area_rect`)
- Modify: `src/main.rs` (compute chrome-adjusted root rect; call `app.build_frame`)

**Interfaces:**
- Consumes: `build_text` (Task 3), `Tab::build_frame`, `snapshot_cells`, `GpuAtlas::uv_for`, `QuadInstance`, `GlyphInfo`.
- Produces (used by Task 5):
  - constants `SIDEBAR_W`, `TAB_BAR_H`, `ROW_H`, `PAD` (in `app::mod`, `pub`)
  - `pub fn pane_area_rect(window: Rect) -> Rect` (free fn in `app::mod`): `Rect { x: SIDEBAR_W, y: TAB_BAR_H, w: window.w - SIDEBAR_W, h: window.h - TAB_BAR_H }` (clamped ≥ 0). The tab bar spans only the area to the right of the sidebar.
  - `pub fn sidebar_row_rect(i: usize) -> Rect` — the i-th workspace row: `Rect { x: 0, y: TAB_BAR_H_or_0?, ... }`. Decision: the sidebar spans the full window height starting at `y=0`; workspace rows start at `y = PAD + i*ROW_H`. The "+ new workspace" row is at index `workspaces.len()`.
  - `pub fn tab_chip_rect(i: usize) -> Rect` — the i-th tab chip in the tab bar: fixed width `CHIP_W = 120.0`, `Rect { x: SIDEBAR_W + PAD + i as f32 * CHIP_W, y: 0.0, w: CHIP_W - PAD, h: TAB_BAR_H }`. The "+ new tab" chip is at index `tabs.len()`.
  - `pub fn build_frame(&mut self, queue: &wgpu::Queue, atlas: &mut GpuAtlas, window: Rect) -> (Vec<QuadInstance>, Vec<QuadInstance>)` on `App`.

Colors (define as consts): sidebar bg `[0.10,0.10,0.12,1.0]`; active row/chip highlight `[0.20,0.22,0.28,1.0]`; inactive chip `[0.13,0.13,0.16,1.0]`; label text `[0.85,0.85,0.85]`; "+" text same.

- [ ] **Step 1: Add chrome constants + geometry free functions to `src/app/mod.rs`**

```rust
pub const SIDEBAR_W: f32 = 180.0;
pub const TAB_BAR_H: f32 = 28.0;
pub const ROW_H: f32 = 24.0;
pub const CHIP_W: f32 = 120.0;
pub const PAD: f32 = 6.0;

use crate::layout::tree::Rect;

pub fn pane_area_rect(window: Rect) -> Rect {
    Rect {
        x: SIDEBAR_W,
        y: TAB_BAR_H,
        w: (window.w - SIDEBAR_W).max(0.0),
        h: (window.h - TAB_BAR_H).max(0.0),
    }
}

pub fn sidebar_row_rect(i: usize) -> Rect {
    Rect { x: 0.0, y: PAD + i as f32 * ROW_H, w: SIDEBAR_W, h: ROW_H }
}

pub fn tab_chip_rect(i: usize) -> Rect {
    Rect { x: SIDEBAR_W + PAD + i as f32 * CHIP_W, y: 0.0, w: CHIP_W - PAD, h: TAB_BAR_H }
}
```
(These free functions are pure and can carry unit tests if the reviewer wants; a smoke test covers them via rendering.)

- [ ] **Step 2: Implement `App::build_frame` in `src/app/workspace.rs`**

The method: (1) let the active tab build its pane quads, (2) prepend/append chrome quads. Because `Tab::build_frame` already resolves its glyph UVs against the atlas, and chrome labels also need atlas UVs, resolve chrome label glyphs the same way (`atlas.uv_for`). Mutable-borrow discipline: call `Tab::build_frame` first (it borrows `atlas` mutably and returns owned Vecs), then build chrome quads with a fresh `atlas` borrow.

```rust
use crate::app::{pane_area_rect, sidebar_row_rect, tab_chip_rect, PAD, TAB_BAR_H, SIDEBAR_W};
use crate::render::atlas_gpu::GpuAtlas;
use crate::render::grid_draw::{GlyphInfo, QuadInstance};
use crate::render::text_draw::build_text;

const SIDEBAR_BG: [f32; 4] = [0.10, 0.10, 0.12, 1.0];
const HILITE: [f32; 4] = [0.20, 0.22, 0.28, 1.0];
const CHIP_BG: [f32; 4] = [0.13, 0.13, 0.16, 1.0];
const LABEL: [f32; 3] = [0.85, 0.85, 0.85];

impl App {
    pub fn build_frame(
        &mut self,
        queue: &wgpu::Queue,
        atlas: &mut GpuAtlas,
        window: Rect,
    ) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
        // 1. Active tab's pane frame.
        let (mut bg, mut glyphs) = self.active_tab_mut().build_frame(queue, atlas);

        // Helper to solid-fill a rect.
        let solid = |r: Rect, color: [f32; 4]| QuadInstance {
            pos: [r.x, r.y],
            size: [r.w, r.h],
            uv_min: [0.0, 0.0],
            uv_max: [0.0, 0.0],
            color,
        };

        // 2. Sidebar panel background (full height).
        bg.push(solid(Rect { x: 0.0, y: 0.0, w: SIDEBAR_W, h: window.h }, SIDEBAR_BG));

        // 3. Tab bar background (right of sidebar).
        bg.push(solid(
            Rect { x: SIDEBAR_W, y: 0.0, w: (window.w - SIDEBAR_W).max(0.0), h: TAB_BAR_H },
            SIDEBAR_BG,
        ));

        // Collect the label strings first (avoid borrow overlap), then resolve UVs.
        let ws_labels: Vec<String> =
            self.workspaces.iter().map(|w| w.name.clone()).collect();
        let active_ws = self.active;
        let tab_count = self.active_ws().tabs.len();
        let active_tab_idx = self.active_ws().active_tab;
        let metrics = self.metrics;

        // Resolve a UV map for all label characters via the atlas.
        let mut chars: Vec<char> = Vec::new();
        for s in &ws_labels { chars.extend(s.chars()); }
        chars.push('+');
        for i in 0..tab_count { chars.extend(format!("{}", i + 1).chars()); }
        let mut uv_map: std::collections::HashMap<char, GlyphInfo> =
            std::collections::HashMap::new();
        for c in chars {
            if c != ' ' && !uv_map.contains_key(&c) {
                uv_map.insert(c, atlas.uv_for(queue, c));
            }
        }
        let dg = GlyphInfo { uv_min: [0.0,0.0], uv_max: [0.0,0.0], px_size: [0.0,0.0], offset: [0.0,0.0] };
        let lookup = |c: char| uv_map.get(&c).copied().unwrap_or(dg);

        // 4. Sidebar workspace rows.
        for (i, name) in ws_labels.iter().enumerate() {
            let r = sidebar_row_rect(i);
            if i == active_ws {
                bg.push(solid(r, HILITE));
            }
            let ty = r.y + (r.h - metrics.cell_h) * 0.5;
            glyphs.extend(build_text(name, &metrics, [r.x + PAD, ty], LABEL, &lookup));
        }
        // "+ new workspace" row.
        let plus_r = sidebar_row_rect(ws_labels.len());
        let pty = plus_r.y + (plus_r.h - metrics.cell_h) * 0.5;
        glyphs.extend(build_text("+", &metrics, [plus_r.x + PAD, pty], LABEL, &lookup));

        // 5. Tab bar chips for the active workspace.
        for i in 0..tab_count {
            let r = tab_chip_rect(i);
            bg.push(solid(r, if i == active_tab_idx { HILITE } else { CHIP_BG }));
            let ty = r.y + (r.h - metrics.cell_h) * 0.5;
            let title = format!("{}", i + 1);
            glyphs.extend(build_text(&title, &metrics, [r.x + PAD, ty], LABEL, &lookup));
        }
        // "+ new tab" chip.
        let plus_chip = tab_chip_rect(tab_count);
        bg.push(solid(plus_chip, CHIP_BG));
        let cty = plus_chip.y + (plus_chip.h - metrics.cell_h) * 0.5;
        glyphs.extend(build_text("+", &metrics, [plus_chip.x + PAD, cty], LABEL, &lookup));

        (bg, glyphs)
    }

    pub fn pane_area(&self, window: Rect) -> Rect { pane_area_rect(window) }
}
```
Note: import `Rect` in `workspace.rs` if not already.

- [ ] **Step 3: Update `src/main.rs` to use chrome-aware rects**

- Compute the window rect from `renderer.surface_size()` as before, but the pane-area rect for the active tab is `app::pane_area_rect(window_rect)`.
- Replace the initial `root_rect` and every `Resized` handler to: build `window_rect`, then `let pane_rect = app::pane_area_rect(window_rect); app.set_root_rect(pane_rect); app.active_tab_mut().relayout(pane_rect);`
- In `RedrawRequested`: `let (bg, glyphs) = app.build_frame(renderer.queue(), &mut atlas, window_rect); renderer.draw_quads(&bg, &glyphs, &atlas);` (pass the full `window_rect`, not the pane rect — `build_frame` needs full window height for the sidebar).
- The redraw-pending clear loop still iterates `app.active_tab().sessions`.

- [ ] **Step 4: Build, test, smoke**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test 2>&1 | tail -6 && cargo build 2>&1 | tail -2 && timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation" ; echo DONE
```
Expected: tests green; no panic/validation before DONE. (Visual verification by the human: sidebar with row "1" highlighted, tab bar with chip "1" + "+", terminal offset into the pane area.)

- [ ] **Step 5: Commit**

```bash
git add src/app/mod.rs src/app/workspace.rs src/main.rs && git commit -m "feat: render sidebar + tab bar chrome, reserve pane area"
```

---

## Task 5: Mouse routing + chrome hit-testing

Route mouse clicks by region: sidebar rows/`+` switch or create workspaces; tab chips/`+` switch or create tabs; clicks in the pane area keep the existing focus/drag behavior (coordinates are already absolute, and pane rects already live inside the pane area, so pane hit-testing is unchanged).

**Files:**
- Modify: `src/app/mod.rs` (hit-test helpers)
- Modify: `src/main.rs` (mouse dispatch)

**Interfaces:**
- Consumes: `sidebar_row_rect`, `tab_chip_rect`, `SIDEBAR_W`, `TAB_BAR_H` (Task 4).
- Produces (used by Task 7 for rename/delete routing):
  - `pub enum ChromeHit { Workspace(usize), NewWorkspace, Tab(usize), NewTab, PaneArea }`
  - `pub fn chrome_hit(window: Rect, ws_count: usize, tab_count: usize, p: (f32,f32)) -> ChromeHit` (free fn in `app::mod`)

Logic:
- If `p.0 < SIDEBAR_W`: it is in the sidebar. Compute row index `i` from `p.1` via `sidebar_row_rect`. If `i < ws_count` → `Workspace(i)`; if `i == ws_count` → `NewWorkspace`; else `PaneArea` (miss → treat as nothing; return `PaneArea` but caller ignores since it is outside pane rects).
- Else if `p.1 < TAB_BAR_H`: it is in the tab bar. Find the chip index whose `tab_chip_rect(i)` contains `p`. If `i < tab_count` → `Tab(i)`; if `i == tab_count` → `NewTab`; else fall through.
- Else → `PaneArea`.

- [ ] **Step 1: Write failing tests for `chrome_hit`**

```rust
#[cfg(test)]
mod chrome_tests {
    use super::*;
    use crate::layout::tree::Rect;

    fn win() -> Rect { Rect { x: 0.0, y: 0.0, w: 1000.0, h: 700.0 } }

    #[test]
    fn click_first_sidebar_row_is_workspace_0() {
        // row 0 spans y in [PAD, PAD+ROW_H).
        let hit = chrome_hit(win(), 2, 1, (20.0, PAD + 2.0));
        assert!(matches!(hit, ChromeHit::Workspace(0)));
    }

    #[test]
    fn click_new_workspace_row() {
        let hit = chrome_hit(win(), 2, 1, (20.0, PAD + 2.0 * ROW_H + 2.0));
        assert!(matches!(hit, ChromeHit::NewWorkspace));
    }

    #[test]
    fn click_first_tab_chip() {
        // tab bar y < TAB_BAR_H, chip 0 starts at SIDEBAR_W + PAD.
        let hit = chrome_hit(win(), 1, 2, (SIDEBAR_W + PAD + 5.0, 10.0));
        assert!(matches!(hit, ChromeHit::Tab(0)));
    }

    #[test]
    fn click_new_tab_chip() {
        let x = SIDEBAR_W + PAD + 2.0 * CHIP_W + 5.0;
        let hit = chrome_hit(win(), 1, 2, (x, 10.0));
        assert!(matches!(hit, ChromeHit::NewTab));
    }

    #[test]
    fn click_pane_area() {
        let hit = chrome_hit(win(), 1, 1, (400.0, 400.0));
        assert!(matches!(hit, ChromeHit::PaneArea));
    }
}
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test app::chrome 2>&1 | tail -10
```
Expected: FAIL (type/function missing).

- [ ] **Step 3: Implement `ChromeHit` + `chrome_hit`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeHit {
    Workspace(usize),
    NewWorkspace,
    Tab(usize),
    NewTab,
    PaneArea,
}

pub fn chrome_hit(window: Rect, ws_count: usize, tab_count: usize, p: (f32, f32)) -> ChromeHit {
    if p.0 < SIDEBAR_W {
        // Sidebar: which row?
        if p.1 >= PAD {
            let i = ((p.1 - PAD) / ROW_H).floor() as usize;
            if i < ws_count {
                return ChromeHit::Workspace(i);
            } else if i == ws_count {
                return ChromeHit::NewWorkspace;
            }
        }
        return ChromeHit::PaneArea; // empty sidebar space
    }
    if p.1 < TAB_BAR_H {
        for i in 0..=tab_count {
            let r = tab_chip_rect(i);
            if p.0 >= r.x && p.0 <= r.x + r.w {
                return if i < tab_count { ChromeHit::Tab(i) } else { ChromeHit::NewTab };
            }
        }
        return ChromeHit::PaneArea;
    }
    ChromeHit::PaneArea
}
```

- [ ] **Step 4: Dispatch mouse in `src/main.rs`**

In the `MouseInput { state: Pressed, button: Left }` arm, BEFORE the existing border-drag / pane-focus logic, classify the click:
```rust
let window_rect = /* current full window rect from surface_size */;
let ws_count = app.workspaces.len();
let tab_count = app.active_ws().tabs.len();
match crate::app::chrome_hit(window_rect, ws_count, tab_count, cursor_pos) {
    crate::app::ChromeHit::Workspace(i) => { app.switch_workspace(i); window.request_redraw(); }
    crate::app::ChromeHit::NewWorkspace => {
        let spawn_one = /* build spawn closure (see below) */;
        app.new_workspace(spawn_one);
        window.request_redraw();
    }
    crate::app::ChromeHit::Tab(i) => { app.switch_tab(i); window.request_redraw(); }
    crate::app::ChromeHit::NewTab => {
        let spawn_one = /* build spawn closure */;
        app.new_tab(spawn_one);
        window.request_redraw();
    }
    crate::app::ChromeHit::PaneArea => {
        // existing behavior: border drag-resize hit_test, else pane focus click.
        // (unchanged code moves inside this arm)
    }
}
```
The spawn closure is the same pattern already used for `Ctrl+Shift+D` in main (clone the proxy, build a `move |rows, cols| Session::spawn(rows, cols, "cmd.exe", move || { let _ = pp.send_event(UserEvent::PtyOutput); })`). Factor it into a small local closure-builder to avoid duplication if convenient, but duplication is acceptable (FnMut/FnOnce capture differences).

Note: `new_workspace`/`new_tab` take `impl FnMut`, so a `move` closure that clones the proxy per call works.

- [ ] **Step 5: Build, test, smoke**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test 2>&1 | tail -6 && cargo build 2>&1 | tail -2 && timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation" ; echo DONE
```
Expected: green; no panic/validation. (Human visual: clicking "+" in sidebar adds a workspace row and switches to it; clicking a row switches; tab "+" adds a tab.)

- [ ] **Step 6: Commit**

```bash
git add src/app/mod.rs src/main.rs && git commit -m "feat: region-routed mouse for sidebar + tab bar"
```

---

## Task 6: Keyboard chords for workspaces + tabs

Add the new Ctrl+Shift / Ctrl chords without disturbing the existing M2 chords (split D/S, close W, pane focus Tab/O). All wiring lives in `src/main.rs`'s keyboard handler.

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `App::new_workspace`, `next_workspace`, `prev_workspace`, `new_tab`, `next_tab`, `prev_tab`.
- Winit keys: `Key::Named(NamedKey::PageUp)`, `Key::Named(NamedKey::PageDown)`, `Key::Character("n"/"t")`.

Chord table (all require `mods.control_key() && mods.shift_key()` unless noted):
- `Ctrl+Shift+N` → `new_workspace`
- `Ctrl+Shift+PageDown` → `next_workspace`; `Ctrl+Shift+PageUp` → `prev_workspace`
- `Ctrl+Shift+T` → `new_tab`
- `Ctrl+PageDown` (Ctrl only, no Shift) → `next_tab`; `Ctrl+PageUp` → `prev_tab`

Keep existing D/S/W/Tab/O handling. Because `next_tab` uses Ctrl-only, add a separate branch guarded by `mods.control_key() && !mods.shift_key()` for PageUp/PageDown.

- [ ] **Step 1: Add the Ctrl+Shift branches**

Inside the existing `if mods.control_key() && mods.shift_key() { ... match &event.logical_key { ... } }` block, add arms:
```rust
Key::Character(s) if s.as_str().eq_ignore_ascii_case("n") => {
    let spawn_one = /* spawn closure as in Task 5 */;
    app.new_workspace(spawn_one);
    true
}
Key::Character(s) if s.as_str().eq_ignore_ascii_case("t") => {
    let spawn_one = /* spawn closure */;
    app.new_tab(spawn_one);
    true
}
Key::Named(NamedKey::PageDown) => { app.next_workspace(); true }
Key::Named(NamedKey::PageUp) => { app.prev_workspace(); true }
```
(These join the existing D/S/W/Tab/O arms; on `true` the block already does `window.request_redraw(); return;`.)

- [ ] **Step 2: Add a Ctrl-only (no Shift) branch for tab switching**

Immediately after the Ctrl+Shift block, add:
```rust
if mods.control_key() && !mods.shift_key() {
    let handled = match &event.logical_key {
        Key::Named(NamedKey::PageDown) => { app.next_tab(); true }
        Key::Named(NamedKey::PageUp) => { app.prev_tab(); true }
        _ => false,
    };
    if handled {
        window.request_redraw();
        return;
    }
}
```

- [ ] **Step 3: Build, test, smoke**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test 2>&1 | tail -4 && cargo build 2>&1 | tail -2 && timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation" ; echo DONE
```
Expected: green; no panic/validation. (Human visual: Ctrl+Shift+N/T create; Ctrl+Shift+PageUp/Dn switch workspaces; Ctrl+PageUp/Dn switch tabs.)

- [ ] **Step 4: Commit**

```bash
git add src/main.rs && git commit -m "feat: keyboard chords for workspace/tab create + switch"
```

---

## Task 7: Rename (edit-mode) + delete workspace

Double-click a sidebar row → inline edit its name (typed characters append, Backspace deletes, Enter commits, Esc cancels). Right-click a sidebar row → delete that workspace (refuse when only one remains; its tabs/panes drop, ending their PTYs). While in edit mode, keyboard input is captured into the name instead of being written to the focused pane.

**Files:**
- Modify: `src/app/workspace.rs` (rename state + methods)
- Modify: `src/app/mod.rs` (nothing new expected; `chrome_hit` reused)
- Modify: `src/main.rs` (double-click / right-click detection, edit-mode keyboard routing, caret rendering hook)

**Interfaces:**
- Produces:
  - `App` gains `pub editing: Option<usize>` (index of the workspace being renamed) and `edit_buf: String`.
  - `pub fn begin_rename(&mut self, ws: usize)` — sets `editing`, seeds `edit_buf` from the current name.
  - `pub fn rename_push(&mut self, ch: char)` / `pub fn rename_backspace(&mut self)`
  - `pub fn rename_commit(&mut self)` — writes `edit_buf` into the workspace name (if non-empty), clears `editing`.
  - `pub fn rename_cancel(&mut self)` — clears `editing` without changing the name.
  - `pub fn delete_workspace(&mut self, ws: usize)` — removes it if `workspaces.len() > 1`; fixes `active` to stay in range; relayouts the newly-active tab. Refuses (no-op) if only one workspace remains.
  - `App::build_frame` shows the edit buffer + a trailing caret `_` for the row in `editing`.

- [ ] **Step 1: Write failing tests for rename/delete logic**

```rust
#[test]
fn rename_commit_updates_name() {
    let mut app = test_app();
    app.begin_rename(0);
    app.rename_backspace(); // clear seeded "1"
    for c in "work".chars() { app.rename_push(c); }
    app.rename_commit();
    assert_eq!(app.workspaces[0].name, "work");
    assert!(app.editing.is_none());
}

#[test]
fn rename_cancel_keeps_old_name() {
    let mut app = test_app();
    let old = app.workspaces[0].name.clone();
    app.begin_rename(0);
    app.rename_push('x');
    app.rename_cancel();
    assert_eq!(app.workspaces[0].name, old);
    assert!(app.editing.is_none());
}

#[test]
fn delete_workspace_refuses_last() {
    let mut app = test_app();
    app.delete_workspace(0);
    assert_eq!(app.workspaces.len(), 1);
}

#[test]
fn delete_workspace_removes_and_fixes_active() {
    let mut app = test_app();
    app.new_workspace(spawn_stub); // active=1, len=2
    app.delete_workspace(1);
    assert_eq!(app.workspaces.len(), 1);
    assert_eq!(app.active, 0);
}
```

- [ ] **Step 2: Run tests, verify fail**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test app::workspace 2>&1 | tail -12
```
Expected: FAIL (methods/fields missing).

- [ ] **Step 3: Implement rename/delete on `App`**

Add fields to `App` (init `editing: None`, `edit_buf: String::new()` in `App::new`). Methods:
```rust
pub fn begin_rename(&mut self, ws: usize) {
    if ws < self.workspaces.len() {
        self.edit_buf = self.workspaces[ws].name.clone();
        self.editing = Some(ws);
    }
}
pub fn rename_push(&mut self, ch: char) {
    if self.editing.is_some() && !ch.is_control() {
        self.edit_buf.push(ch);
    }
}
pub fn rename_backspace(&mut self) {
    if self.editing.is_some() { self.edit_buf.pop(); }
}
pub fn rename_commit(&mut self) {
    if let Some(ws) = self.editing.take() {
        let name = self.edit_buf.trim();
        if !name.is_empty() {
            self.workspaces[ws].name = name.to_string();
        }
    }
    self.edit_buf.clear();
}
pub fn rename_cancel(&mut self) {
    self.editing = None;
    self.edit_buf.clear();
}
pub fn delete_workspace(&mut self, ws: usize) {
    if self.workspaces.len() <= 1 || ws >= self.workspaces.len() { return; }
    self.workspaces.remove(ws); // drops its Tabs => Sessions => PTYs end
    if self.active >= self.workspaces.len() {
        self.active = self.workspaces.len() - 1;
    }
    self.editing = None;
    self.edit_buf.clear();
    self.relayout_active();
}
```

- [ ] **Step 4: Show the edit buffer in `App::build_frame`**

In the sidebar-row loop, when `Some(i) == self.editing`, render `edit_buf` + `"_"` instead of the stored name:
```rust
let shown = if self.editing == Some(i) {
    format!("{}_", self.edit_buf)
} else {
    name.clone()
};
```
Also extend the label `chars` collection to include `edit_buf` characters + `'_'` so their glyphs are resolved. (Simplest: when building `chars`, if `editing.is_some()`, `chars.extend(self.edit_buf.chars()); chars.push('_');`.)

- [ ] **Step 5: Wire double-click / right-click / edit-mode keys in `src/main.rs`**

- **Double-click detection:** track `last_click: Option<(std::time::Instant,(f32,f32))>`. On a left press in the sidebar over a `Workspace(i)`, if the previous click was within 400ms and ~same position, treat as double-click → `app.begin_rename(i); window.request_redraw();` (and skip the single-click switch for this event). Otherwise perform the normal `switch_workspace(i)` and record `last_click`.
- **Right-click delete:** add a `MouseInput { button: Right, state: Pressed }` arm; if `chrome_hit` is `Workspace(i)` → `app.delete_workspace(i); window.request_redraw();`.
- **Edit-mode keyboard routing:** at the very TOP of `KeyboardInput` handling (before the Ctrl+Shift chord checks and before pane write), if `app.editing.is_some()`:
```rust
match &event.logical_key {
    Key::Named(NamedKey::Enter) => { app.rename_commit(); window.request_redraw(); return; }
    Key::Named(NamedKey::Escape) => { app.rename_cancel(); window.request_redraw(); return; }
    Key::Named(NamedKey::Backspace) => { app.rename_backspace(); window.request_redraw(); return; }
    _ => {
        if let Some(text) = &event.text {
            for ch in text.chars() { app.rename_push(ch); }
            window.request_redraw();
        }
        return;
    }
}
```
This fully captures input while editing; nothing reaches the PTY.

- [ ] **Step 6: Build, test, smoke**

```bash
cd "D:/Development/Cursor Apps/miniterm" && export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin" && cargo test 2>&1 | tail -12 && cargo build 2>&1 | tail -2 && timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation" ; echo DONE
```
Expected: all green (including the 4 new rename/delete tests); no panic/validation. (Human visual: double-click a sidebar row → caret appears, typing edits the name, Enter commits, Esc cancels; right-click deletes a workspace when more than one exists; idle CPU ~0%.)

- [ ] **Step 7: Commit**

```bash
git add src/app/workspace.rs src/app/mod.rs src/main.rs && git commit -m "feat: rename (edit-mode) and delete workspaces via mouse + keyboard"
```

---

## Idle-0%-CPU checklist (verify during final review)

- Every `window.request_redraw()` is on a genuine damage event: `UserEvent::PtyOutput`, `Resized`, keyboard input, mouse press that changes state (focus/switch/create/rename/delete), and during an active border drag. Hover (cursor-icon change) requests NO redraw.
- `RedrawRequested` never requests another redraw. `about_to_wait` is unused / requests nothing. winit stays in `Wait` mode.
- At the top of `RedrawRequested`, only the active tab's sessions have `redraw_pending` cleared. Inactive sessions keep their flag; they are not drawn, and a still-set flag simply means no further wakeups until the tab is shown again (acceptable — they are invisible). On switch, the switch itself requests a redraw.
- Inactive workspaces/tabs keep their PTY reader threads alive (blocked on read ≈ 0% CPU) — Sessions are only dropped on close/delete.

## Self-Review Notes (author)

- **Spec coverage:** sidebar workspaces (§4.1, create/rename/delete) → Tasks 4,5,7; tab groups (§4.1) → Tasks 2,4,5,6; render scope active-tab-only (§6.4) → Task 4 + idle checklist; sidebar/tab bar via same pipeline (§7) → Tasks 3,4; input keymap + mouse routing (§8) → Tasks 5,6,7. Persistence (§9.3) intentionally deferred to M5 (in-memory only) per approved scope.
- **Type consistency:** `Tab` field/method names in Task 1 match their consumers in Tasks 2/4/5. `App` method names (`new_workspace`, `next_workspace`, `switch_tab`, etc.) are identical across Tasks 2/5/6. `build_text` signature in Task 3 matches its call in Task 4. `chrome_hit`/`ChromeHit` in Task 5 match Task 7's reuse.
- **No placeholders:** every code step has concrete code; the two `/* spawn closure */` markers reference the exact existing pattern in `main.rs` (Ctrl+Shift+D arm) and are the same closure shape already in the codebase.
