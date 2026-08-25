use crate::layout::hit::SplitHit;
use crate::layout::tree::{split_rect, Dir, LayoutTree, Node, PaneId, Rect, Side};
use crate::render::atlas_gpu::GpuAtlas;
use crate::render::grid_draw::{build_instances, CellView, GlyphInfo, QuadInstance};
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
        let mut colors = std::collections::HashMap::new();
        colors.insert(first, pane_color(0));
        App {
            sessions,
            tree,
            focus: first,
            metrics,
            rects,
            gutter,
            colors,
            color_seed: 1,
        }
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
                    color: [0.85, 0.85, 0.85, 1.0],
                });
            }

            all_bg.extend(bg);
            all_glyphs.extend(glyphs);
        }

        // Per-pane boundary borders: a thin colored line on the left edge of
        // panes not flush to the window's left, and on the top edge of panes
        // not flush to the window's top (stacked splits). Leftmost/topmost
        // panes get no border on that side.
        let t = 2.0f32;
        for (id, rect) in &rects {
            let color = self.colors.get(id).copied().unwrap_or([0.5, 0.5, 0.5, 1.0]);
            if rect.x > 0.5 {
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

    pub fn close_focused(&mut self, root_rect: Rect) {
        if self.sessions.len() <= 1 {
            return; // never close the last pane
        }
        let closing = self.focus;
        if self.tree.close(closing) {
            self.sessions.remove(closing); // drops Session => PTY + reader thread end
            self.colors.remove(&closing);
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
