pub mod batch;
pub mod ring;

use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};
use ring::{RingBuffer, RING_CAPACITY};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

pub type SessionId = u64;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnOpts {
    pub cwd: String,
    pub program: String,
    pub args: Vec<String>,
    pub initial_command: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug)]
pub enum PtyError {
    NotFound(SessionId),
    Spawn(String),
    Io(String),
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtyError::NotFound(id) => write!(f, "session {id} not found"),
            PtyError::Spawn(m) => write!(f, "spawn failed: {m}"),
            PtyError::Io(m) => write!(f, "io error: {m}"),
        }
    }
}

pub type OutputSink = Arc<dyn Fn(&[u8]) + Send + Sync>;

type ExitHandler = Arc<dyn Fn(SessionId, Option<i32>) + Send + Sync>;

/// Recover from a poisoned mutex rather than propagating the panic.
/// Neither `RingBuffer` nor the PTY writer has an invariant a panic could
/// leave broken, so poison should not cascade.
fn lock_recover<'a, T>(
    result: Result<MutexGuard<'a, T>, PoisonError<MutexGuard<'a, T>>>,
) -> MutexGuard<'a, T> {
    result.unwrap_or_else(|e| e.into_inner())
}

struct Session {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Held so the exit-watcher thread (which Arc-clones it) keeps the child
    /// alive; never accessed directly via the Session after spawn.
    #[allow(dead_code)]
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    /// Separate killer handle so `kill()` never needs to acquire the child
    /// mutex — which the exit-watcher thread holds while blocking in `wait()`.
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    buffer: Arc<Mutex<RingBuffer>>,
    /// Used by the initial-command thread only.
    #[allow(dead_code)]
    saw_output: Arc<(Mutex<bool>, Condvar)>,
    alive: Arc<AtomicBool>,
    sink: Arc<Mutex<Option<OutputSink>>>,
    /// Latched true by the first `attach()`. Until then the reader thread answers
    /// the shell's cursor-position requests itself — see the DSR note on the
    /// reader thread.
    attached_ever: Arc<AtomicBool>,
}

