//! Prompt history read from Claude Code's own transcripts.
//!
//! Claude Code writes every session to `~/.claude/projects/<project>/<id>.jsonl`
//! (subagents to `<id>/subagents/*.jsonl`). Each prompt the person typed is
//! paired here with what the turn it started cost: tokens, model calls, tool
//! calls and how long the answer took. Nothing is written; files are only read.
//!
//! Transcripts add up to hundreds of MB, so parsed files are cached by size and
//! mtime and a re-scan only re-reads the sessions that grew.

use serde::de::{self, Deserializer, IgnoredAny, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub session_id: String,
    /// Session title: the one the person set, else the one Claude generated.
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub branch: Option<String>,
    /// ISO-8601 as written in the transcript.
    pub timestamp: String,
    /// Unix epoch milliseconds, for sorting and arithmetic.
    pub time_ms: i64,
    pub text: String,
    pub images: u32,
    /// `typed`, `queued`, `suggestion_accepted`, `sdk`… when the transcript says.
    pub source: Option<String>,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    /// Everything the turn's subagents used, all four kinds together.
    pub subagent_tokens: u64,
    pub api_calls: u32,
    pub tool_calls: u32,
    /// Prompt to the first model reply.
    pub first_reply_ms: Option<u64>,
    /// Prompt to the end of the turn.
    pub duration_ms: Option<u64>,
    /// Usage per model, the turn's own calls and its subagents' kept apart.
    pub models: Vec<ModelUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub subagent: bool,
    pub calls: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

/// One API reply: its model and the last usage seen for it.
#[derive(Clone)]
struct Call {
    model: Option<String>,
    usage: Usage,
}

/// Sums replies per (model, subagent) into `out`.
fn add_calls<'a>(out: &mut Vec<ModelUsage>, calls: impl Iterator<Item = &'a Call>, subagent: bool) {
    for c in calls {
        let Some(model) = c.model.clone() else { continue };
        let i = match out.iter().position(|m| m.model == model && m.subagent == subagent) {
            Some(i) => i,
            None => {
                out.push(ModelUsage { model, subagent, ..ModelUsage::default() });
                out.len() - 1
            }
        };
        let m = &mut out[i];
        m.calls += 1;
        m.input_tokens += c.usage.input_tokens.unwrap_or(0);
        m.output_tokens += c.usage.output_tokens.unwrap_or(0);
        m.cache_read_tokens += c.usage.cache_read_input_tokens.unwrap_or(0);
        m.cache_write_tokens += c.usage.cache_creation_input_tokens.unwrap_or(0);
    }
}

/// `<synthetic>` and friends are Claude Code placeholders, not models.
fn real_model(m: Option<String>) -> Option<String> {
    m.filter(|m| !m.is_empty() && !m.starts_with('<'))
}

// ---------------------------------------------------------------- parsing ---

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    subtype: Option<String>,
    message: Option<Msg>,
    timestamp: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
    prompt_id: Option<String>,
    origin: Option<Origin>,
    prompt_source: Option<String>,
    is_meta: Option<bool>,
    is_sidechain: Option<bool>,
    is_compact_summary: Option<bool>,
    duration_ms: Option<u64>,
    ai_title: Option<String>,
    custom_title: Option<String>,
}

#[derive(Deserialize)]
struct Origin {
    kind: Option<String>,
}

#[derive(Deserialize)]
struct Msg {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
    content: Option<Content>,
}

#[derive(Deserialize, Clone, Copy, Default)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

impl Usage {
    fn total(&self) -> u64 {
        self.input_tokens.unwrap_or(0)
            + self.output_tokens.unwrap_or(0)
            + self.cache_creation_input_tokens.unwrap_or(0)
            + self.cache_read_input_tokens.unwrap_or(0)
    }
}

