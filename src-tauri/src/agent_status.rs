//! Agent status from Claude Code hook events.
//!
//! The hook (see `claude_hooks.rs`) appends the JSON it gets on stdin as one
//! line to `<status_dir>/<session_id>.jsonl`. A single watcher tails those
//! files and turns each session's events into working / waiting / done.
//!
//! Done in Rust rather than in xterm.js because hidden workspaces have no
//! xterm instance — and a waiting agent in a hidden workspace is the case this
//! exists for.

use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

/// Output within this window counts as `active` for sessions without hooks.
pub const ACTIVE_WINDOW: Duration = Duration::from_secs(2);
pub const TICK: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Working,
    Waiting,
    Done,
    /// Hint for sessions that never sent a hook event: output in the last
    /// `ACTIVE_WINDOW`. Cannot tell waiting from done, so it never notifies.
    Active,
    Idle,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub id: u64,
    pub state: State,
    pub subagents: u32,
}

#[derive(Debug, Default, Clone)]
pub struct Tracker {
    pub state: Option<State>,
    pub subagents: u32,
    /// A `Stop` arrived while subagents were still running.
    stop_pending: bool,
}

impl Tracker {
    /// Applies one hook event line. Returns true when state or count changed.
    pub fn apply(&mut self, line: &str) -> bool {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return false;
        };
        let event = v.get("hook_event_name").and_then(Value::as_str).unwrap_or("");
        let tool = v.get("tool_name").and_then(Value::as_str).unwrap_or("");
        let before = (self.state, self.subagents);
        match event {
            "UserPromptSubmit" => {
                // A new turn. Also resets the counter: a lost SubagentStop must
                // not leave the row stuck on "working" forever.
                self.subagents = 0;
                self.stop_pending = false;
                self.state = Some(State::Working);
            }
            "PreToolUse" => {
                if tool == "Task" || tool == "Agent" {
                    self.subagents += 1;
                }
                self.state = Some(State::Working);
            }
            // The permission prompt sits between PreToolUse and PostToolUse,
            // so this is what flips "waiting" back once the user answers.
            "PostToolUse" => {
                if self.state == Some(State::Waiting) {
                    self.state = Some(State::Working);
                }
            }
            "Notification" => {
                // After a finished turn this is Claude's idle reminder, not a
                // question; the row already says done.
                if self.state != Some(State::Done) {
                    self.state = Some(State::Waiting);
                }
            }
            "SubagentStop" => {
                self.subagents = self.subagents.saturating_sub(1);
                if self.stop_pending && self.subagents == 0 {
                    self.stop_pending = false;
                    self.state = Some(State::Done);
                }
            }
            "Stop" => {
                if self.subagents > 0 {
                    self.stop_pending = true;
                    self.state = Some(State::Working);
                } else {
                    self.state = Some(State::Done);
                }
            }
            _ => {}
        }
        (self.state, self.subagents) != before
    }
}

#[derive(Default)]
struct FileState {
    offset: u64,
    /// Bytes after the last newline — a line the hook is still writing.
    partial: Vec<u8>,
    tracker: Tracker,
}

#[derive(Default)]
pub struct Watcher {
    files: HashMap<u64, FileState>,
    /// Last active/idle hint sent per hook-less session.
    hints: HashMap<u64, State>,
}

impl Watcher {
    /// One pass: tail every status file, compute hints for sessions without
    /// one, forget and delete whatever belongs to a session that is gone.
    pub fn tick(&mut self, dir: &Path, ages: &[(u64, Duration)]) -> Vec<Update> {
        let live: HashSet<u64> = ages.iter().map(|(id, _)| *id).collect();
        let mut out = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(id) = session_of(&path) else { continue };
                if !live.contains(&id) {
                    let _ = fs::remove_file(&path);
                    self.files.remove(&id);
                    continue;
                }
                let Ok(len) = entry.metadata().map(|m| m.len()) else { continue };
                let file = self.files.entry(id).or_default();
                if len <= file.offset {
                    continue;
                }
                if let Some(update) = read_new(&path, id, file) {
                    out.push(update);
                }
            }
        }
        self.files.retain(|id, _| live.contains(id));
        self.hints.retain(|id, _| live.contains(id));

        for (id, age) in ages {
            if self.files.get(id).is_some_and(|f| f.tracker.state.is_some()) {
                continue;
            }
            let hint = if *age < ACTIVE_WINDOW { State::Active } else { State::Idle };
            if self.hints.insert(*id, hint) != Some(hint) {
                out.push(Update { id: *id, state: hint, subagents: 0 });
            }
        }
        out
    }
}

fn session_of(path: &Path) -> Option<u64> {
    if path.extension()? != "jsonl" {
        return None;
    }
    path.file_stem()?.to_str()?.parse().ok()
}

fn read_new(path: &Path, id: u64, file: &mut FileState) -> Option<Update> {
    let mut f = fs::File::open(path).ok()?;
    f.seek(SeekFrom::Start(file.offset)).ok()?;
    let mut buf = Vec::new();
    let n = f.read_to_end(&mut buf).ok()?;
    file.offset += n as u64;
    file.partial.extend_from_slice(&buf);

    let mut changed = false;
    while let Some(nl) = file.partial.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = file.partial.drain(..=nl).collect();
        changed |= file.tracker.apply(String::from_utf8_lossy(&line).trim());
    }
    let t = &file.tracker;
    match (changed, t.state) {
        (true, Some(state)) => Some(Update { id, state, subagents: t.subagents }),
        _ => None,
    }
}

