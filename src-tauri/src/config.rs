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
    Recovered {
        backup: String,
    },
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
            let (clamped, changed) = clamp_config(config);
            if changed {
                // Valid JSON but out-of-range values — back up the original
                // using the same mechanism as the corrupt-file path.
                let backup: PathBuf = dir.join(format!("{FILE_NAME}.bak"));
                let _ = fs::remove_file(&backup);
                match fs::rename(&path, &backup) {
                    Ok(_) => {}
                    Err(_) => {
                        let _ = fs::copy(&path, &backup);
                        let _ = fs::remove_file(&path);
                    }
                }
                LoadResult {
                    config: clamped,
                    status: LoadStatus::Recovered {
                        backup: backup.to_string_lossy().to_string(),
                    },
                }
            } else {
                LoadResult {
                    config: clamped,
                    status: LoadStatus::Loaded,
                }
            }
        }
        _ => {
            let backup: PathBuf = dir.join(format!("{FILE_NAME}.bak"));
            let _ = fs::remove_file(&backup);
            match fs::rename(&path, &backup) {
                Ok(_) => {}
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

const MAX_TERMINALS: u8 = 6;
const MAX_DIM: u8 = 3;
const MAX_DEPTH: usize = 5;

/// Clamp a parsed config to the project's global constraints.
/// Returns `(clamped_config, was_changed)`.
fn clamp_config(mut config: Config) -> (Config, bool) {
    let mut changed = false;
    config.tree = clamp_nodes(config.tree, 0, &mut changed);
    (config, changed)
}

fn clamp_nodes(nodes: Vec<Node>, depth: usize, changed: &mut bool) -> Vec<Node> {
    if depth >= MAX_DEPTH {
        // Drop nodes at or beyond max depth.
        if !nodes.is_empty() {
            *changed = true;
        }
        return vec![];
    }
    nodes
        .into_iter()
        .map(|node| match node {
            Node::Folder {
                id,
                name,
                expanded,
                children,
            } => {
                let clamped_children = clamp_nodes(children, depth + 1, changed);
                Node::Folder {
                    id,
                    name,
                    expanded,
                    children: clamped_children,
                }
            }
            Node::Workspace {
                id,
                name,
                path,
                ai_tool_id,
                shell_id,
                mut rows,
                mut cols,
                mut row_sizes,
                mut col_sizes,
            } => {
                // Clamp each dimension to [1, MAX_DIM].
                let rows_orig = rows;
                let cols_orig = cols;
                rows = rows.clamp(1, MAX_DIM);
                cols = cols.clamp(1, MAX_DIM);

                // Ensure product <= MAX_TERMINALS; reduce cols first, then rows.
                while rows as u16 * cols as u16 > MAX_TERMINALS as u16 {
                    if cols > 1 {
                        cols -= 1;
                    } else {
                        rows -= 1;
                    }
                }

                if rows != rows_orig || cols != cols_orig {
                    *changed = true;
                }

                // rowSizes must have exactly `rows` entries of equal fractions.
                if row_sizes.len() != rows as usize {
                    *changed = true;
                    let frac = 1.0 / rows as f64;
                    row_sizes = vec![frac; rows as usize];
                }
                // colSizes must have exactly `cols` entries of equal fractions.
                if col_sizes.len() != cols as usize {
                    *changed = true;
                    let frac = 1.0 / cols as f64;
                    col_sizes = vec![frac; cols as usize];
                }

                Node::Workspace {
                    id,
                    name,
                    path,
                    ai_tool_id,
                    shell_id,
                    rows,
                    cols,
                    row_sizes,
                    col_sizes,
                }
            }
        })
        .collect()
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
        let d =
            std::env::temp_dir().join(format!("miniterm-test-{}-{}", name, uuid::Uuid::new_v4()));
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
            id: "w1".into(),
            name: "api".into(),
            path: "/tmp/x".into(),
            ai_tool_id: Some("t1".into()),
            shell_id: None,
            rows: 2,
            cols: 2,
            row_sizes: vec![0.5, 0.5],
            col_sizes: vec![0.5, 0.5],
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
            id: "f1".into(),
            name: "grp".into(),
            expanded: true,
            children: vec![],
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

    // ── clamp_config unit tests ──────────────────────────────────────────────

    fn ws(rows: u8, cols: u8, row_sizes: Vec<f64>, col_sizes: Vec<f64>) -> Node {
        Node::Workspace {
            id: uuid::Uuid::new_v4().to_string(),
            name: "test".into(),
            path: "/tmp".into(),
            ai_tool_id: None,
            shell_id: None,
            rows,
            cols,
            row_sizes,
            col_sizes,
        }
    }

    fn valid_ws() -> Node {
        ws(2, 2, vec![0.5, 0.5], vec![0.5, 0.5])
    }

    #[test]
    fn valid_config_passes_through_unchanged() {
        let mut c = default_config("bash".into());
        c.tree.push(valid_ws());
        let (out, changed) = clamp_config(c.clone());
        assert!(!changed, "valid config must not be flagged as changed");
        assert_eq!(out.tree.len(), 1);
        if let Node::Workspace { rows, cols, .. } = &out.tree[0] {
            assert_eq!(*rows, 2);
            assert_eq!(*cols, 2);
        } else {
            panic!("expected Workspace");
        }
    }

    #[test]
    fn out_of_range_rows_cols_are_clamped() {
        let mut c = default_config("bash".into());
        // rows=0 → clamped to 1; cols=5 → clamped to MAX_DIM=3, but 1*3<=6 so fine
        c.tree.push(ws(0, 5, vec![], vec![]));
        let (out, changed) = clamp_config(c);
        assert!(changed);
        if let Node::Workspace { rows, cols, .. } = &out.tree[0] {
            assert_eq!(*rows, 1);
            assert_eq!(*cols, 3);
        } else {
            panic!("expected Workspace");
        }
    }

    #[test]
    fn product_above_6_is_reduced() {
        let mut c = default_config("bash".into());
        // rows=3, cols=3 → product=9 → must be reduced to <=6
        c.tree.push(ws(3, 3, vec![], vec![]));
        let (out, changed) = clamp_config(c);
        assert!(changed);
        if let Node::Workspace { rows, cols, .. } = &out.tree[0] {
            assert!(
                (*rows as u16) * (*cols as u16) <= 6,
                "product must be <= 6, got {}*{}={}",
                rows,
                cols,
                (*rows as u16) * (*cols as u16)
            );
        } else {
            panic!("expected Workspace");
        }
    }

    #[test]
    fn depth_6_tree_is_pruned_to_5() {
        // Build a chain: folder > folder > folder > folder > folder > workspace
        // depth 0            1        2        3        4        5   (dropped)
        fn nest(depth: usize) -> Vec<Node> {
            if depth == 0 {
                vec![Node::Workspace {
                    id: "leaf".into(),
                    name: "leaf".into(),
                    path: "/tmp".into(),
                    ai_tool_id: None,
                    shell_id: None,
                    rows: 1,
                    cols: 1,
                    row_sizes: vec![1.0],
                    col_sizes: vec![1.0],
                }]
            } else {
                vec![Node::Folder {
                    id: format!("f{depth}"),
                    name: format!("f{depth}"),
                    expanded: true,
                    children: nest(depth - 1),
                }]
            }
        }
        // depth=5 means: folder(0)>folder(1)>folder(2)>folder(3)>folder(4)>workspace(5 — dropped)
        let mut c = default_config("bash".into());
        c.tree = nest(5);
        let (out, changed) = clamp_config(c);
        assert!(changed, "depth-6 tree should be flagged as changed");
        // Walk the output and confirm no workspace survived
        fn has_workspace(nodes: &[Node]) -> bool {
            nodes.iter().any(|n| match n {
                Node::Workspace { .. } => true,
                Node::Folder { children, .. } => has_workspace(children),
            })
        }
        assert!(
            !has_workspace(&out.tree),
            "workspace at depth 5 should have been dropped"
        );
    }

    #[test]
    fn mismatched_sizes_are_regenerated() {
        let mut c = default_config("bash".into());
        // rows=2, cols=3 but sizes vectors are wrong length
        c.tree
            .push(ws(2, 3, vec![1.0], vec![0.25, 0.25, 0.25, 0.25]));
        let (out, changed) = clamp_config(c);
        assert!(changed);
        if let Node::Workspace {
            rows,
            cols,
            row_sizes,
            col_sizes,
            ..
        } = &out.tree[0]
        {
            assert_eq!(*rows, 2);
            assert_eq!(*cols, 3);
            assert_eq!(row_sizes.len(), 2);
            assert_eq!(col_sizes.len(), 3);
            // Each entry should be equal fraction
            for &s in row_sizes {
                assert!((s - 0.5).abs() < 1e-9);
            }
            for &s in col_sizes {
                assert!((s - 1.0 / 3.0).abs() < 1e-9);
            }
        } else {
            panic!("expected Workspace");
        }
    }

    #[test]
    fn clamped_config_triggers_backup_on_load() {
        let d = tmp("clamp-bak");
        // Write a config with rows=3, cols=3 (product=9 > 6)
        let raw = r#"{
            "version": 1,
            "aiTools": [],
            "directories": [],
            "recentDirs": [],
            "defaultShellId": "bash",
            "tree": [{
                "kind": "workspace",
                "id": "w1",
                "name": "big",
                "path": "/tmp",
                "aiToolId": null,
                "shellId": null,
                "rows": 3,
                "cols": 3,
                "rowSizes": [0.333, 0.333, 0.334],
                "colSizes": [0.333, 0.333, 0.334]
            }]
        }"#;
        fs::write(d.join("config.json"), raw).unwrap();
        let r = load(&d, "bash".into());
        assert!(
            matches!(r.status, LoadStatus::Recovered { .. }),
            "clamped config must report Recovered"
        );
        assert!(d.join("config.json.bak").exists());
    }

    #[test]
    fn save_leaves_no_temp_file_behind() {
        let d = tmp("atomic");
        save(&d, &default_config("bash".into())).unwrap();
        let names: Vec<_> = fs::read_dir(&d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["config.json".to_string()]);
    }
}
