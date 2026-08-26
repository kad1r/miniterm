//! Resolve the user's configured terminal font at runtime.
//!
//! Priority: Windows Terminal `settings.json` (`profiles.defaults.font.face`),
//! then the legacy console registry (`HKCU\Console\FaceName`), else a bundled
//! Consolas fallback. The chosen face name is resolved to a real `.ttf`/`.otf`
//! path via the Windows font registry (machine + per-user hives).

use std::fs;
use std::path::PathBuf;

/// Resolve font bytes for the terminal. Returns the bytes plus a human label of
/// what was chosen (for logging). When `preferred` is Some(non-empty), that face
/// is tried first; otherwise (or on failure) falls back to auto-detect, then the
/// bundled Consolas.
pub fn resolve_font(bundled: &[u8], preferred: Option<&str>) -> (Vec<u8>, String) {
    if let Some(face) = preferred {
        let face = face.trim();
        if !face.is_empty() {
            if let Some(path) = face_to_path(face) {
                if let Ok(bytes) = fs::read(&path) {
                    return (bytes, format!("{face} ({}) [config]", path.display()));
                }
            }
        }
    }
    if let Some(face) = detect_face() {
        if let Some(path) = face_to_path(&face) {
            if let Ok(bytes) = fs::read(&path) {
                return (bytes, format!("{face} ({})", path.display()));
            }
        }
        return (bundled.to_vec(), format!("{face} -> unresolved, bundled Consolas"));
    }
    (bundled.to_vec(), "Consolas (bundled default)".to_string())
}

/// Pick the configured face name: Windows Terminal first, then console registry.
fn detect_face() -> Option<String> {
    for path in wt_settings_paths() {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Some(face) = parse_wt_face(&text) {
                return Some(face);
            }
        }
    }
    console_face()
}

/// Candidate Windows Terminal settings.json locations (stable, preview, unpackaged).
fn wt_settings_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let base = PathBuf::from(&local);
        out.push(base.join(
            "Packages\\Microsoft.WindowsTerminal_8wekyb3d8bbwe\\LocalState\\settings.json",
        ));
        out.push(base.join(
            "Packages\\Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe\\LocalState\\settings.json",
        ));
        out.push(base.join("Microsoft\\Windows Terminal\\settings.json"));
    }
    out
}

/// Extract `profiles.defaults.font.face` (or legacy `fontFace`) from a Windows
/// Terminal settings.json, tolerating JSONC comments and trailing commas.
pub fn parse_wt_face(json: &str) -> Option<String> {
    let cleaned = strip_jsonc(json);
    let v: serde_json::Value = serde_json::from_str(&cleaned).ok()?;
    let defaults = v.get("profiles")?.get("defaults")?;
    // Modern: font.face. Legacy: fontFace.
    if let Some(face) = defaults.get("font").and_then(|f| f.get("face")).and_then(|s| s.as_str()) {
        if !face.trim().is_empty() {
            return Some(face.trim().to_string());
        }
    }
    if let Some(face) = defaults.get("fontFace").and_then(|s| s.as_str()) {
        if !face.trim().is_empty() {
            return Some(face.trim().to_string());
        }
    }
    None
}

/// Strip `//` line comments and `/* */` block comments while respecting string
/// literals, then drop trailing commas so serde_json can parse WT's JSONC.
fn strip_jsonc(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut in_str = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            out.push(c as char);
            if c == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_str = true;
                out.push('"');
                i += 1;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                // line comment
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                // block comment
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            _ => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    strip_trailing_commas(&out)
}

/// Remove commas that immediately precede a closing `}` or `]` (ignoring
/// whitespace), respecting string literals.
fn strip_trailing_commas(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            out.push(c as char);
            if c == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if c == b'"' {
            in_str = true;
            out.push('"');
            i += 1;
            continue;
        }
        if c == b',' {
            // look ahead past whitespace
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b'}' || bytes[j] == b']') {
                // skip the comma
                i += 1;
                continue;
            }
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Read the legacy console face name from `HKCU\Console\FaceName`.
fn console_face() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let console = hkcu.open_subkey("Console").ok()?;
    let face: String = console.get_value("FaceName").ok()?;
    let face = face.trim();
    if face.is_empty() || face == "__DefaultTTFont__" {
        return None;
    }
    Some(face.to_string())
}

/// Resolve a face name to a font file path via the Windows font registry.
fn face_to_path(face: &str) -> Option<PathBuf> {
    let entries = enumerate_font_registry();
    let file = match_font_file(&entries, face)?;
    Some(PathBuf::from(file))
}