/// Fresh, empty status folder. Session ids restart at 1 every run, so files
/// left by a previous run would be read as the new sessions' events.
pub fn reset_dir(dir: &Path) -> std::io::Result<()> {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn ev(name: &str) -> String {
        format!(r#"{{"hook_event_name":"{name}","session_id":"s"}}"#)
    }

    fn tool(name: &str, tool: &str) -> String {
        format!(r#"{{"hook_event_name":"{name}","tool_name":"{tool}"}}"#)
    }

    fn replay(lines: &[String]) -> Tracker {
        let mut t = Tracker::default();
        for l in lines {
            t.apply(l);
        }
        t
    }

    #[test]
    fn a_plain_turn_goes_working_then_done() {
        let t = replay(&[ev("UserPromptSubmit"), tool("PreToolUse", "Bash"), ev("Stop")]);
        assert_eq!(t.state, Some(State::Done));
    }

    #[test]
    fn permission_prompt_waits_and_resumes_after_the_answer() {
        let mut t = replay(&[ev("UserPromptSubmit"), tool("PreToolUse", "Bash"), ev("Notification")]);
        assert_eq!(t.state, Some(State::Waiting));
        t.apply(&tool("PostToolUse", "Bash"));
        assert_eq!(t.state, Some(State::Working));
        t.apply(&ev("Stop"));
        assert_eq!(t.state, Some(State::Done));
    }

    #[test]
    fn idle_reminder_after_done_is_not_a_question() {
        let t = replay(&[ev("UserPromptSubmit"), ev("Stop"), ev("Notification")]);
        assert_eq!(t.state, Some(State::Done));
    }

    #[test]
    fn subagents_outliving_stop_keep_it_working() {
        let mut t = replay(&[
            ev("UserPromptSubmit"),
            tool("PreToolUse", "Task"),
            tool("PreToolUse", "Agent"),
            ev("Stop"),
        ]);
        assert_eq!((t.state, t.subagents), (Some(State::Working), 2));
        t.apply(&ev("SubagentStop"));
        assert_eq!((t.state, t.subagents), (Some(State::Working), 1));
        t.apply(&ev("SubagentStop"));
        assert_eq!((t.state, t.subagents), (Some(State::Done), 0));
    }

    #[test]
    fn a_new_prompt_resets_a_drifted_counter() {
        let t = replay(&[tool("PreToolUse", "Task"), ev("UserPromptSubmit"), ev("Stop")]);
        assert_eq!((t.state, t.subagents), (Some(State::Done), 0));
    }

    #[test]
    fn malformed_and_unknown_lines_change_nothing() {
        let mut t = Tracker::default();
        assert!(!t.apply("{not json"));
        assert!(!t.apply(&ev("SessionStart")));
        assert_eq!(t.state, None);
    }

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("miniterm-status-{tag}-{}", uuid::Uuid::new_v4()));
        reset_dir(&d).unwrap();
        d
    }

    fn append(dir: &Path, id: u64, s: &str) {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(format!("{id}.jsonl")))
            .unwrap();
        f.write_all(s.as_bytes()).unwrap();
    }

    #[test]
    fn watcher_tails_lines_and_waits_for_a_partial_one() {
        let dir = tmp("tail");
        let mut w = Watcher::default();
        let ages = [(7, Duration::from_secs(60))];

        let prompt = ev("UserPromptSubmit");
        append(&dir, 7, &format!("{prompt}\n{}", &ev("Stop")[..10]));
        let ups = w.tick(&dir, &ages);
        assert_eq!(ups, vec![Update { id: 7, state: State::Working, subagents: 0 }]);

        append(&dir, 7, &format!("{}\n", &ev("Stop")[10..]));
        let ups = w.tick(&dir, &ages);
        assert_eq!(ups, vec![Update { id: 7, state: State::Done, subagents: 0 }]);

        assert!(w.tick(&dir, &ages).is_empty(), "nothing new, nothing sent");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn watcher_deletes_files_of_dead_sessions() {
        let dir = tmp("dead");
        append(&dir, 3, &format!("{}\n", ev("Stop")));
        let mut w = Watcher::default();
        w.tick(&dir, &[]);
        assert!(!dir.join("3.jsonl").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hookless_sessions_get_active_idle_hints_on_change_only() {
        let dir = tmp("hint");
        let mut w = Watcher::default();
        let ups = w.tick(&dir, &[(1, Duration::from_millis(100))]);
        assert_eq!(ups, vec![Update { id: 1, state: State::Active, subagents: 0 }]);
        assert!(w.tick(&dir, &[(1, Duration::from_millis(900))]).is_empty());
        let ups = w.tick(&dir, &[(1, Duration::from_secs(5))]);
        assert_eq!(ups, vec![Update { id: 1, state: State::Idle, subagents: 0 }]);

        // Once hooks speak for a session, the hint stops.
        append(&dir, 1, &format!("{}\n", ev("UserPromptSubmit")));
        let ups = w.tick(&dir, &[(1, Duration::from_millis(10))]);
        assert_eq!(ups, vec![Update { id: 1, state: State::Working, subagents: 0 }]);
        let _ = fs::remove_dir_all(&dir);
    }
}
