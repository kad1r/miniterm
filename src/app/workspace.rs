use crate::app::{pane_area_rect, sidebar_row_rect, PAD, SIDEBAR_W};
use crate::layout::hit::SplitHit;
use crate::layout::tree::{split_rect, Dir, LayoutTree, Node, PaneId, Rect, Side};
use crate::render::atlas_gpu::GpuAtlas;
use crate::render::grid_draw::{build_instances, CellView, GlyphInfo, QuadInstance};
use crate::render::text_draw::build_text;
use crate::terminal::session::Session;
use crate::text::metrics::CellMetrics;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use slotmap::SlotMap;

/// Snapshot one Term grid into CellView rows + cursor (line, col).
pub fn snapshot_cells(
    session: &Session,
    fg: crate::config::Rgb,
    bg: crate::config::Rgb,
) -> (Vec<Vec<CellView>>, (usize, usize)) {
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
            row.push(CellView { ch: cell.c, fg, bg });
        }
        out.push(row);
    }
    (out, (cursor_line, cursor_col))
}

/// A named group holding a tiling split of terminal panes.
pub struct Workspace {
    pub name: String,
    pub sessions: SlotMap<PaneId, Session>,
    pub tree: LayoutTree,
    pub focus: PaneId,
    pub metrics: CellMetrics,
    pub rects: Vec<(PaneId, Rect)>,
    pub gutter: f32,
    /// Stable per-pane border color, assigned at creation.
    pub colors: std::collections::HashMap<PaneId, [f32; 4]>,
    /// Monotonic counter feeding the golden-ratio hue generator.
    color_seed: u32,
}

/// Distinct, visually-spread border color from a counter (golden-ratio hue walk).
fn pane_color(n: u32) -> [f32; 4] {
    let hue = (n as f32 * 0.618_034).fract(); // 0..1
    let (r, g, b) = hsv_to_rgb(hue, 0.65, 0.95);
    [r, g, b, 1.0]
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match (i as i32).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

impl Workspace {
    pub fn new(
        name: String,
        root_rect: Rect,
        metrics: CellMetrics,
        gutter: f32,
        mut spawn: impl FnMut(u16, u16) -> Session,
    ) -> Workspace {
        let (rows, cols) = Self::rows_cols_for_rect(root_rect, &metrics);
        let mut sessions: SlotMap<PaneId, Session> = SlotMap::with_key();
        let first = sessions.insert(spawn(rows, cols));
        let tree = LayoutTree::new(first);
        let rects = tree.compute_rects(root_rect, gutter);
        let mut colors = std::collections::HashMap::new();
        colors.insert(first, pane_color(0));
        Workspace { name, sessions, tree, focus: first, metrics, rects, gutter, colors, color_seed: 1 }
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
        theme: crate::config::Theme,
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
            let (cells, (cur_line, cur_col)) = snapshot_cells(session, theme.foreground, theme.background);

            // Pre-resolve UVs + glyph placement for this pane's distinct glyphs.
            let mut uv_map: std::collections::HashMap<char, GlyphInfo> =
                std::collections::HashMap::new();
            for row in &cells {
                for cell in row {
                    if cell.ch != ' ' && cell.ch != '\0' && !uv_map.contains_key(&cell.ch) {
                        uv_map.insert(cell.ch, atlas.uv_for(queue, cell.ch));
                    }
                }
            }
            let default_glyph = GlyphInfo {
                uv_min: [0.0, 0.0],
                uv_max: [0.0, 0.0],
                px_size: [0.0, 0.0],
                offset: [0.0, 0.0],
            };
            let (mut bg, glyphs) = build_instances(
                &cells,
                &metrics,
                [rect.x, rect.y],
                &|ch| uv_map.get(&ch).copied().unwrap_or(default_glyph),
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
                    color: [theme.cursor[0], theme.cursor[1], theme.cursor[2], 1.0],
                });
            }

            all_bg.extend(bg);
            all_glyphs.extend(glyphs);
        }

        // Per-pane boundary borders: a thin colored line on the left edge of
        // panes not flush to the pane area's left, and on the top edge of panes
        // not flush to the top (stacked splits). Leftmost/topmost panes get no
        // border on that side.
        let t = 2.0f32;
        for (id, rect) in &rects {
            let color = self.colors.get(id).copied().unwrap_or([0.5, 0.5, 0.5, 1.0]);
            if rect.x > SIDEBAR_W + 0.5 {
                all_bg.push(QuadInstance {
                    pos: [rect.x - t, rect.y],
                    size: [t, rect.h],
                    uv_min: [0.0, 0.0],
                    uv_max: [0.0, 0.0],
                    color,
                });
            }
            if rect.y > 0.5 {
                all_bg.push(QuadInstance {
                    pos: [rect.x, rect.y - t],
                    size: [rect.w, t],
                    uv_min: [0.0, 0.0],
                    uv_max: [0.0, 0.0],
                    color,
                });
            }
        }

        (all_bg, all_glyphs)
    }

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
            self.colors.insert(new_id, pane_color(self.color_seed));
            self.color_seed += 1;
            self.focus = new_id;
            self.relayout(root_rect);
        } else {
            // Split failed (focus not a leaf?) — roll back the orphan session.
            self.sessions.remove(new_id);
        }
    }

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

    pub fn focus_next(&mut self) {
        let ids = self.tree.pane_ids();
        if ids.len() <= 1 {
            return;
        }
        let idx = ids.iter().position(|&id| id == self.focus).unwrap_or(0);
        self.focus = ids[(idx + 1) % ids.len()];
    }

    pub fn pane_at_point(&self, p: (f32, f32)) -> Option<PaneId> {
        for (id, r) in &self.rects {
            if p.0 >= r.x && p.0 <= r.x + r.w && p.1 >= r.y && p.1 <= r.y + r.h {
                return Some(*id);
            }
        }
        None
    }

    /// Recompute the dragged split's ratio from the cursor and refresh rects (visual only).
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
        // `rect` is now the rect of the split whose ratio we adjust; `hit.dir` is its orientation.
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

