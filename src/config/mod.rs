//! Startup configuration: theme colors + font, loaded once from
//! `%APPDATA%\miniterm\config.toml`. Every failure path falls back to
//! `Config::default()` (today's hardcoded look) and logs one stderr line.

use serde::Deserialize;

/// RGB in 0..1, ready for the render pipeline (each channel = byte / 255.0).
pub type Rgb = [f32; 3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub background: Rgb,
    pub foreground: Rgb,
    pub cursor: Rgb,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: [0.05, 0.05, 0.06],
            foreground: [0.85, 0.85, 0.85],
            cursor: [0.85, 0.85, 0.85],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontConfig {
    pub family: Option<String>,
    pub size: f32,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig { family: None, size: 18.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub font: FontConfig,
    pub theme: Theme,
}

// --- Wire (serde) types: hex strings + optional fields deserialize cleanly ---

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RawConfig {
    font: RawFont,
    colors: RawColors,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawFont {
    family: Option<String>,
    size: f32,
}
impl Default for RawFont {
    fn default() -> Self { RawFont { family: None, size: 18.0 } }
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RawColors {
    background: Option<String>,
    foreground: Option<String>,
    cursor: Option<String>,
}

impl RawConfig {
    fn into_config(self) -> Config {
        let d = Theme::default();
        let theme = Theme {
            background: self.colors.background.as_deref().and_then(parse_hex).unwrap_or(d.background),
            foreground: self.colors.foreground.as_deref().and_then(parse_hex).unwrap_or(d.foreground),
            cursor: self.colors.cursor.as_deref().and_then(parse_hex).unwrap_or(d.cursor),
        };
        let font = FontConfig { family: self.font.family, size: self.font.size };
        Config { font, theme }
    }
}

/// Parse "#rrggbb" (case-insensitive, leading '#' optional) into Rgb.
/// Returns None on wrong length or a non-hex character.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

/// Parse a TOML string into a Config; any parse error yields Config::default().
/// Test/helper seam for `load` (which adds the file IO around this).
pub fn parse_str(s: &str) -> Config {
    match toml::from_str::<RawConfig>(s) {
        Ok(raw) => raw.into_config(),
        Err(_) => Config::default(),
    }
}

/// `%APPDATA%\miniterm\config.toml`, or None if APPDATA is unset.
pub fn config_path() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA")
        .map(|a| std::path::PathBuf::from(a).join("miniterm").join("config.toml"))
}

/// Read + parse the config. Any failure (no APPDATA, missing/unreadable file,
/// TOML parse error) yields Config::default() and a single stderr log line.
pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("[miniterm] config: none at {}, using defaults", path.display());
            return Config::default();
        }
    };
    match toml::from_str::<RawConfig>(&text) {
        Ok(raw) => raw.into_config(),
        Err(e) => {
            eprintln!("[miniterm] config: parse error ({e}), using defaults");
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_basic() {
        assert_eq!(parse_hex("#0d0d10"), Some([13.0 / 255.0, 13.0 / 255.0, 16.0 / 255.0]));
    }

    #[test]
    fn parse_hex_no_prefix_and_uppercase() {
        assert_eq!(parse_hex("0D0D10"), Some([13.0 / 255.0, 13.0 / 255.0, 16.0 / 255.0]));
    }

    #[test]
    fn parse_hex_rejects_bad_input() {
        assert_eq!(parse_hex("#fff"), None);      // wrong length
        assert_eq!(parse_hex("#12345g"), None);   // non-hex char
        assert_eq!(parse_hex(""), None);
    }

    #[test]
    fn empty_toml_is_all_defaults() {
        assert_eq!(parse_str(""), Config::default());
    }

    #[test]
    fn full_config_maps_all_fields() {
        let toml = "[font]\nfamily = \"JetBrains Mono\"\nsize = 20.0\n[colors]\nbackground = \"#101014\"\nforeground = \"#c0c0c0\"\ncursor = \"#ff8800\"\n";
        let c = parse_str(toml);
        assert_eq!(c.font.family.as_deref(), Some("JetBrains Mono"));
        assert_eq!(c.font.size, 20.0);
        assert_eq!(c.theme.background, parse_hex("#101014").unwrap());
        assert_eq!(c.theme.foreground, parse_hex("#c0c0c0").unwrap());
        assert_eq!(c.theme.cursor, parse_hex("#ff8800").unwrap());
    }

    #[test]
    fn partial_colors_keep_defaults() {
        let c = parse_str("[colors]\nbackground = \"#101014\"\n");
        let d = Theme::default();
        assert_eq!(c.theme.background, parse_hex("#101014").unwrap());
        assert_eq!(c.theme.foreground, d.foreground);
        assert_eq!(c.theme.cursor, d.cursor);
    }

    #[test]
    fn bad_hex_on_one_field_defaults_only_that_field() {
        let c = parse_str("[colors]\nbackground = \"nothex\"\nforeground = \"#c0c0c0\"\n");
        let d = Theme::default();
        assert_eq!(c.theme.background, d.background); // bad hex -> default
        assert_eq!(c.theme.foreground, parse_hex("#c0c0c0").unwrap());
    }

    #[test]
    fn unknown_field_yields_default() {
        // deny_unknown_fields makes this a parse error -> default.
        let c = parse_str("[font]\nbogus = 1\n");
        assert_eq!(c, Config::default());
    }

    #[test]
    fn font_size_default_when_font_section_omitted() {
        let c = parse_str("[colors]\ncursor = \"#ff8800\"\n");
        assert_eq!(c.font.size, 18.0);
        assert_eq!(c.font.family, None);
    }

    #[test]
    fn config_path_shape_when_appdata_set() {
        std::env::set_var("APPDATA", "C:\\Users\\test\\AppData\\Roaming");
        let p = config_path().unwrap();
        assert!(p.ends_with("miniterm\\config.toml") || p.ends_with("miniterm/config.toml"));
    }
}
