use crate::render::grid_draw::CellView;
use crate::terminal::session::Session;
use alacritty_terminal::index::{Column, Line, Point};

/// Snapshot the alacritty Term grid into CellView rows.
/// Monochrome palette for M1; theme colours arrive in the config milestone.
pub fn grid_to_cells(session: &Session, rows: usize, cols: usize) -> Vec<Vec<CellView>> {
    let term = session.term.lock().unwrap();
    let grid = term.grid();
    let mut out = Vec::with_capacity(rows);
    for line in 0..rows {
        let mut row = Vec::with_capacity(cols);
        for col in 0..cols {
            let cell = &grid[Point::new(Line(line as i32), Column(col))];
            row.push(CellView {
                ch: cell.c,
                fg: [0.85, 0.85, 0.85],
                bg: [0.05, 0.05, 0.06],
            });
        }
        out.push(row);
    }
    out
}