pub struct App {
    pub workspaces: Vec<Workspace>,
    pub active: usize,
    pub metrics: CellMetrics,
    pub gutter: f32,
    root_rect: Rect,
    pub theme: crate::config::Theme,
    pub editing: Option<usize>,
    pub edit_buf: String,
}

impl App {
    pub fn new(
        root_rect: Rect,
        metrics: CellMetrics,
        theme: crate::config::Theme,
        spawn: impl FnMut(u16, u16) -> Session,
    ) -> App {
        let gutter = 4.0;
        let ws = Workspace::new("1".to_string(), root_rect, metrics, gutter, spawn);
        App {
            workspaces: vec![ws],
            active: 0,
            metrics,
            gutter,
            root_rect,
            theme,
            editing: None,
            edit_buf: String::new(),
        }
    }

    pub fn set_root_rect(&mut self, r: Rect) { self.root_rect = r; }

    pub fn active_ws(&self) -> &Workspace { &self.workspaces[self.active] }
    pub fn active_ws_mut(&mut self) -> &mut Workspace { &mut self.workspaces[self.active] }

    pub fn new_workspace(&mut self, spawn: impl FnMut(u16, u16) -> Session) {
        let name = format!("{}", self.workspaces.len() + 1);
        let ws = Workspace::new(name, self.root_rect, self.metrics, self.gutter, spawn);
        self.workspaces.push(ws);
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
            let name = self.edit_buf.trim().to_string();
            if !name.is_empty() {
                self.workspaces[ws].name = name;
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
        self.workspaces.remove(ws);
        if self.active >= self.workspaces.len() {
            self.active = self.workspaces.len() - 1;
        }
        self.editing = None;
        self.edit_buf.clear();
        self.relayout_active();
    }

    fn relayout_active(&mut self) {
        let r = self.root_rect;
        self.active_ws_mut().relayout(r);
    }

    pub fn build_frame(
        &mut self,
        queue: &wgpu::Queue,
        atlas: &mut GpuAtlas,
        window: Rect,
    ) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
        const SIDEBAR_BG: [f32; 4] = [0.10, 0.10, 0.12, 1.0];
        const HILITE: [f32; 4] = [0.20, 0.22, 0.28, 1.0];
        const LABEL: [f32; 3] = [0.85, 0.85, 0.85];

        // 1. Active workspace's pane frame (borrows atlas mutably, returns owned Vecs).
        let theme = self.theme;
        let (mut bg, mut glyphs) = self.active_ws_mut().build_frame(queue, atlas, theme);

        let solid = |r: Rect, color: [f32; 4]| QuadInstance {
            pos: [r.x, r.y],
            size: [r.w, r.h],
            uv_min: [0.0, 0.0],
            uv_max: [0.0, 0.0],
            color,
        };

        // 2. Sidebar panel background (full height).
        bg.push(solid(Rect { x: 0.0, y: 0.0, w: SIDEBAR_W, h: window.h }, SIDEBAR_BG));

        // Collect label strings and indices before borrowing atlas.
        let ws_labels: Vec<String> = self.workspaces.iter().map(|w| w.name.clone()).collect();
        let active_ws = self.active;
        let metrics = self.metrics;

        // Resolve UV map for all label characters via the atlas (fresh borrow).
        let mut chars: Vec<char> = Vec::new();
        for s in &ws_labels {
            chars.extend(s.chars());
        }
        if self.editing.is_some() {
            chars.extend(self.edit_buf.chars());
            chars.push('_');
        }
        chars.push('+');
        let mut uv_map: std::collections::HashMap<char, GlyphInfo> =
            std::collections::HashMap::new();
        for c in chars {
            if c != ' ' && !uv_map.contains_key(&c) {
                uv_map.insert(c, atlas.uv_for(queue, c));
            }
        }
        let dg = GlyphInfo {
            uv_min: [0.0, 0.0],
            uv_max: [0.0, 0.0],
            px_size: [0.0, 0.0],
            offset: [0.0, 0.0],
        };
        let lookup = |c: char| uv_map.get(&c).copied().unwrap_or(dg);

        // 3. Sidebar workspace rows.
        let editing_idx = self.editing;
        let edit_buf_snapshot = self.edit_buf.clone();
        for (i, name) in ws_labels.iter().enumerate() {
            let r = sidebar_row_rect(i);
            if i == active_ws {
                bg.push(solid(r, HILITE));
            }
            let ty = r.y + (r.h - metrics.cell_h) * 0.5;
            let shown = if editing_idx == Some(i) {
                format!("{}_", edit_buf_snapshot)
            } else {
                name.clone()
            };
            glyphs.extend(build_text(&shown, &metrics, [r.x + PAD, ty], LABEL, &lookup));
        }
        // "+ new workspace" row.
        let plus_r = sidebar_row_rect(ws_labels.len());
        let pty = plus_r.y + (plus_r.h - metrics.cell_h) * 0.5;
        glyphs.extend(build_text("+", &metrics, [plus_r.x + PAD, pty], LABEL, &lookup));

        (bg, glyphs)
    }