/// Message content, reduced while it is parsed: tool inputs and tool results
/// can be megabytes and are skipped rather than built into values.
#[derive(Default)]
struct Content {
    text: String,
    images: u32,
    tool_uses: u32,
    tool_results: u32,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: Option<String>,
    text: Option<String>,
}

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Content;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or a list of content blocks")
            }
            fn visit_str<E: de::Error>(self, s: &str) -> Result<Content, E> {
                Ok(Content { text: s.to_owned(), ..Content::default() })
            }
            fn visit_string<E: de::Error>(self, s: String) -> Result<Content, E> {
                Ok(Content { text: s, ..Content::default() })
            }
            fn visit_unit<E: de::Error>(self) -> Result<Content, E> {
                Ok(Content::default())
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Content, A::Error> {
                let mut c = Content::default();
                while let Some(b) = seq.next_element::<Block>()? {
                    match b.kind.as_deref() {
                        Some("text") => {
                            if let Some(t) = b.text {
                                if !c.text.is_empty() {
                                    c.text.push('\n');
                                }
                                c.text.push_str(&t);
                            }
                        }
                        Some("image") => c.images += 1,
                        Some("tool_use") => c.tool_uses += 1,
                        Some("tool_result") => c.tool_results += 1,
                        _ => {}
                    }
                }
                Ok(c)
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Content, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(Content::default())
            }
        }
        d.deserialize_any(V)
    }
}

/// Text Claude Code itself injects as a user message in transcripts that
/// predate the `origin` field.
const SYNTHETIC_PREFIXES: [&str; 6] = [
    "<local-command",
    "<task-notification",
    "<system-reminder",
    "<bash-",
    "Caveat: ",
    "This session is being continued",
];

/// The prompt text, or None when the line is not something the person sent.
fn human_text(l: &Line) -> Option<(String, u32)> {
    if l.kind.as_deref() != Some("user")
        || l.is_meta == Some(true)
        || l.is_sidechain == Some(true)
        || l.is_compact_summary == Some(true)
    {
        return None;
    }
    let c = l.message.as_ref()?.content.as_ref()?;
    if c.tool_results > 0 || (c.text.trim().is_empty() && c.images == 0) {
        return None;
    }
    match l.origin.as_ref().and_then(|o| o.kind.as_deref()) {
        Some("human") => {}
        Some(_) => return None,
        None if SYNTHETIC_PREFIXES.iter().any(|p| c.text.starts_with(p)) => return None,
        None => {}
    }
    Some((slash_command(&c.text).unwrap_or_else(|| c.text.clone()), c.images))
}

/// `<command-name>/x</command-name>…<command-args>y</command-args>` → `/x y`.
fn slash_command(text: &str) -> Option<String> {
    let name = between(text, "<command-name>", "</command-name>")?;
    let args = between(text, "<command-args>", "</command-args>").unwrap_or("");
    let name = if name.starts_with('/') { name.to_owned() } else { format!("/{name}") };
    Some(if args.trim().is_empty() { name } else { format!("{name} {}", args.trim()) })
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = s.find(open)? + open.len();
    let end = s[start..].find(close)? + start;
    Some(&s[start..end])
}

/// `2026-11-19T06:22:05.660Z` → epoch ms. Offsets other than `Z` are honoured.
pub fn parse_iso_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, sec) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut i = 19;
    let mut ms = 0;
    if b.get(i) == Some(&b'.') {
        let start = i + 1;
        i = start;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let frac = &s[start..i];
        let padded: String = frac.chars().chain("000".chars()).take(3).collect();
        ms = padded.parse::<i64>().ok()?;
    }
    let offset_min = match b.get(i) {
        None | Some(b'Z') | Some(b'z') => 0,
        Some(&sign @ (b'+' | b'-')) => {
            let oh = num(i + 1..i + 3)?;
            let om = num(i + 4..i + 6).or_else(|| num(i + 3..i + 5)).unwrap_or(0);
            let m = oh * 60 + om;
            if sign == b'+' { m } else { -m }
        }
        _ => return None,
    };
    // Howard Hinnant's days_from_civil.
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 24 + h) * 60 + mi - offset_min) * 60_000 + sec * 1000 + ms)
}

