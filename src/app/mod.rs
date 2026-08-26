mod workspace;
#[allow(unused_imports)]
pub use workspace::{App, Workspace};

use crate::layout::tree::Rect;

pub const SIDEBAR_W: f32 = 180.0;
pub const ROW_H: f32 = 24.0;
pub const PAD: f32 = 6.0;

pub fn pane_area_rect(window: Rect) -> Rect {
    Rect {
        x: SIDEBAR_W,
        y: 0.0,
        w: (window.w - SIDEBAR_W).max(0.0),
        h: window.h.max(0.0),
    }
}

pub fn sidebar_row_rect(i: usize) -> Rect {
    Rect { x: 0.0, y: PAD + i as f32 * ROW_H, w: SIDEBAR_W, h: ROW_H }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeHit {
    Workspace(usize),
    NewWorkspace,
    PaneArea,
}

pub fn chrome_hit(_window: Rect, ws_count: usize, p: (f32, f32)) -> ChromeHit {
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
    ChromeHit::PaneArea
}

#[cfg(test)]
mod chrome_tests {
    use super::*;
    use crate::layout::tree::Rect;

    fn win() -> Rect { Rect { x: 0.0, y: 0.0, w: 1000.0, h: 700.0 } }

    #[test]
    fn click_first_sidebar_row_is_workspace_0() {
        // row 0 spans y in [PAD, PAD+ROW_H).
        let hit = chrome_hit(win(), 2, (20.0, PAD + 2.0));
        assert!(matches!(hit, ChromeHit::Workspace(0)));
    }

    #[test]
    fn click_new_workspace_row() {
        let hit = chrome_hit(win(), 2, (20.0, PAD + 2.0 * ROW_H + 2.0));
        assert!(matches!(hit, ChromeHit::NewWorkspace));
    }

    #[test]
    fn click_pane_area() {
        let hit = chrome_hit(win(), 1, (400.0, 400.0));
        assert!(matches!(hit, ChromeHit::PaneArea));
    }
}