/// Enumerate installed fonts from both machine and per-user hives, resolving
/// each entry to a full path. Returns (registryValueName, fullPath).
fn enumerate_font_registry() -> Vec<(String, String)> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let mut out = Vec::new();
    const SUBKEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";

    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let machine_base = PathBuf::from(windir).join("Fonts");
    let user_base = std::env::var("LOCALAPPDATA")
        .ok()
        .map(|l| PathBuf::from(l).join("Microsoft\\Windows\\Fonts"));

    // Machine fonts: file names are relative to %WINDIR%\Fonts.
    if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(SUBKEY) {
        for (name, val) in k.enum_values().flatten() {
            let file = val.to_string();
            let full = resolve_font_file(&file, &machine_base);
            out.push((name, full));
        }
    }
    // Per-user fonts: data is typically already a full path.
    if let (Ok(k), Some(base)) = (
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(SUBKEY),
        user_base.as_ref(),
    ) {
        for (name, val) in k.enum_values().flatten() {
            let file = val.to_string();
            let full = resolve_font_file(&file, base);
            out.push((name, full));
        }
    }
    out
}

fn resolve_font_file(file: &str, base: &std::path::Path) -> String {
    let f = file.trim();
    // Absolute if it has a drive letter or UNC prefix.
    if f.len() >= 2 && &f[1..2] == ":" || f.starts_with("\\\\") {
        f.to_string()
    } else {
        base.join(f).to_string_lossy().into_owned()
    }
}

/// Given font-registry entries (valueName, path) and a desired face, choose the
/// best-matching file path (prefers the regular/upright weight).
pub fn match_font_file(entries: &[(String, String)], face: &str) -> Option<String> {
    let face_l = face.to_lowercase();
    let mut candidates: Vec<(&str, &str)> = entries
        .iter()
        .map(|(n, f)| (strip_type_suffix(n), f.as_str()))
        .filter(|(n, _)| {
            let nl = n.to_lowercase();
            nl == face_l || nl.starts_with(&format!("{face_l} "))
        })
        .collect();
    candidates.sort_by_key(|(n, _)| style_rank(n, &face_l));
    candidates.first().map(|(_, f)| f.to_string())
}

/// Strip a trailing " (TrueType)" / " (OpenType)" / " (All Res)" suffix.
fn strip_type_suffix(name: &str) -> &str {
    for suffix in [" (TrueType)", " (OpenType)", " (All Res)"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            return stripped;
        }
    }
    name
}

/// Lower rank sorts first: exact face and Regular win over Bold/Italic/etc.
fn style_rank(name: &str, face_l: &str) -> u8 {
    let nl = name.to_lowercase();
    if nl == *face_l {
        return 0;
    }
    if nl.contains("regular") {
        return 1;
    }
    for style in [
        "bold", "italic", "oblique", "light", "thin", "medium", "semibold",
        "semi-bold", "black", "heavy", "extrabold", "extralight", "condensed",
    ] {
        if nl.contains(style) {
            return 3;
        }
    }
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_font_face_from_jsonc() {
        let json = r#"
        {
            // user comment
            "profiles": {
                "defaults": {
                    "font": { "face": "JetBrains Mono", "size": 11 },
                }, /* trailing comma above */
            }
        }
        "#;
        assert_eq!(parse_wt_face(json).as_deref(), Some("JetBrains Mono"));
    }

    #[test]
    fn parses_legacy_font_face() {
        let json = r#"{ "profiles": { "defaults": { "fontFace": "Cascadia Code" } } }"#;
        assert_eq!(parse_wt_face(json).as_deref(), Some("Cascadia Code"));
    }

    #[test]
    fn missing_face_returns_none() {
        let json = r#"{ "profiles": { "defaults": { "colorScheme": "Campbell" } } }"#;
        assert_eq!(parse_wt_face(json), None);
    }

    #[test]
    fn comment_marker_inside_string_is_preserved() {
        let json = r#"{ "profiles": { "defaults": { "icon": "ms-appx://Logo.png",
            "font": { "face": "Consolas" } } } }"#;
        assert_eq!(parse_wt_face(json).as_deref(), Some("Consolas"));
    }

    #[test]
    fn matches_regular_weight_over_bold() {
        let entries = vec![
            (
                "JetBrains Mono Bold (TrueType)".to_string(),
                "C:\\Fonts\\JetBrainsMono-Bold.ttf".to_string(),
            ),
            (
                "JetBrains Mono Regular (TrueType)".to_string(),
                "C:\\Fonts\\JetBrainsMono-Regular.ttf".to_string(),
            ),
        ];
        assert_eq!(
            match_font_file(&entries, "JetBrains Mono").as_deref(),
            Some("C:\\Fonts\\JetBrainsMono-Regular.ttf")
        );
    }

    #[test]
    fn exact_face_name_wins() {
        let entries = vec![
            (
                "Consolas (TrueType)".to_string(),
                "C:\\Windows\\Fonts\\consola.ttf".to_string(),
            ),
            (
                "Consolas Bold (TrueType)".to_string(),
                "C:\\Windows\\Fonts\\consolab.ttf".to_string(),
            ),
        ];
        assert_eq!(
            match_font_file(&entries, "Consolas").as_deref(),
            Some("C:\\Windows\\Fonts\\consola.ttf")
        );
    }

    #[test]
    fn no_match_returns_none() {
        let entries = vec![(
            "Arial (TrueType)".to_string(),
            "C:\\Windows\\Fonts\\arial.ttf".to_string(),
        )];
        assert_eq!(match_font_file(&entries, "JetBrains Mono"), None);
    }
}
