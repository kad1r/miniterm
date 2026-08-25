# miniterm — Design Spec

**Date:** 2026-08-25
**Status:** Approved (design phase)

## 1. Overview

miniterm is a high-performance Windows terminal multiplexer. A single
resizable OS window hosts many independent shell sessions
(PowerShell / cmd / WSL — whatever the OS default is). Sessions are
organized into **workspaces** (groups) shown in a left sidebar. Within
a workspace, terminals are arranged as **tiling splits** whose borders
can be dragged with the mouse to resize in X and Y; terminals auto-resize
their PTY grid to match. Primary non-functional goal: minimal CPU / RAM /
GPU usage, especially at idle.

## 2. Goals / Non-Goals

**Goals (v1, full-featured)**
- One window, many terminals via real ConPTY shells.
- Left sidebar: create / rename / delete unlimited workspaces (groups).
- Tab groups within a workspace.
- Tiling split layout; mouse drag-resize on any pane border (X and Y).
- Auto PTY resize on window resize, split, close, and border drag.
- Config file (font, theme, default shell, keybindings, scrollback).
- Themes (16 ANSI + fg/bg/cursor/selection).
- Keybindings (configurable).
- Session persistence: restore workspaces, tabs, layout, cwd on relaunch.
- Event-driven rendering with damage tracking; ~0% idle CPU/GPU.

**Non-Goals (v1)**
- Restoring scrollback contents or live process state across restarts
  (shells are re-spawned at saved cwd).
- Pixel-accurate render regression tests.
- Free-floating overlapping panes (tiling only).
- Text selection/copy is a stretch, not required for v1 MVP wiring
  (see Phasing).

## 3. Technology Decisions

| Concern | Choice | Rationale |
|---|---|---|
| Language | Rust | Max performance, memory safety, mature terminal ecosystem |
| Window + input | `winit` | Cross-platform window/event loop, low overhead |
| GPU render | `wgpu` | Modern GPU API, single surface for all panes |
| Font shaping + raster | `swash` (+ `cosmic-text` if needed) | Fast glyph rasterization for the atlas |
| VT engine | `alacritty_terminal` crate | Battle-tested ANSI/VT parser, grid, scrollback |
| PTY | `portable-pty` (ConPTY backend) | Spawn/resize real Windows shells |
| Serialization | `serde` + `toml` (config) + `serde_json` (session) | Standard, simple |
| Collections | `slotmap` for panes | Stable pane ids across insert/remove |

Rendering approach mirrors Alacritty/WezTerm: shared glyph atlas,
instanced quad draw, damage-tracked redraw, event-driven (no constant
repaint).

## 4. Architecture

Single binary, internal modules:

```
src/
  main.rs            winit event loop, wiring
  app.rs             App state: workspaces, active ws, focus, dispatch
  terminal/
    session.rs       one terminal: Term + PTY reader thread + writer
    pty.rs           ConPTY spawn/resize via portable-pty
  layout/
    tree.rs          split tree (H/V splits, leaf=pane), ratios, relayout
    hit.rs           border hit-test for mouse drag-resize
  render/
    renderer.rs      wgpu device/surface/pipeline, scissor per pane
    atlas.rs         glyph cache -> GPU texture (shelf packer)
    grid_draw.rs     Term grid -> instanced bg + glyph quads
  ui/
    sidebar.rs       left workspace panel (list, add/rename/delete)
    tabs.rs          tab group bar per workspace
  config/
    config.rs        toml load/save: theme, keybindings, font, shell
    theme.rs         color palettes, embedded defaults
    session_store.rs persist workspaces + layout + cwd (json)
  input/
    keymap.rs        keybinding dispatch (string -> action)
```

### 4.1 Data model

```
App {
  workspaces: Vec<Workspace>,
  active: usize,
  config: Config,
}
Workspace {
  name: String,
  tabs: Vec<Tab>,
  active_tab: usize,
}
Tab {
  title: String,
  layout: LayoutTree,
  panes: SlotMap<PaneId, Session>,
  focus: PaneId,
}
LayoutTree node =
  Leaf(PaneId)
  | Split { dir: Horizontal | Vertical, ratio: f32, a: Box<Node>, b: Box<Node> }
Session {
  term: Arc<Mutex<Term>>,   // alacritty_terminal
  writer: PtyWriter,
  reader_thread: JoinHandle,
  cwd: PathBuf,
  shell: ShellKind,
  grid_size: (rows, cols),
}
```

## 5. Threading & Event Model

- **Main thread:** winit event loop + wgpu rendering. Reacts to input,
  resize, and PTY-output wakeups. Never polls.
- **Per-terminal PTY reader thread:** blocking read on the PTY; feeds
  bytes into that terminal's `Term` (behind `Mutex`); marks damage; wakes
  the main thread via `EventLoopProxy`. Blocked (≈0% CPU) when idle.
- 20 idle terminals = 20 blocked threads, negligible CPU.
- Writes (keyboard input) go from main thread to the PTY writer.

Redraw happens only when there is damage (input, PTY output, resize,
layout change). Idle → surface not re-presented → GPU sleeps.

## 6. Layout, Resize, Auto PTY Resize

### 6.1 Split tree
- Binary tree; leaves hold `PaneId`, internal nodes are splits with a
  `ratio` in (0,1) and a direction.
- Layout = recursive rect assignment. Root rect = window minus sidebar
  minus tab bar. Each split divides its parent rect along `dir` by
  `ratio`, minus a fixed gutter (default 4px, configurable).
