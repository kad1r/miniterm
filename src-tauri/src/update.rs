use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// Strip a single leading `v`/`V` from a release tag.
pub fn strip_v(tag: &str) -> &str {
    tag.strip_prefix('v').or_else(|| tag.strip_prefix('V')).unwrap_or(tag)
}

fn parts(v: &str) -> [u64; 3] {
    let v = strip_v(v.trim());
    let mut out = [0u64; 3];
    for (i, seg) in v.split('.').take(3).enumerate() {
        // Stop at the first non-digit so "1.0.0-rc1" degrades to 1.0.0.
        let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
        out[i] = digits.parse().unwrap_or(0);
    }
    out
}

/// True when `latest` is a strictly higher semver than `current`.
pub fn is_newer(current: &str, latest: &str) -> bool {
    parts(latest) > parts(current)
}

/// The NSIS installer asset, if the release carries one.
pub fn pick_setup_asset(assets: &[Asset]) -> Option<&Asset> {
    assets.iter().find(|a| a.name.ends_with("_x64-setup.exe"))
}

/// Only GitHub-owned hosts may be downloaded or launched.
pub fn is_allowed_host(url: &str) -> bool {
    match reqwest::Url::parse(url) {
        Ok(u) => match u.host_str() {
            Some(h) => h == "github.com" || h.ends_with(".githubusercontent.com"),
            None => false,
        },
        Err(_) => false,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub is_newer: bool,
    pub download_url: String,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Stream `url` to a temp file, emitting `update-progress` as bytes arrive.
/// Rejects any non-GitHub host before opening a connection.
pub async fn download(app: &AppHandle, url: &str) -> Result<std::path::PathBuf, String> {
    if !is_allowed_host(url) {
        return Err("refusing to download a non-GitHub url".into());
    }
    let file_name = url.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or("miniterm-setup.exe");
    let dest = std::env::temp_dir().join(file_name);

    let client = reqwest::Client::builder()
        .user_agent("miniterm-updater")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("download failed: {}", resp.status()));
    }
    let total = resp.content_length();
    let mut file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        let _ = app.emit("update-progress", Progress { downloaded, total });
    }
    file.flush().map_err(|e| e.to_string())?;
    Ok(dest)
}

const LATEST_URL: &str = "https://api.github.com/repos/kad1r/miniterm/releases/latest";

/// Query GitHub for the latest release and fold it against `current`.
/// `Err` on any network/parse failure or when the release has no NSIS asset.
pub async fn fetch_latest(current: &str) -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("miniterm/{current}"))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("github returned {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let rel: Release = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let latest = strip_v(&rel.tag_name).to_string();
    let asset = pick_setup_asset(&rel.assets).ok_or("no NSIS installer asset on latest release")?;
    Ok(UpdateInfo {
        current: current.to_string(),
        latest: latest.clone(),
        is_newer: is_newer(current, &latest),
        download_url: asset.browser_download_url.clone(),
        notes: rel.body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> Asset {
        Asset { name: name.into(), browser_download_url: format!("https://github.com/x/{name}") }
    }

    #[test]
    fn strip_v_removes_leading_v() {
        assert_eq!(strip_v("v0.9.0"), "0.9.0");
        assert_eq!(strip_v("0.9.0"), "0.9.0");
    }

    #[test]
    fn is_newer_compares_semver() {
        assert!(is_newer("0.9.0", "0.10.0"));
        assert!(is_newer("0.9.0", "1.0.0"));
        assert!(is_newer("0.9.0", "0.9.1"));
        assert!(!is_newer("0.9.0", "0.9.0"));
        assert!(!is_newer("0.9.1", "0.9.0"));
        assert!(!is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn is_newer_tolerates_v_prefix_and_short_parts() {
        assert!(is_newer("v0.9.0", "v0.9.1"));
        assert!(is_newer("0.9", "0.9.1"));
        assert!(!is_newer("0.9.0", "0.9"));
    }

    #[test]
    fn pick_setup_asset_finds_nsis_exe() {
        let assets = vec![
            asset("miniterm_0.9.0_x64_en-US.msi"),
            asset("miniterm_0.9.0_x64-setup.exe"),
        ];
        assert_eq!(pick_setup_asset(&assets).unwrap().name, "miniterm_0.9.0_x64-setup.exe");
    }

    #[test]
    fn pick_setup_asset_none_when_absent() {
        let assets = vec![asset("miniterm_0.9.0_x64_en-US.msi")];
        assert!(pick_setup_asset(&assets).is_none());
    }

    #[test]
    fn host_validation_accepts_github_only() {
        assert!(is_allowed_host("https://github.com/kad1r/miniterm/releases/download/v1/x.exe"));
        assert!(is_allowed_host("https://objects.githubusercontent.com/foo/x.exe"));
        assert!(!is_allowed_host("https://evil.com/x.exe"));
        assert!(!is_allowed_host("https://github.com.evil.com/x.exe"));
        assert!(!is_allowed_host("not a url"));
    }
}