/// One open turn while a transcript is replayed.
struct Turn {
    prompt: Prompt,
    /// Every promptId seen while this turn was open: a background-task
    /// follow-up gets its own id, and subagents it starts carry that one.
    prompt_ids: Vec<String>,
    /// Per API message id the last usage seen: a reply is split over several
    /// lines that all repeat the message's usage.
    usage: HashMap<String, Call>,
    sidechain: HashMap<String, Call>,
    last_ms: Option<i64>,
}

impl Turn {
    fn finish(mut self) -> (Prompt, Vec<String>) {
        let p = &mut self.prompt;
        for Call { usage: u, .. } in self.usage.values() {
            p.input_tokens += u.input_tokens.unwrap_or(0);
            p.output_tokens += u.output_tokens.unwrap_or(0);
            p.cache_read_tokens += u.cache_read_input_tokens.unwrap_or(0);
            p.cache_write_tokens += u.cache_creation_input_tokens.unwrap_or(0);
        }
        p.api_calls = self.usage.len() as u32;
        p.subagent_tokens += self.sidechain.values().map(|c| c.usage.total()).sum::<u64>();
        add_calls(&mut p.models, self.usage.values(), false);
        add_calls(&mut p.models, self.sidechain.values(), true);
        if p.duration_ms.is_none() {
            p.duration_ms = self
                .last_ms
                .filter(|&l| l >= p.time_ms)
                .map(|l| (l - p.time_ms) as u64);
        }
        (self.prompt, self.prompt_ids)
    }
}

fn lines_of(path: &Path, mut f: impl FnMut(Line)) {
    let Ok(file) = fs::File::open(path) else { return };
    let mut reader = BufReader::with_capacity(1 << 20, file);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if let Ok(line) = serde_json::from_slice::<Line>(&buf) {
                    f(line);
                }
            }
        }
    }
}

/// Every prompt of one session transcript, with its subagent usage folded in.
pub fn parse_session(main: &Path, subagent_files: &[PathBuf]) -> Vec<Prompt> {
    let mut done: Vec<(Prompt, Vec<String>)> = Vec::new();
    let mut turn: Option<Turn> = None;
    let mut ai_title = None;
    let mut custom_title = None;

    lines_of(main, |l| {
        match l.kind.as_deref() {
            Some("ai-title") => {
                if l.ai_title.is_some() {
                    ai_title = l.ai_title;
                }
                return;
            }
            Some("custom-title") => {
                if l.custom_title.is_some() {
                    custom_title = l.custom_title;
                }
                return;
            }
            _ => {}
        }
        if let Some((text, images)) = human_text(&l) {
            if let Some(t) = turn.take() {
                done.push(t.finish());
            }
            let timestamp = l.timestamp.clone().unwrap_or_default();
            turn = Some(Turn {
                prompt: Prompt {
                    session_id: l.session_id.clone().unwrap_or_default(),
                    cwd: l.cwd.clone(),
                    branch: l.git_branch.clone().filter(|b| !b.is_empty() && b != "HEAD"),
                    time_ms: parse_iso_ms(&timestamp).unwrap_or(0),
                    timestamp,
                    text,
                    images,
                    source: l.prompt_source.clone(),
                    ..Prompt::default()
                },
                prompt_ids: l.prompt_id.clone().into_iter().collect(),
                usage: HashMap::new(),
                sidechain: HashMap::new(),
                last_ms: None,
            });
            return;
        }
        let Some(t) = turn.as_mut() else { return };
        if let Some(pid) = &l.prompt_id {
            if !t.prompt_ids.contains(pid) {
                t.prompt_ids.push(pid.clone());
            }
        }
        match l.kind.as_deref() {
            Some("assistant") => {
                let Some(m) = l.message else { return };
                let ms = l.timestamp.as_deref().and_then(parse_iso_ms);
                let model = real_model(m.model);
                if l.is_sidechain == Some(true) {
                    if let (Some(id), Some(usage)) = (m.id, m.usage) {
                        t.sidechain.insert(id, Call { model, usage });
                    }
                    return;
                }
                if let Some(ms) = ms {
                    if t.prompt.first_reply_ms.is_none() && ms >= t.prompt.time_ms {
                        t.prompt.first_reply_ms = Some((ms - t.prompt.time_ms) as u64);
                    }
                    t.last_ms = Some(ms);
                }
                if model.is_some() {
                    t.prompt.model = model.clone();
                }
                t.prompt.tool_calls += m.content.map_or(0, |c| c.tool_uses);
                if let (Some(id), Some(usage)) = (m.id, m.usage) {
                    t.usage.insert(id, Call { model, usage });
                }
            }
            // Only the turn the prompt started; later turns in the same span
            // are background-task follow-ups.
            Some("system") if l.subtype.as_deref() == Some("turn_duration") => {
                if t.prompt.duration_ms.is_none() {
                    t.prompt.duration_ms = l.duration_ms;
                }
            }
            _ => {}
        }
    });
    if let Some(t) = turn.take() {
        done.push(t.finish());
    }

    if !subagent_files.is_empty() {
        let mut by_prompt: HashMap<String, HashMap<String, Call>> = HashMap::new();
        for f in subagent_files {
            // Only the opening user line carries the promptId; replies inherit it.
            let mut current: Option<String> = None;
            lines_of(f, |l| {
                if l.prompt_id.is_some() {
                    current = l.prompt_id;
                }
                if l.kind.as_deref() != Some("assistant") {
                    return;
                }
                let (Some(pid), Some(m)) = (current.clone(), l.message) else { return };
                if let (Some(id), Some(usage)) = (m.id, m.usage) {
                    let model = real_model(m.model);
                    by_prompt.entry(pid).or_default().insert(id, Call { model, usage });
                }
            });
        }
        for (p, pids) in &mut done {
            for calls in pids.iter().filter_map(|pid| by_prompt.get(pid)) {
                p.subagent_tokens += calls.values().map(|c| c.usage.total()).sum::<u64>();
                add_calls(&mut p.models, calls.values(), true);
            }
        }
    }

    let title = custom_title.or(ai_title);
    done.into_iter()
        .map(|(mut p, _)| {
            p.title = title.clone();
            p.models.sort_by_key(|m| {
                std::cmp::Reverse(
                    m.input_tokens + m.output_tokens + m.cache_read_tokens + m.cache_write_tokens,
                )
            });
            p
        })
        .collect()
}