- **New pane:** split the focused leaf — focused becomes `a`, new pane
  `b`, ratio 0.5.
- **Close pane:** remove the leaf; the sibling collapses up into the
  parent split's rect.

### 6.2 Drag-resize
- Each internal split's divider (the gutter between `a` and `b`) is a hit
  rect. `layout/hit.rs` walks the tree and returns the split under the
  cursor plus orientation; cursor changes to ↔ / ↕.
- Because the tree is nested, a pane's four edges map to the nearest
  ancestor split of the matching orientation. An edge on the outer window
  boundary has no such ancestor → no-op. This yields the "grab any of the
  four edges" feel without overlaps or gaps.
- Drag updates that split's `ratio`, clamped to a minimum pane size
  (min cols/rows, default 2×1). Only the affected subtree relayouts.

### 6.3 Rect → grid and auto resize
- `cols = floor(rect_w / cell_w)`, `rows = floor(rect_h / cell_h)` from
  font metrics.
- On any relayout (window resize, drag, split, close), each changed leaf
  recomputes `(rows, cols)`. If changed: `Term::resize(...)` +
  `pty.resize(rows, cols)`. Shell reflows via ConPTY.
- During a live drag, ConPTY resize calls are debounced (coalesced to
  ~16ms / on settle) to avoid spamming the shell.
- Window resize changes the root rect → whole tree relayouts → all
  visible panes resize automatically.

### 6.4 Render scope
- Only the active tab's panes are drawn and allocate GPU quads.
- Inactive tabs/workspaces keep their PTY reader threads alive (shells
  keep running) but are not drawn.

## 7. Rendering Pipeline

- **Metrics:** monospace `cell_w`/`cell_h` measured at font load.
- **Atlas:** first sighting of a glyph is rasterized (`swash`) and packed
  into a growing GPU texture via a shelf packer, then cached. Bold /
  italic / underline are separate glyph variants. Color comes from the
  cell attribute, not baked into the glyph → one raster serves all colors.
- **Draw (damage-tracked), instanced, two passes per frame:**
  1. Background quads — per-cell bg color (solid rects).
  2. Glyph quads — atlas UV + fg color, single instanced draw.
  - Cursor and selection are separate quads. Focused pane = filled block
    cursor; unfocused = hollow.
- All panes draw to one surface; each pane's rect is bounded by a
  scissor/viewport. One pass, many panes.
- Frame is drawn only when damage exists.
- Sidebar and tab bar are drawn with the same wgpu pipeline (rects +
  text), no separate UI framework.

## 8. Input & Keymap

- Keyboard event → check keybinding table (from config) first. On match,
  run the action (split_h, split_v, close_pane, focus_next, new_tab,
  next_tab, switch_workspace, new_workspace, ...).
- No match → write bytes to the focused pane's PTY writer. Ctrl / Alt /
  function keys are mapped to correct VT sequences.
- Mouse: left click → focus pane / select sidebar row / select tab. On a
  border → begin drag-resize. Scroll wheel → `Term` scrollback offset.

## 9. Config, Theme, Persistence

### 9.1 Config (`%APPDATA%/miniterm/config.toml`)
- Font family + size, theme name, default shell (pwsh/cmd/wsl),
  scrollback line limit, border/gutter px, keybinding map
  (`"ctrl+shift+d" = "split_horizontal"`).
- Loaded at startup. Missing → write defaults. Corrupt → fall back to
  defaults + log; never crash.

### 9.2 Theme
- 16 ANSI colors + fg/bg/cursor/selection, as a TOML table. A few
  embedded themes (dark default). `alacritty_terminal` color index maps
  to the theme palette.

### 9.3 Session persistence (`%APPDATA%/miniterm/session.json`)
- Saved: workspace list + names, each tab's layout tree (split dir/ratio),
  each leaf pane's cwd + shell kind.
- Not saved: scrollback contents, live process state (shells are
  re-spawned at saved cwd).
- Save trigger: debounced write on change + write on exit. On startup,
  restore; if absent, start with one empty workspace + one pane.
- **cwd tracking:** capture OSC 7 / OSC 9;9 sequences from the shell when
  emitted; otherwise fall back to spawn cwd.

## 10. Testing Strategy

- `layout/tree.rs`, `layout/hit.rs` → pure unit tests: rect math after
  split/close/resize, ratio clamp, min-size, border hit-test. No GPU,
  fast. Core logic covered first (TDD).
- `config`, `session_store` → serde round-trip tests (write→read equal);
  corrupt input → defaults.
- `terminal/pty.rs` → integration: spawn `cmd /c echo`, assert output
  reaches the grid; resize adjusts rows/cols.
- Render → headless wgpu smoke test (pipeline builds). Pixel validation is
  out of scope for v1; manual visual check.

## 11. Risks / Notes

- ConPTY cwd reporting depends on the shell emitting OSC; without it,
  restore uses spawn cwd only. Accepted.
- Live-drag ConPTY resize spam → debounce (settle / ~16ms).
- First-glyph raster hitch → pre-warm common ASCII at startup.

## 12. Phasing (build order)

1. Skeleton: winit window + wgpu clear + font metrics.
2. Single pane: ConPTY spawn + `alacritty_terminal` + atlas + grid draw +
   keyboard input. One working shell.
3. Layout tree + split/close + render multiple panes (scissor).
4. Mouse drag-resize + auto PTY resize + window resize.
5. Sidebar workspaces + tab groups.
6. Config + theme + keybindings.
7. Session persistence (save/restore).
8. Polish: scrollback wheel, cursor styles, pre-warm, debounce tuning.
