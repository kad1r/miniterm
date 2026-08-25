use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};

pub struct Pty {
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn Read + Send>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Pty {
    pub fn spawn(rows: u16, cols: u16, shell: &str) -> Pty {
        let sys = NativePtySystem::default();
        let pair = sys
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .expect("openpty");
        let cmd = CommandBuilder::new(shell);
        let child = pair.slave.spawn_command(cmd).expect("spawn shell");
        let writer = pair.master.take_writer().expect("writer");
        let reader = pair.master.try_clone_reader().expect("reader");
        Pty { writer, reader, master: pair.master, _child: child }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let _ = self.master.resize(PtySize {
            rows, cols, pixel_width: 0, pixel_height: 0,
        });
    }
}
