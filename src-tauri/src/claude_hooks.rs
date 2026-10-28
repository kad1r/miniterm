//! Installs and removes miniterm's status hook in `~/.claude/settings.json`.
//!
//! Only ever touched from an explicit button in Settings. Entries are merged
//! into whatever hooks the user already has and are found again by `MARKER`,
//! so removal takes out ours and nothing else.

use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const MARKER: &str = "miniterm-agent-status";

/// Forwards the hook's stdin JSON as one line. The guard makes it a no-op for
/// any Claude session not started inside miniterm, which matters because the
/// file is global. Prints nothing and always exits 0, so it can never block or
/// change what Claude does.
pub const COMMAND: &str = concat!(
    r#"[ -n "$MINITERM_STATUS_DIR" ] && { tr -d '\r\n'; echo; } >> "$MINITERM_STATUS_DIR/$MINITERM_SESSION_ID.jsonl"; true # "#,
    "miniterm-agent-status"
);

pub const EVENTS: [&str; 6] = [
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Notification",
    "Stop",
    "SubagentStop",
];

fn is_tool_event(event: &str) -> bool {
    event == "PreToolUse" || event == "PostToolUse"
}

fn entry(event: &str) -> Value {
    let hook = json!({ "type": "command", "command": COMMAND, "timeout": 5 });
    if is_tool_event(event) {
        json!({ "matcher": "*", "hooks": [hook] })
    } else {
        json!({ "hooks": [hook] })
    }
}

fn is_ours(group: &Value) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hs| {
            hs.iter().any(|h| {
                h.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.contains(MARKER))
            })
        })
}

pub fn is_installed(settings: &Value) -> bool {
    let Some(hooks) = settings.get("hooks") else { return false };
    EVENTS.iter().all(|e| {
        hooks
            .get(*e)
            .and_then(Value::as_array)
            .is_some_and(|groups| groups.iter().any(is_ours))
    })
}

/// Adds our entry to every event that lacks one. Idempotent. Fails rather than
/// guess when the file's shape is not what Claude Code writes.
pub fn install(mut settings: Value) -> Result<Value, String> {
    let root = settings
        .as_object_mut()
        .ok_or("settings.json is not a JSON object")?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or("\"hooks\" in settings.json is not an object")?;
    for event in EVENTS {
        let groups = hooks
            .entry(event)
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| format!("\"hooks.{event}\" in settings.json is not a list"))?;
        if !groups.iter().any(is_ours) {
            groups.push(entry(event));
        }
    }
    Ok(settings)
}

/// Removes our entries only. A list we emptied goes too, and so does `hooks`
/// if nothing else is left in it.
pub fn uninstall(mut settings: Value) -> Value {
    let Some(root) = settings.as_object_mut() else { return settings };
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return settings;
    };
    for event in EVENTS {
        if let Some(groups) = hooks.get_mut(event).and_then(Value::as_array_mut) {
            let had = groups.len();
            groups.retain(|g| !is_ours(g));
            if groups.is_empty() && had > 0 {
                hooks.remove(event);
            }
        }
    }
    if hooks.is_empty() {
        root.remove("hooks");
    }
    settings
}

pub fn settings_path(home: &Path) -> PathBuf {
    home.join(".claude").join("settings.json")
}

/// A missing file reads as `{}`. An unreadable or invalid one is an error —
/// writing over a file we could not parse would lose the user's settings.
pub fn read(path: &Path) -> Result<Value, String> {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(json!({})),
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("{} is not valid JSON: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

/// Backup of the current file next to it, then temp file + rename.
pub fn write(path: &Path, settings: &Value) -> Result<(), String> {
    let dir = path.parent().ok_or("settings path has no parent")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    if path.exists() {
        fs::copy(path, path.with_extension("json.miniterm-bak")).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("json.{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&tmp, text + "\n").map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        e.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_into_an_empty_file_adds_every_event() {
        let s = install(json!({})).unwrap();
        assert!(is_installed(&s));
        assert_eq!(s["hooks"]["PreToolUse"][0]["matcher"], "*");
        assert!(s["hooks"]["Stop"][0].get("matcher").is_none());
    }

    #[test]
    fn install_keeps_existing_hooks_and_settings() {
        let mine = json!({ "hooks": [{ "type": "command", "command": "my-stop.sh" }] });
        let s = install(json!({ "theme": "dark", "hooks": { "Stop": [mine.clone()] } })).unwrap();
        assert_eq!(s["theme"], "dark");
        assert_eq!(s["hooks"]["Stop"][0], mine);
        assert_eq!(s["hooks"]["Stop"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn install_twice_adds_nothing_the_second_time() {
        let once = install(json!({})).unwrap();
        assert_eq!(install(once.clone()).unwrap(), once);
    }

    #[test]
    fn install_refuses_a_shape_it_does_not_know() {
        assert!(install(json!([])).is_err());
        assert!(install(json!({ "hooks": "nope" })).is_err());
        assert!(install(json!({ "hooks": { "Stop": {} } })).is_err());
    }

    #[test]
    fn uninstall_removes_only_ours() {
        let mine = json!({ "hooks": [{ "type": "command", "command": "my-stop.sh" }] });
        let before = json!({ "theme": "dark", "hooks": { "Stop": [mine.clone()] } });
        let after = uninstall(install(before.clone()).unwrap());
        assert_eq!(after, before);
        assert!(!is_installed(&after));
    }

    #[test]
    fn uninstall_drops_hooks_when_nothing_else_is_left() {
        assert_eq!(uninstall(install(json!({})).unwrap()), json!({}));
    }

    #[test]
    fn write_backs_up_then_replaces() {
        let dir = std::env::temp_dir().join(format!("miniterm-hooks-{}", uuid::Uuid::new_v4()));
        let path = settings_path(&dir);
        assert_eq!(read(&path).unwrap(), json!({}), "missing file reads as empty");
        write(&path, &json!({ "a": 1 })).unwrap();
        write(&path, &json!({ "a": 2 })).unwrap();
        assert_eq!(read(&path).unwrap(), json!({ "a": 2 }));
        let bak = path.with_extension("json.miniterm-bak");
        assert_eq!(read(&bak).unwrap(), json!({ "a": 1 }));
        fs::write(&path, "{ broken").unwrap();
        assert!(read(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