pub struct SessionManager {
    next_id: AtomicU64,
    sessions: Mutex<HashMap<SessionId, Session>>,
    /// Wrapped in Arc so watcher threads can clone the Arc and read the
    /// handler at exit time rather than at spawn time (Fix 6).
    on_exit: Arc<Mutex<Option<ExitHandler>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SessionManager {
    /// Kill every remaining session on drop so PTY handles and shell processes
    /// are not left alive when the manager is torn down (e.g. panic, test
    /// teardown, Tauri state drop without an explicit shutdown_all).
    fn drop(&mut self) {
        let ids: Vec<SessionId> = lock_recover(self.sessions.lock()).keys().copied().collect();
        for id in ids {
            let _ = self.kill(id);
        }
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            sessions: Mutex::new(HashMap::new()),
            on_exit: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_exit_handler(&self, f: impl Fn(SessionId, Option<i32>) + Send + Sync + 'static) {
        *lock_recover(self.on_exit.lock()) = Some(Arc::new(f));
    }

    pub fn spawn(&self, opts: SpawnOpts) -> Result<SessionId, PtyError> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: opts.rows,
                cols: opts.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&opts.program);
        for a in &opts.args {
            cmd.arg(a);
        }
        cmd.cwd(&opts.cwd);

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;
        drop(pair.slave);

        // Clone the killer BEFORE wrapping child in Arc<Mutex> so we have a
        // separate handle that does not need the child mutex to call kill().
        let killer = child.clone_killer();

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Io(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Io(e.to_string()))?;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let buffer = Arc::new(Mutex::new(RingBuffer::with_capacity(RING_CAPACITY)));
        let writer = Arc::new(Mutex::new(writer));
        let killer = Mutex::new(killer);
        let child = Arc::new(Mutex::new(child));
        let master = Arc::new(Mutex::new(pair.master));
        let saw_output = Arc::new((Mutex::new(false), Condvar::new()));
        let alive = Arc::new(AtomicBool::new(true));
        let attached_ever = Arc::new(AtomicBool::new(false));

        let sink: Arc<Mutex<Option<OutputSink>>> = Arc::new(Mutex::new(None));
        // sync_channel caps the in-flight queue at 64 chunks (~4 MB ceiling).
        // An unbounded channel would defeat the 256 KB per-session ring buffer
        // guarantee when the sink stalls.
        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(64);

        // Reader thread: drains PTY output into the ring buffer.
        // Task 6 adds batched IPC emission here.
        //
        // ESC[6n (cursor position request) handling:
        //   PowerShell issues this query the instant it starts and then blocks
        //   completely until it is answered — no prompt, no echo, no reaction
        //   to typed input. Once a pane has attached, xterm.js answers these
        //   itself via its onData → IPC → write path, so we must not reply as
        //   well or the shell would receive two answers.
        //
        //   But before the first attach there is no xterm.js to answer, and
        //   attaching the first pane of a grid takes hundreds of ms (mounting
        //   every xterm instance and its WebGL context). The query lands in
        //   the ring buffer unanswered, the shell never starts, and replaying
        //   the buffer at attach time does not help: xterm generates a reply
        //   but the pane drops it because it is not yet marked attached.
        //
        //   So: answer only while `attached_ever` is false. That flag latches
        //   on the first attach() and never clears, which makes this a
        //   one-shot bootstrap responder rather than a permanent one. Replying
        //   ESC[1;1R is honest here — the shell has emitted nothing yet, so
        //   the cursor really is at the origin.
        {
            let buffer = Arc::clone(&buffer);
            let saw_output = Arc::clone(&saw_output);
            let alive = Arc::clone(&alive);
            let writer_for_cpr = Arc::clone(&writer);
            let attached_ever_for_cpr = Arc::clone(&attached_ever);
            // Move tx into the reader thread directly — the sole Sender.  Its
            // drop when this thread exits disconnects the channel, which is one
            // shutdown signal for the collector.  Do not clone it anywhere else;
            // moving rather than cloning makes an accidental extra Sender a
            // compile error.
            std::thread::spawn(move || {
                let mut chunk = [0u8; 65536];
                loop {
                    match reader.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let slice = &chunk[..n];
                            // try_lock, never lock: a contended writer must not
                            // stall the drain loop. Losing the race is safe —
                            // the shell repeats the query until answered.
                            if !attached_ever_for_cpr.load(Ordering::SeqCst)
                                && slice.windows(4).any(|w| w == b"\x1b[6n")
                            {
                                if let Ok(mut w) = writer_for_cpr.try_lock() {
                                    let _ = w.write_all(b"\x1b[1;1R");
                                    let _ = w.flush();
                                }
                            }
                            lock_recover(buffer.lock()).push(slice);
                            // try_send, not send: a blocking send here would
                            // stall the PTY drain loop, reintroducing the Task 5
                            // freeze class.  Dropping is safe — the ring buffer
                            // already has these bytes and the frontend can
                            // re-snapshot.
                            let _ = tx.try_send(chunk[..n].to_vec());
                            let (lock, cv) = &*saw_output;
                            let mut seen = lock_recover(lock.lock());
                            if !*seen {
                                *seen = true;
                                cv.notify_all();
                            }
                        }
                    }
                }
                alive.store(false, Ordering::SeqCst);
            });
        }

        // Collector thread: batches chunks from the reader and flushes to the
        // sink when the byte threshold or time interval is reached.
        // The thread exits when the reader thread drops its tx end (on EOF/error),
        // or — since EOF is unreliable on Windows ConPTY — when alive goes false.
        {
            let sink = Arc::clone(&sink);
            let alive = Arc::clone(&alive);
            std::thread::spawn(move || {
                let mut pending: Vec<u8> = Vec::with_capacity(batch::FLUSH_BYTES);
                let mut first_at: Option<Instant> = None;

                loop {
                    // When there are no pending bytes, use a longer idle timeout
                    // so we wake rarely (just often enough to notice the child has
                    // exited via the alive check below).  When bytes are pending,
                    // use the normal 8 ms flush window.
                    let timeout = match first_at {
                        None => Duration::from_millis(250),
                        Some(t) => {
                            let waited = t.elapsed().as_millis() as u64;
                            Duration::from_millis(
                                batch::FLUSH_INTERVAL_MS.saturating_sub(waited).max(1),
                            )
                        }
                    };

                    match rx.recv_timeout(timeout) {
                        Ok(bytes) => {
                            if pending.is_empty() {
                                first_at = Some(Instant::now());
                            }
                            pending.extend_from_slice(&bytes);
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            // A timeout means the channel was empty for the whole
                            // window, so no bytes are in flight.  If the child is
                            // gone, flush any remaining bytes and stop.  We cannot
                            // rely solely on the Disconnected arm because EOF on
                            // the reader is not guaranteed on Windows (grandchild
                            // holding the console keeps the pipe open — see the
                            // exit-watcher comment above).
                            if !alive.load(Ordering::SeqCst) {
                                if !pending.is_empty() {
                                    let maybe_sink = {
                                        let guard = lock_recover(sink.lock());
                                        guard.clone()
                                    };
                                    if let Some(s) = maybe_sink {
                                        s(&pending);
                                    }
                                }
                                break;
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }

                    let waited = first_at
                        .map(|t| t.elapsed().as_millis() as u64)
                        .unwrap_or(0);
                    if batch::should_flush(pending.len(), waited) {
                        // Take the guard, clone the Option, drop the guard, then
                        // invoke the sink outside the lock.  This prevents a
                        // self-deadlock if the sink calls detach(), and avoids
                        // holding the lock while executing user code.
                        let maybe_sink = {
                            let guard = lock_recover(sink.lock());
                            guard.clone()
                        };
                        if let Some(s) = maybe_sink {
                            s(&pending);
                        }
                        pending.clear();
                        first_at = None;
                    }
                }
            });
        }

        // Initial-command thread: wait for the first prompt, then write to
        // stdin. Launching via `-Command`/`-c` would close the shell on exit;
        // sending via stdin keeps the shell alive for subsequent interaction.
        if let Some(command) = opts.initial_command.clone() {
            let writer = Arc::clone(&writer);
            let saw_output = Arc::clone(&saw_output);
            std::thread::spawn(move || {
                let (lock, cv) = &*saw_output;
                let seen = lock_recover(lock.lock());
                let _ = cv.wait_timeout_while(seen, Duration::from_secs(1), |s| !*s);
                let mut w = lock_recover(writer.lock());
                let _ = w.write_all(format!("{command}\r").as_bytes());
                let _ = w.flush();
            });
        }

        // Exit-watcher thread.
        // Clone the Arc<Mutex<Option<ExitHandler>>> itself — not a snapshot of
        // its current contents — so the handler is read at exit time.  A
        // set_exit_handler call after spawn will therefore be visible to all
        // sessions, including ones spawned before the handler was registered.
        {
            let child = Arc::clone(&child);
            let on_exit = Arc::clone(&self.on_exit);
            let alive = Arc::clone(&alive);
            std::thread::spawn(move || {
                let status = lock_recover(child.lock()).wait().ok();
                let code = status.map(|s| s.exit_code() as i32);
                // Mark alive=false as soon as the process exits.  The reader
                // loop also sets this on EOF, but on Windows a grandchild
                // holding the console can keep the output pipe open
                // indefinitely, so we must not rely solely on EOF.
                alive.store(false, Ordering::SeqCst);
                if let Some(h) = lock_recover(on_exit.lock()).clone() {
                    h(id, code);
                }
            });
        }

        lock_recover(self.sessions.lock()).insert(
            id,
            Session {
                master,
                writer,
                child,
                killer,
                buffer,
                saw_output,
                alive,
                sink,
                attached_ever,
            },
        );

        Ok(id)
    }

    pub fn write(&self, id: SessionId, data: &[u8]) -> Result<(), PtyError> {
        // Clone the Arc under the map lock, then drop the map lock before
        // doing I/O. Holding the map lock across write_all/flush allows a
        // single wedged pane to block every other write/resize/kill/snapshot
        // in the process. (Fix 1)
        let writer = {
            let sessions = lock_recover(self.sessions.lock());
            let s = sessions.get(&id).ok_or(PtyError::NotFound(id))?;
            Arc::clone(&s.writer)
        };
        let mut w = lock_recover(writer.lock());
        w.write_all(data).map_err(|e| PtyError::Io(e.to_string()))?;
        w.flush().map_err(|e| PtyError::Io(e.to_string()))
    }

    pub fn resize(&self, id: SessionId, cols: u16, rows: u16) -> Result<(), PtyError> {
        // Clone the master Arc before dropping the map lock. (Fix 1)
        let master = {
            let sessions = lock_recover(self.sessions.lock());
            let s = sessions.get(&id).ok_or(PtyError::NotFound(id))?;
            Arc::clone(&s.master)
        };
        let result = lock_recover(master.lock())
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Io(e.to_string()));
        result
    }

    pub fn kill(&self, id: SessionId) -> Result<(), PtyError> {
        let mut sessions = lock_recover(self.sessions.lock());
        let s = sessions.remove(&id).ok_or(PtyError::NotFound(id))?;
        // Use the pre-cloned killer so we never need to lock `child` here.
        // The exit-watcher thread holds child.lock() while blocked in wait(),
        // so locking child here would deadlock.
        let _ = lock_recover(s.killer.lock()).kill();
        Ok(())
    }

    pub fn snapshot(&self, id: SessionId) -> Result<Vec<u8>, PtyError> {
        // Clone the buffer Arc before dropping the map lock — copying 256 KB
        // under the global lock is the same bug as blocking on I/O. (Fix 1)
        let buffer = {
            let sessions = lock_recover(self.sessions.lock());
            let s = sessions.get(&id).ok_or(PtyError::NotFound(id))?;
            Arc::clone(&s.buffer)
        };
        let data = lock_recover(buffer.lock()).snapshot();
        Ok(data)
    }

    /// Start streaming batched output to `sink`.
    ///
    /// Bytes buffered before this call are *not* replayed; call `snapshot` first
    /// to backfill, or everything between `spawn` and `attach` is lost from the
    /// stream (it remains in the ring buffer).
    pub fn attach(&self, id: SessionId, sink: OutputSink) -> Result<(), PtyError> {
        // Clone the per-session Arc under the map lock, then release the map
        // guard before swapping the sink.  This follows the same pattern as
        // write/resize/snapshot (Fix 1 from Task 5): the old OutputSink's
        // destructor — and anything it captured — must not run while the global
        // sessions lock is held.
        let slot = {
            let sessions = lock_recover(self.sessions.lock());
            let s = sessions.get(&id).ok_or(PtyError::NotFound(id))?;
            // Latch before the sink is installed: from here on xterm.js owns
            // the DSR reply, so the bootstrap responder must stand down. A
            // later detach() does not clear it — xterm.js has already answered
            // the startup query, and a second reply would confuse the shell.
            s.attached_ever.store(true, Ordering::SeqCst);
            Arc::clone(&s.sink)
        };
        let previous = lock_recover(slot.lock()).replace(sink);
        drop(previous); // user destructor runs outside both locks
        Ok(())
    }

    /// Stop streaming to the sink.
    ///
    /// Not a synchronization barrier: a flush already in flight holds a cloned
    /// `OutputSink` Arc, so the sink may be invoked once after this returns,
    /// carrying only pre-detach bytes.  Bytes produced after this returns are
    /// never streamed.
    pub fn detach(&self, id: SessionId) -> Result<(), PtyError> {
        // Same lock discipline as attach: clone the Arc, release the map guard,
        // then swap so the old sink's destructor runs outside both locks.
        let slot = {
            let sessions = lock_recover(self.sessions.lock());
            Arc::clone(&sessions.get(&id).ok_or(PtyError::NotFound(id))?.sink)
        };
        let previous = lock_recover(slot.lock()).take();
        drop(previous); // user destructor runs outside both locks
        Ok(())
    }

    /// Kill all sessions and reap them. Called at application exit.
    ///
    /// The previous implementation wrote `exit\r` and waited up to 2 s for
    /// each session to terminate.  That handshake is never effective for AI
    /// CLI TUIs (Claude Code, Gemini CLI) — they do not interpret `exit\r` as
    /// a shell command.  As a result the full 2-second deadline was hit on
    /// every single close for the app's primary use-case.
    ///
    /// The ring buffer is deliberately not persisted across restarts (spec §4),
    /// so there is nothing to flush.  Killing child processes on exit is
    /// exactly what a terminal emulator is expected to do.
    pub fn shutdown_all(&self) {
        let ids: Vec<SessionId> = lock_recover(self.sessions.lock()).keys().copied().collect();
        for id in ids {
            let _ = self.kill(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn interactive_shell() -> (String, Vec<String>) {
        if cfg!(windows) {
            (r"C:\Windows\System32\cmd.exe".to_string(), vec![])
        } else {
            ("/bin/sh".to_string(), vec!["-i".to_string()])
        }
    }

    fn temp_cwd() -> String {
        std::env::temp_dir().to_string_lossy().to_string()
    }

    /// A sink that stands in for an attached xterm.js: it records what it
    /// receives *and* answers cursor-position requests. The reply matters —
    /// `attach()` retires the bootstrap responder precisely because the
    /// frontend takes over that duty, so a sink that only records would leave
    /// the shell blocked and the test measuring nothing.
    ///
    /// Holds a Weak, not an Arc: the manager owns the sink, so an Arc here
    /// would be a cycle and the manager would never drop.
    fn terminal_sink(mgr: &Arc<SessionManager>, id: u64, got: Arc<Mutex<Vec<u8>>>) -> OutputSink {
        let weak = Arc::downgrade(mgr);
        Arc::new(move |bytes: &[u8]| {
            got.lock().unwrap().extend_from_slice(bytes);
            if bytes.windows(4).any(|w| w == b"\x1b[6n") {
                if let Some(mgr) = weak.upgrade() {
                    let _ = mgr.write(id, b"\x1b[1;1R");
                }
            }
        })
    }

    fn wait_for(mgr: &SessionManager, id: u64, needle: &str, timeout: Duration) -> String {
        let start = Instant::now();
        loop {
            let text = String::from_utf8_lossy(&mgr.snapshot(id).unwrap()).to_string();
            if text.contains(needle) {
                return text;
            }
            if start.elapsed() > timeout {
                panic!("timed out waiting for {needle:?}; buffer was:\n{text}");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    #[test]
    fn spawns_a_shell_and_buffers_its_output() {
        let mgr = SessionManager::new();
        let (program, args) = interactive_shell();
        let id = mgr
            .spawn(SpawnOpts {
                cwd: temp_cwd(),
                program,
                args,
                initial_command: None,
                cols: 80,
                rows: 24,
            })
            .expect("spawn failed");

        mgr.write(id, b"echo hello_from_write\r").unwrap();
        wait_for(&mgr, id, "hello_from_write", Duration::from_secs(10));
        mgr.kill(id).unwrap();
    }

    #[test]
    fn runs_the_initial_command_through_stdin_and_keeps_the_shell_alive() {
        let mgr = SessionManager::new();
        let (program, args) = interactive_shell();
        let id = mgr
            .spawn(SpawnOpts {
                cwd: temp_cwd(),
                program,
                args,
                initial_command: Some("echo marker_42".to_string()),
                cols: 80,
                rows: 24,
            })
            .expect("spawn failed");

        wait_for(&mgr, id, "marker_42", Duration::from_secs(10));

        // Asıl mesele: ilk komut bittikten sonra shell hâlâ komut alabiliyor olmalı.
        mgr.write(id, b"echo still_alive\r").unwrap();
        wait_for(&mgr, id, "still_alive", Duration::from_secs(10));
        mgr.kill(id).unwrap();
    }

    /// Regression: a shell that opens with a cursor-position request (ESC[6n)
    /// blocks until something answers it. In production the answer can only
    /// come from an attached xterm.js, but the first pane of a grid attaches
    /// several hundred ms after spawn — so the shell was still blocked when
    /// its initial command was typed, and the pane stayed blank forever.
    ///
    /// Deliberately never attaches: that is the state the bug lives in. Uses
    /// PowerShell rather than the `interactive_shell()` helper because cmd.exe
    /// does not issue the query, which is why the other tests never caught it.
    #[test]
    #[cfg(windows)]
    fn answers_cursor_position_requests_before_any_pane_attaches() {
        let mgr = SessionManager::new();
        let id = mgr
            .spawn(SpawnOpts {
                cwd: temp_cwd(),
                program: "powershell.exe".to_string(),
                args: vec!["-NoLogo".to_string()],
                initial_command: Some("echo marker_dsr".to_string()),
                cols: 80,
                rows: 24,
            })
            .expect("spawn failed");

        // The shell echoes what was typed, so one occurrence proves nothing —
        // the command actually RAN only if the marker appears twice.
        let start = Instant::now();
        loop {
            let text = String::from_utf8_lossy(&mgr.snapshot(id).unwrap()).to_string();
            if text.matches("marker_dsr").count() > 1 {
                break;
            }
            assert!(
                start.elapsed() <= Duration::from_secs(15),
                "shell never ran its initial command — still blocked on ESC[6n; buffer was:\n{text}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        mgr.kill(id).unwrap();
    }

    #[test]
    fn reports_exit_with_a_status_code() {
        let mgr = SessionManager::new();
        let seen: Arc<Mutex<Vec<(u64, Option<i32>)>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        mgr.set_exit_handler(move |id, code| sink.lock().unwrap().push((id, code)));

        let (program, args) = interactive_shell();
        let id = mgr
            .spawn(SpawnOpts {
                cwd: temp_cwd(),
                program,
                args,
                initial_command: Some("exit 3".to_string()),
                cols: 80,
                rows: 24,
            })
            .unwrap();

        let start = Instant::now();
        loop {
            if seen.lock().unwrap().iter().any(|(sid, _)| *sid == id) {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "no exit event arrived"
            );
            std::thread::sleep(Duration::from_millis(25));
        }

        let events = seen.lock().unwrap().clone();
        let (_, code) = events.iter().find(|(sid, _)| *sid == id).unwrap();
        assert_eq!(*code, Some(3));
    }

    #[test]
    fn unknown_session_ids_are_errors_not_panics() {
        let mgr = SessionManager::new();
        assert!(mgr.write(9999, b"x").is_err());
        assert!(mgr.resize(9999, 10, 10).is_err());
        assert!(mgr.kill(9999).is_err());
        assert!(mgr.snapshot(9999).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resize_reaches_the_child_process() {
        let mgr = SessionManager::new();
        let id = mgr
            .spawn(SpawnOpts {
                cwd: temp_cwd(),
                program: "/bin/sh".into(),
                args: vec!["-i".into()],
                initial_command: None,
                cols: 80,
                rows: 24,
            })
            .unwrap();

        mgr.resize(id, 132, 43).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        mgr.write(id, b"stty size\r").unwrap();
        wait_for(&mgr, id, "43 132", Duration::from_secs(10));
        mgr.kill(id).unwrap();
    }

    #[test]
    fn attached_sessions_stream_bytes_to_the_sink() {
        let mgr = Arc::new(SessionManager::new());
        let (program, args) = interactive_shell();
        let id = mgr
            .spawn(SpawnOpts {
                cwd: temp_cwd(),
                program,
                args,
                initial_command: None,
                cols: 80,
                rows: 24,
            })
            .unwrap();

        let got: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        mgr.attach(id, terminal_sink(&mgr, id, Arc::clone(&got)))
            .unwrap();

        mgr.write(id, b"echo streamed_ok\r").unwrap();

        let start = Instant::now();
        loop {
            let text = String::from_utf8_lossy(&got.lock().unwrap()).to_string();
            if text.contains("streamed_ok") {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "sink never saw the output"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        mgr.kill(id).unwrap();
    }

    #[test]
    fn detached_sessions_keep_buffering_but_stop_streaming() {
        let mgr = Arc::new(SessionManager::new());
        let (program, args) = interactive_shell();
        let id = mgr
            .spawn(SpawnOpts {
                cwd: temp_cwd(),
                program,
                args,
                initial_command: None,
                cols: 80,
                rows: 24,
            })
            .unwrap();

        let got: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        mgr.attach(id, terminal_sink(&mgr, id, Arc::clone(&got)))
            .unwrap();
        mgr.write(id, b"echo before_detach\r").unwrap();
        wait_for(&mgr, id, "before_detach", Duration::from_secs(10));

        // Prove the sink is actually streaming before we assert it stops;
        // otherwise a no-op attach would make the assertion below pass vacuously.
        // The sink lags the ring buffer by up to one 8 ms flush window, so poll
        // rather than asserting immediately.
        let start = Instant::now();
        loop {
            if String::from_utf8_lossy(&got.lock().unwrap()).contains("before_detach") {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "sink never streamed while attached"
            );
            std::thread::sleep(Duration::from_millis(25));
        }

        mgr.detach(id).unwrap();
        got.lock().unwrap().clear();

        mgr.write(id, b"echo after_detach\r").unwrap();
        wait_for(&mgr, id, "after_detach", Duration::from_secs(10));

        // Ring buffer doldu ama sink hiçbir şey görmedi.
        let streamed = String::from_utf8_lossy(&got.lock().unwrap()).to_string();
        assert!(!streamed.contains("after_detach"), "sink got: {streamed}");
        mgr.kill(id).unwrap();
    }
}
