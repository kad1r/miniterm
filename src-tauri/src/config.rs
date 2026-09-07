use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const CONFIG_VERSION: u32 = 1;
const FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiTool {
    pub id: String,
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DirAlias {
    pub id: String,
    pub alias: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Node {
    #[serde(rename_all = "camelCase")]
    Folder {
        id: String,
        name: String,
        expanded: bool,
        children: Vec<Node>,
    },
    #[serde(rename_all = "camelCase")]
    Workspace {
        id: String,
        name: String,
        path: String,
        ai_tool_id: Option<String>,
        shell_id: Option<String>,
        rows: u8,
        cols: u8,
        row_sizes: Vec<f64>,
        col_sizes: Vec<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub version: u32,
    pub ai_tools: Vec<AiTool>,
    pub directories: Vec<DirAlias>,
    pub recent_dirs: Vec<String>,
    pub default_shell_id: String,
    pub tree: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LoadStatus {
    Fresh,
    Loaded,
    #[serde(rename_all = "camelCase")]
    Recovered { backup: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadResult {
    pub config: Config,
    pub status: LoadStatus,
}

pub fn default_config(default_shell_id: String) -> Config {
    Config {
        version: CONFIG_VERSION,
        ai_tools: vec![
            AiTool {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Claude Code".into(),
                command: "claude".into(),
            },
            AiTool {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Gemini".into(),
                command: "gemini".into(),
            },
        ],
        directories: Vec::new(),
        recent_dirs: Vec::new(),
        default_shell_id,
        tree: Vec::new(),
    }
}

pub fn load(dir: &Path, default_shell_id: String) -> LoadResult {
    let path = dir.join(FILE_NAME);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => {
            return LoadResult {
                config: default_config(default_shell_id),
                status: LoadStatus::Fresh,
            };
        }
    };

    match serde_json::from_str::<Config>(&raw) {
        Ok(config) if config.version == CONFIG_VERSION => {
            LoadResult { config, status: LoadStatus::Loaded }
        }
        _ => {
            let backup: PathBuf = dir.join(format!("{FILE_NAME}.bak"));
            let _ = fs::remove_file(&backup);
            match fs::rename(&path, &backup) {
                Ok(_) => {},
                Err(_) => {
                    let _ = fs::copy(&path, &backup);
                    let _ = fs::remove_file(&path);
                }
            }
            LoadResult {
                config: default_config(default_shell_id),
                status: LoadStatus::Recovered {
                    backup: backup.to_string_lossy().to_string(),
                },
            }
        }
    }
}

/// Atomic write: temp file in same directory, fsync, then rename.
/// A crash during write cannot corrupt the existing config.
pub fn save(dir: &Path, config: &Config) -> std::io::Result<()> {
    use std::io::Write;

    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{FILE_NAME}.{}.tmp", Uuid::new_v4()));
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
    }

    fs::rename(&tmp, dir.join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("miniterm-test-{}-{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn missing_file_yields_fresh_defaults() {
        let d = tmp("missing");
        let r = load(&d, "pwsh".into());
        assert!(matches!(r.status, LoadStatus::Fresh));
        assert_eq!(r.config.version, CONFIG_VERSION);
        assert_eq!(r.config.default_shell_id, "pwsh");
        assert_eq!(r.config.ai_tools.len(), 2);
        assert_eq!(r.config.ai_tools[0].name, "Claude Code");
        assert_eq!(r.config.ai_tools[0].command, "claude");
        assert_eq!(r.config.ai_tools[1].name, "Gemini");
        assert_eq!(r.config.ai_tools[1].command, "gemini");
        assert!(r.config.tree.is_empty());
    }

    #[test]
    fn round_trips_a_saved_config() {
        let d = tmp("roundtrip");
        let mut c = default_config("bash".into());
        c.recent_dirs.push("/tmp/x".into());
        c.tree.push(Node::Workspace {
            id: "w1".into(), name: "api".into(), path: "/tmp/x".into(),
            ai_tool_id: Some("t1".into()), shell_id: None,
            rows: 2, cols: 2,
            row_sizes: vec![0.5, 0.5], col_sizes: vec![0.5, 0.5],
        });
        save(&d, &c).unwrap();

        let r = load(&d, "ignored".into());
        assert!(matches!(r.status, LoadStatus::Loaded));
        assert_eq!(r.config.default_shell_id, "bash");
        assert_eq!(r.config.recent_dirs, vec!["/tmp/x".to_string()]);
        assert_eq!(r.config.tree.len(), 1);
    }

    #[test]
    fn serialises_field_names_as_camel_case() {
        let d = tmp("camel");
        let mut c = default_config("bash".into());
        c.tree.push(Node::Folder {
            id: "f1".into(), name: "grp".into(), expanded: true, children: vec![],
        });
        save(&d, &c).unwrap();
        let raw = fs::read_to_string(d.join("config.json")).unwrap();
        assert!(raw.contains("\"aiTools\""));
        assert!(raw.contains("\"defaultShellId\""));
        assert!(raw.contains("\"kind\": \"folder\"") || raw.contains("\"kind\":\"folder\""));
    }

    #[test]
    fn corrupt_file_is_moved_aside_and_defaults_returned() {
        let d = tmp("corrupt");
        fs::write(d.join("config.json"), b"{ not json").unwrap();
        let r = load(&d, "cmd".into());
        match r.status {
            LoadStatus::Recovered { ref backup } => assert!(backup.ends_with("config.json.bak")),
            other => panic!("expected Recovered, got {:?}", other),
        }
        assert!(d.join("config.json.bak").exists());
        assert_eq!(r.config.default_shell_id, "cmd");
    }

    #[test]
    fn unknown_version_is_treated_as_corrupt() {
        let d = tmp("version");
        fs::write(d.join("config.json"), br#"{"version":99,"aiTools":[],"directories":[],"recentDirs":[],"defaultShellId":"x","tree":[]}"#).unwrap();
        let r = load(&d, "cmd".into());
        assert!(matches!(r.status, LoadStatus::Recovered { .. }));
    }

    #[test]
    fn save_leaves_no_temp_file_behind() {
        let d = tmp("atomic");
        save(&d, &default_config("bash".into())).unwrap();
        let names: Vec<_> = fs::read_dir(&d).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["config.json".to_string()]);
    }
}
