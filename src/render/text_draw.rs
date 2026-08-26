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
