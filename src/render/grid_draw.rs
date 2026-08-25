use crate::text::metrics::CellMetrics;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub color: [f32; 4],
}

pub struct CellView {
    pub ch: char,
    pub fg: [f32; 3],
    pub bg: [f32; 3],
}

/// Glyph atlas lookup result: UV rect plus the actual glyph bitmap size and
/// its baseline-relative offset (swash placement left/top).
#[derive(Clone, Copy)]
pub struct GlyphInfo {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// Actual glyph bitmap size in px (placement width/height).
    pub px_size: [f32; 2],
    /// Placement offset: left = x from pen origin, top = y above baseline.
    pub offset: [f32; 2],
}

pub fn build_instances(
    cells: &[Vec<CellView>],
    m: &CellMetrics,
    origin: [f32; 2],
    atlas_uv: &dyn Fn(char) -> GlyphInfo,
) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
    let mut bg = Vec::new();
    let mut glyphs = Vec::new();
    for (row_idx, row) in cells.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            let x = origin[0] + col_idx as f32 * m.cell_w;
            let y = origin[1] + row_idx as f32 * m.cell_h;
            bg.push(QuadInstance {
                pos: [x, y],
                size: [m.cell_w, m.cell_h],
                uv_min: [0.0, 0.0],
                uv_max: [0.0, 0.0],
                color: [cell.bg[0], cell.bg[1], cell.bg[2], 1.0],
            });
            if cell.ch != ' ' && cell.ch != '\0' {
                let g = atlas_uv(cell.ch);
                // Size the quad to the actual glyph bitmap and place it at the
                // baseline: pen x + left, (top of cell + ascent) - top.
                let gx = x + g.offset[0];
                let gy = y + m.ascent - g.offset[1];
                glyphs.push(QuadInstance {
                    pos: [gx, gy],
                    size: g.px_size,
                    uv_min: g.uv_min,
                    uv_max: g.uv_max,
                    color: [cell.fg[0], cell.fg[1], cell.fg[2], 1.0],
                });
            }
        }
    }
    (bg, glyphs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_grid_cells_by_metrics_and_skips_spaces_for_glyphs() {
        let m = CellMetrics { cell_w: 10.0, cell_h: 20.0, ascent: 15.0 };
        let cells = vec![vec![
            CellView { ch: 'A', fg: [1.0, 1.0, 1.0], bg: [0.0, 0.0, 0.0] },
            CellView { ch: ' ', fg: [1.0, 1.0, 1.0], bg: [0.0, 0.0, 0.0] },
        ]];
        let uv = |_c: char| GlyphInfo {
            uv_min: [0.0, 0.0],
            uv_max: [0.5, 0.5],
            px_size: [8.0, 12.0],
            offset: [1.0, 10.0],
        };
        let (bg, glyphs) = build_instances(&cells, &m, [100.0, 50.0], &uv);
        // One bg quad per cell.
        assert_eq!(bg.len(), 2);
        assert_eq!(bg[1].pos, [110.0, 50.0]);
        assert_eq!(bg[0].size, [10.0, 20.0]);
        // Spaces produce no glyph quad.
        assert_eq!(glyphs.len(), 1);
        // Glyph sized to bitmap, placed at baseline: x+left, y+ascent-top.
        assert_eq!(glyphs[0].pos, [101.0, 55.0]);
        assert_eq!(glyphs[0].size, [8.0, 12.0]);
        assert_eq!(glyphs[0].uv_max, [0.5, 0.5]);
    }
}