// ------------------------------------------------------------------ cache ---

type Stamp = (u64, Option<SystemTime>);

#[derive(Clone, PartialEq)]
struct Signature {
    main: Stamp,
    subagents: Vec<Stamp>,
}

struct Job {
    main: PathBuf,
    subagents: Vec<PathBuf>,
    sig: Signature,
}

#[derive(Default)]
pub struct Cache {
    sessions: HashMap<PathBuf, (Signature, Vec<Prompt>)>,
}

fn stamp(p: &Path) -> Option<Stamp> {
    let m = fs::metadata(p).ok()?;
    Some((m.len(), m.modified().ok()))
}

fn jsonl_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl") && p.is_file())
        .collect();
    v.sort();
    v
}

fn find_jobs(projects: &Path) -> Vec<Job> {
    let mut jobs = Vec::new();
    for project in fs::read_dir(projects).into_iter().flatten().flatten() {
        let dir = project.path();
        if !dir.is_dir() {
            continue;
        }
        for main in jsonl_in(&dir) {
            let Some(main_stamp) = stamp(&main) else { continue };
            let subagents = jsonl_in(&main.with_extension("").join("subagents"));
            let sub_stamps = subagents.iter().filter_map(|p| stamp(p)).collect();
            jobs.push(Job {
                main,
                subagents,
                sig: Signature { main: main_stamp, subagents: sub_stamps },
            });
        }
    }
    jobs
}

