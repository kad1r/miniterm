use serde::Deserialize;

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