    // Public convenience wrapper; callers use the free `pane_area_rect`
    // directly, so this method has no in-tree consumer.
    #[allow(dead_code)]
    pub fn pane_area(&self, window: Rect) -> Rect {
        pane_area_rect(window)
    }
}

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
        App::new(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, m, crate::config::Theme::default(), spawn_stub)
    }

    #[test]
    fn snapshot_uses_given_colors() {
        let s = spawn_stub(5, 10);
        let (rows, _) = snapshot_cells(&s, [0.1, 0.2, 0.3], [0.4, 0.5, 0.6]);
        assert!(!rows.is_empty());
        let cell = &rows[0][0];
        assert_eq!(cell.fg, [0.1, 0.2, 0.3]);
        assert_eq!(cell.bg, [0.4, 0.5, 0.6]);
    }

    #[test]
    fn starts_with_one_workspace() {
        let app = test_app();
        assert_eq!(app.workspaces.len(), 1);
        assert_eq!(app.active, 0);
        assert_eq!(app.active_ws().sessions.len(), 1);
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
    fn switch_workspace_bounds_checked() {
        let mut app = test_app();
        app.switch_workspace(99); // out of range -> no-op
        assert_eq!(app.active, 0);
    }

    #[test]
    fn rows_cols_floor_and_clamp() {
        let m = CellMetrics { cell_w: 10.0, cell_h: 20.0, ascent: 15.0 };
        let (rows, cols) = Workspace::rows_cols_for_rect(
            Rect { x: 0.0, y: 0.0, w: 105.0, h: 42.0 },
            &m,
        );
        assert_eq!(cols, 10); // floor(105/10)
        assert_eq!(rows, 2);  // floor(42/20)
        let (r2, c2) = Workspace::rows_cols_for_rect(
            Rect { x: 0.0, y: 0.0, w: 3.0, h: 3.0 },
            &m,
        );
        assert_eq!((r2, c2), (1, 1));
    }

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
}