impl Cache {
    /// Every prompt under `projects`, newest first. Unchanged sessions come
    /// from the cache; the rest are parsed in parallel.
    pub fn scan(&mut self, projects: &Path) -> Vec<Prompt> {
        let jobs = find_jobs(projects);
        let stale: Vec<&Job> = jobs
            .iter()
            .filter(|j| self.sessions.get(&j.main).is_none_or(|(sig, _)| *sig != j.sig))
            .collect();

        let results: Mutex<Vec<(PathBuf, Signature, Vec<Prompt>)>> = Mutex::new(Vec::new());
        let next = AtomicUsize::new(0);
        let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
        std::thread::scope(|s| {
            for _ in 0..workers.min(stale.len()) {
                s.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = stale.get(i) else { break };
                    let prompts = parse_session(&job.main, &job.subagents);
                    if let Ok(mut r) = results.lock() {
                        r.push((job.main.clone(), job.sig.clone(), prompts));
                    }
                });
            }
        });
        for (path, sig, prompts) in results.into_inner().unwrap_or_default() {
            self.sessions.insert(path, (sig, prompts));
        }
        let live: std::collections::HashSet<&PathBuf> = jobs.iter().map(|j| &j.main).collect();
        self.sessions.retain(|p, _| live.contains(p));

        let mut all: Vec<Prompt> =
            self.sessions.values().flat_map(|(_, ps)| ps.iter().cloned()).collect();
        all.sort_by(|a, b| b.time_ms.cmp(&a.time_ms));
        all
    }
}

