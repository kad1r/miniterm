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
