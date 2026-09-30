//! Shared agent directory: one folder per workspace where every pane's agent
//! can read what the others are doing and leave them messages.
//!
//! ```text
//! <config>/agents/<workspace-id>/
//!   roster.md          who is here, and how to collaborate
//!   <Name>.log         recent terminal output of that agent (plain text)
//!   inbox/<Name>.md    messages for that agent
//! ```
//!
//! The frontend owns the contents; this module only resolves the folder and
//! refuses any name that could escape it.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const INBOX: &str = "inbox";

/// A workspace id or file stem: letters (any script), digits, `-`, `_`, space.
/// Anything else — separators, dots, drive colons — is refused, so a joined
/// path can never leave the agents folder.
fn safe_stem(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !s.starts_with(' ')
        && !s.ends_with(' ')
        && s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
}

/// `name.ext` or `inbox/name.ext`, with ext one of md/log.
fn safe_rel(rel: &str) -> Option<PathBuf> {
    let (dir, file) = match rel.split_once('/') {
        Some((d, f)) if d == INBOX => (Some(INBOX), f),
        Some(_) => return None,
        None => (None, rel),
    };
    let (stem, ext) = file.rsplit_once('.')?;
    if !safe_stem(stem) || !(ext == "md" || ext == "log") {
        return None;
    }
    let mut p = PathBuf::new();
    if let Some(d) = dir {
        p.push(d);
    }
    p.push(file);
    Some(p)
}

/// The workspace's agent folder, created (with its inbox) on first use.
pub fn dir(config_dir: &Path, workspace_id: &str) -> Result<PathBuf, String> {
    if !safe_stem(workspace_id) {
        return Err("invalid workspace id".into());
    }
    let d = config_dir.join("agents").join(workspace_id);
    fs::create_dir_all(d.join(INBOX)).map_err(|e| e.to_string())?;
    Ok(d)
}

fn resolve(config_dir: &Path, workspace_id: &str, rel: &str) -> Result<PathBuf, String> {
    let rel = safe_rel(rel).ok_or_else(|| format!("invalid agent file name: {rel}"))?;
    Ok(dir(config_dir, workspace_id)?.join(rel))
}

/// `roster.md` -> `roster.md.tmp`. The full name is kept, so `X.md` and `X.log`
/// (an agent named "roster", say) never write through the same temp file.
fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Replace a file's contents. Written to a temp file and renamed so a reader
/// in another pane never sees a half-written log.
pub fn write(config_dir: &Path, workspace_id: &str, rel: &str, content: &str) -> Result<(), String> {
    let path = resolve(config_dir, workspace_id, rel)?;
    let tmp = tmp_path(&path);
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Move a file (an agent was renamed). A missing source is not an error; an
/// existing target is kept rather than overwritten.
pub fn rename(config_dir: &Path, workspace_id: &str, from: &str, to: &str) -> Result<(), String> {
    let src = resolve(config_dir, workspace_id, from)?;
    let dst = resolve(config_dir, workspace_id, to)?;
    if !src.exists() || dst.exists() {
        return Ok(());
    }
    fs::rename(&src, &dst).map_err(|e| e.to_string())
}

pub fn remove(config_dir: &Path, workspace_id: &str, rel: &str) -> Result<(), String> {
    let path = resolve(config_dir, workspace_id, rel)?;
    match fs::remove_file(&path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

/// Drop a deleted workspace's whole folder.
pub fn remove_dir(config_dir: &Path, workspace_id: &str) -> Result<(), String> {
    if !safe_stem(workspace_id) {
        return Err("invalid workspace id".into());
    }
    let d = config_dir.join("agents").join(workspace_id);
    match fs::remove_dir_all(&d) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = std::env::temp_dir().join(format!("miniterm-agents-{tag}-{nanos}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn accepts_plain_and_inbox_names() {
        assert!(safe_rel("Atlas.log").is_some());
        assert!(safe_rel("roster.md").is_some());
        assert!(safe_rel("inbox/Nova.md").is_some());
        assert!(safe_rel("inbox/Çağrı.md").is_some());
    }

    #[test]
    fn refuses_escapes() {
        for bad in [
            "../x.md", "inbox/../x.md", "a/b.md", "C:\\x.md", "x.exe", ".md", "inbox/", "a\\b.log",
            "x..md", "other/Nova.md",
        ] {
            assert!(safe_rel(bad).is_none(), "{bad}");
        }
        assert!(dir(&tmp("bad"), "../evil").is_err());
    }

    #[test]
    fn write_rename_remove_roundtrip() {
        let cfg = tmp("rt");
        write(&cfg, "ws1", "Atlas.log", "hello").unwrap();
        let d = dir(&cfg, "ws1").unwrap();
        assert_eq!(fs::read_to_string(d.join("Atlas.log")).unwrap(), "hello");
        assert!(d.join(INBOX).is_dir());

        rename(&cfg, "ws1", "Atlas.log", "Nova.log").unwrap();
        assert!(!d.join("Atlas.log").exists());
        assert_eq!(fs::read_to_string(d.join("Nova.log")).unwrap(), "hello");

        remove(&cfg, "ws1", "Nova.log").unwrap();
        remove(&cfg, "ws1", "Nova.log").unwrap(); // missing is fine
        assert!(!d.join("Nova.log").exists());

        remove_dir(&cfg, "ws1").unwrap();
        assert!(!d.exists());
    }

    #[test]
    fn temp_files_are_distinct_per_extension() {
        let d = Path::new("agents/w");
        assert_ne!(tmp_path(&d.join("roster.md")), tmp_path(&d.join("roster.log")));
        assert_eq!(tmp_path(&d.join("roster.md")), d.join("roster.md.tmp"));
    }

    #[test]
    fn rename_never_clobbers() {
        let cfg = tmp("clobber");
        write(&cfg, "w", "inbox/A.md", "a").unwrap();
        write(&cfg, "w", "inbox/B.md", "b").unwrap();
        rename(&cfg, "w", "inbox/A.md", "inbox/B.md").unwrap();
        let d = dir(&cfg, "w").unwrap();
        assert_eq!(fs::read_to_string(d.join("inbox/B.md")).unwrap(), "b");
    }
}
