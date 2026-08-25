use swash::FontRef;

pub struct CellMetrics {
    pub cell_w: f32,
    pub cell_h: f32,
    pub ascent: f32,
}

pub fn measure(font_bytes: &[u8], px: f32) -> CellMetrics {
    let font = FontRef::from_index(font_bytes, 0).expect("valid font");
    let metrics = font.metrics(&[]).scale(px);
    let glyph = font.charmap().map('M');
    let advance = font.glyph_metrics(&[]).scale(px).advance_width(glyph);
    let cell_h = (metrics.ascent + metrics.descent + metrics.leading).ceil();
    CellMetrics {
        cell_w: advance.ceil(),
        cell_h,
        ascent: metrics.ascent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../../assets/font/CascadiaMono.ttf");

    #[test]
    fn metrics_are_positive_and_monospace_sized() {
        let m = measure(FONT, 16.0);
assert!(m.cell_w > 0.0 && m.cell_h > 0.0);
        assert!(m.ascent > 0.0 && m.ascent < m.cell_h);
        // Monospace advance is narrower than line height for typical fonts.
        assert!(m.cell_w < m.cell_h);
    }
}