pub fn projects_dir(home: &Path) -> PathBuf {
    home.join(".claude").join("projects")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("miniterm-prompts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(path: &Path, lines: &[&str]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = fs::File::create(path).unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
    }

    const PROMPT: &str = r#"{"type":"user","promptId":"p1","message":{"role":"user","content":"sidebar fontunu degistir"},"timestamp":"2026-11-19T06:22:05.000Z","origin":{"kind":"human"},"promptSource":"typed","cwd":"D:\\x","gitBranch":"main","sessionId":"s1"}"#;

    #[test]
    fn iso_timestamps_parse_to_epoch_ms() {
        assert_eq!(parse_iso_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso_ms("1970-01-01T00:00:01.5Z"), Some(1500));
        assert_eq!(parse_iso_ms("2026-11-19T06:22:05.660Z"), Some(1_795_069_325_660));
        assert_eq!(parse_iso_ms("2026-11-19T09:22:05.660+03:00"), Some(1_795_069_325_660));
        assert_eq!(parse_iso_ms("garbage"), None);
    }

    #[test]
    fn a_turn_collects_tokens_tools_and_timing() {
        let dir = tmp();
        let main = dir.join("p").join("s1.jsonl");
        write(&main, &[
            r#"{"type":"ai-title","aiTitle":"Font işi","sessionId":"s1"}"#,
            PROMPT,
            // One reply split over two lines that repeat the usage: counted once.
            r#"{"type":"assistant","message":{"id":"m1","model":"claude-x","content":[{"type":"text","text":"ok"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":20}},"timestamp":"2026-11-19T06:22:07.000Z"}"#,
            r#"{"type":"assistant","message":{"id":"m1","model":"claude-x","content":[{"type":"tool_use","id":"t","name":"Bash","input":{"command":"ls"}}],"usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":20}},"timestamp":"2026-11-19T06:22:08.000Z"}"#,
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":"huge"}]},"timestamp":"2026-11-19T06:22:09.000Z"}"#,
            r#"{"type":"assistant","message":{"id":"m2","model":"claude-x","content":[{"type":"text","text":"done"}],"usage":{"input_tokens":1,"output_tokens":7}},"timestamp":"2026-11-19T06:22:15.000Z"}"#,
            r#"{"type":"system","subtype":"turn_duration","durationMs":10500}"#,
            r#"{"type":"custom-title","customTitle":"Benim başlığım"}"#,
        ]);
        let ps = parse_session(&main, &[]);
        assert_eq!(ps.len(), 1);
        let p = &ps[0];
        assert_eq!(p.text, "sidebar fontunu degistir");
        assert_eq!(p.title.as_deref(), Some("Benim başlığım"));
        assert_eq!(p.branch.as_deref(), Some("main"));
        assert_eq!((p.input_tokens, p.output_tokens), (11, 12));
        assert_eq!((p.cache_read_tokens, p.cache_write_tokens), (100, 20));
        assert_eq!((p.api_calls, p.tool_calls), (2, 1));
        assert_eq!(p.first_reply_ms, Some(2000));
        assert_eq!(p.duration_ms, Some(10500));
        assert_eq!(p.model.as_deref(), Some("claude-x"));
        assert_eq!(p.models.len(), 1);
        assert_eq!((p.models[0].calls, p.models[0].output_tokens, p.models[0].subagent), (2, 12, false));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn synthetic_user_lines_are_not_prompts() {
        let dir = tmp();
        let main = dir.join("p").join("s1.jsonl");
        write(&main, &[
            r#"{"type":"user","message":{"role":"user","content":"<task-notification>x</task-notification>"},"origin":{"kind":"task-notification"},"timestamp":"2026-11-19T06:00:00Z"}"#,
            r#"{"type":"user","message":{"role":"user","content":"[Image: ...]"},"isMeta":true,"timestamp":"2026-11-19T06:00:00Z"}"#,
            r#"{"type":"user","message":{"role":"user","content":"This session is being continued from a previous conversation"},"isCompactSummary":true,"timestamp":"2026-11-19T06:00:00Z"}"#,
            r#"{"type":"user","message":{"role":"user","content":"Caveat: The messages below were generated by the user"},"timestamp":"2026-11-19T06:00:00Z"}"#,
            r#"{"type":"user","message":{"role":"user","content":"<command-name>/model</command-name>\n<command-args>opus</command-args>"},"timestamp":"2026-11-19T06:00:01Z"}"#,
            r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"bu ekrana bak"},{"type":"image","source":{"type":"base64","data":"AAAA"}}]},"origin":{"kind":"human"},"timestamp":"2026-11-19T06:00:02Z"}"#,
        ]);
        let ps = parse_session(&main, &[]);
        let texts: Vec<_> = ps.iter().map(|p| (p.text.as_str(), p.images)).collect();
        assert_eq!(texts, vec![("/model opus", 0), ("bu ekrana bak", 1)]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn subagent_usage_is_added_to_its_prompt() {
        let dir = tmp();
        let main = dir.join("p").join("s1.jsonl");
        write(&main, &[
            PROMPT,
            r#"{"type":"user","promptId":"p2","message":{"role":"user","content":"<task-notification>x</task-notification>"},"origin":{"kind":"task-notification"}}"#,
        ]);
        let late = dir.join("p").join("s1").join("subagents").join("agent-b.jsonl");
        write(&late, &[
            r#"{"type":"user","promptId":"p2","isSidechain":true,"message":{"role":"user","content":"task"}}"#,
            r#"{"type":"assistant","isSidechain":true,"message":{"id":"y1","model":"claude-haiku","usage":{"output_tokens":10}}}"#,
        ]);
        let sub = dir.join("p").join("s1").join("subagents").join("agent-a.jsonl");
        write(&sub, &[
            r#"{"type":"assistant","promptId":"p1","isSidechain":true,"message":{"id":"x1","model":"claude-haiku","usage":{"input_tokens":3,"output_tokens":4}}}"#,
            r#"{"type":"assistant","promptId":"p1","isSidechain":true,"message":{"id":"x1","model":"claude-haiku","usage":{"input_tokens":3,"output_tokens":4}}}"#,
            r#"{"type":"assistant","promptId":"other","isSidechain":true,"message":{"id":"x2","usage":{"input_tokens":50}}}"#,
        ]);
        let mut cache = Cache::default();
        let ps = cache.scan(&dir);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].subagent_tokens, 17);
        let m = &ps[0].models[0];
        assert_eq!((m.model.as_str(), m.subagent, m.calls), ("claude-haiku", true, 2));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_reparses_only_changed_sessions_and_drops_deleted_ones() {
        let dir = tmp();
        let a = dir.join("p").join("a.jsonl");
        let b = dir.join("p").join("b.jsonl");
        write(&a, &[PROMPT]);
        write(&b, &[PROMPT]);
        let mut cache = Cache::default();
        assert_eq!(cache.scan(&dir).len(), 2);
        write(&a, &[PROMPT, &PROMPT.replace("06:22:05", "06:30:00")]);
        assert_eq!(cache.scan(&dir).len(), 3);
        fs::remove_file(&b).unwrap();
        assert_eq!(cache.scan(&dir).len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }
}
