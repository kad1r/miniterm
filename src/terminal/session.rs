use crate::terminal::pty::Pty;
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::term::{test::TermSize, Config, Term};
use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};
use std::io::Read;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct EventProxy;
impl EventListener for EventProxy {
    fn send_event(&self, _event: Event) {}
}

pub type SharedTerm = Arc<Mutex<Term<EventProxy>>>;

pub struct Session {
    pub term: SharedTerm,
    writer: Box<dyn std::io::Write + Send>,
    pty: Pty,
    size: (u16, u16),
}

impl Session {
    pub fn spawn(
        rows: u16,
        cols: u16,
        shell: &str,
        on_output: impl Fn() + Send + 'static,
    ) -> Session {
        let mut pty = Pty::spawn(rows, cols, shell);
        let writer = std::mem::replace(
            &mut pty.writer,
            Box::new(std::io::sink()),
        );
        let term_size = TermSize::new(cols as usize, rows as usize);
        let term = Arc::new(Mutex::new(Term::new(
            Config::default(),
            &term_size,
            EventProxy,
        )));

        let reader_term = term.clone();
        let mut reader = std::mem::replace(
            &mut pty.reader,
            Box::new(std::io::empty()),
        );
        std::thread::spawn(move || {
            let mut parser: Processor<StdSyncHandler> = Processor::new();
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        {
                            let mut term = reader_term.lock().unwrap();
                            for &byte in &buf[..n] {
                                parser.advance(&mut *term, byte);
                            }
                        }
                        on_output();
                    }
                }
            }
        });

        Session { term, writer, pty, size: (rows, cols) }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        if (rows, cols) == self.size {
            return;
        }
        self.size = (rows, cols);
        self.pty.resize(rows, cols);
        let mut term = self.term.lock().unwrap();
        term.resize(TermSize::new(cols as usize, rows as usize));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::index::{Column, Line, Point};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    #[test]
    fn echo_reaches_the_grid() {
        static WOKE: AtomicBool = AtomicBool::new(false);
        let mut s = Session::spawn(24, 80, "cmd.exe", || {
            WOKE.store(true, Ordering::SeqCst);
        });
        // Ask cmd to print a marker then exit.
        s.write(b"echo MINITERM_OK\r\n");

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut found = false;
        while Instant::now() < deadline && !found {
            std::thread::sleep(Duration::from_millis(50));
            let term = s.term.lock().unwrap();
            let grid = term.grid();
            for line in 0..24 {
                let mut row = String::new();
                for col in 0..80 {
                    let p = Point::new(Line(line), Column(col));
                    row.push(grid[p].c);
                }
                if row.contains("MINITERM_OK") {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "expected echoed marker in grid");
        assert!(WOKE.load(Ordering::SeqCst), "on_output should fire");
    }
}
