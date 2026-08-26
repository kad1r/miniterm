mod tab;
pub use tab::Tab;

use crate::layout::tree::Rect;
use crate::terminal::session::Session;
use crate::text::metrics::CellMetrics;

pub struct App {
    tab: Tab,
}

impl App {
    pub fn new(
        root_rect: Rect,
        metrics: CellMetrics,
        spawn: impl FnMut(u16, u16) -> Session,
    ) -> App {
        let gutter = 4.0;
        let tab = Tab::new(root_rect, metrics, gutter, spawn);
        App { tab }
    }

    pub fn active_tab(&self) -> &Tab {
        &self.tab
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tab
    }
}
