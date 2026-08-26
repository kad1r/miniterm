#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub ch: char,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct ShelfPacker {
    width: u32,
    cursor_x: u32,
    cursor_y: u32,
    shelf_h: u32,
}

impl ShelfPacker {
    pub fn new(width: u32) -> ShelfPacker {
        ShelfPacker { width, cursor_x: 0, cursor_y: 0, shelf_h: 0 }
    }

    /// True if inserting a `w`x`h` glyph would spill past `atlas_size` in height.
    pub fn would_overflow(&self, w: u32, h: u32, atlas_size: u32) -> bool {
        // Mirror the wrap logic in `insert`: a too-wide glyph starts a new shelf.
        let shelf_top = if self.cursor_x + w > self.width {
            self.cursor_y + self.shelf_h
        } else {
            self.cursor_y
        };
        shelf_top + h > atlas_size
    }

    pub fn insert(&mut self, w: u32, h: u32) -> GlyphRect {
        if self.cursor_x + w > self.width {
            self.cursor_y += self.shelf_h;
            self.cursor_x = 0;
            self.shelf_h = 0;
        }
        let rect = GlyphRect { x: self.cursor_x, y: self.cursor_y, w, h };
        self.cursor_x += w;
        self.shelf_h = self.shelf_h.max(h);
        rect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_left_to_right_then_wraps_to_new_shelf() {
        let mut p = ShelfPacker::new(20);
        let a = p.insert(8, 10);
        let b = p.insert(8, 10);
        // Third glyph would overflow width 20 -> wraps to a new shelf.
        let c = p.insert(8, 10);
        assert_eq!(a, GlyphRect { x: 0, y: 0, w: 8, h: 10 });
        assert_eq!(b, GlyphRect { x: 8, y: 0, w: 8, h: 10 });
        assert_eq!(c, GlyphRect { x: 0, y: 10, w: 8, h: 10 });
    }

    #[test]
    fn would_overflow_reports_vertical_spill() {
        let mut p = ShelfPacker::new(20);
        // Fill the first shelf (height 10) so the next glyph wraps to y=10.
        p.insert(20, 10);
        // A 10-tall glyph on the new shelf ends at y=20: exactly fits an atlas
        // of height 20, but overflows one of height 19.
        assert!(!p.would_overflow(8, 10, 20));
        assert!(p.would_overflow(8, 10, 19));
    }
}
