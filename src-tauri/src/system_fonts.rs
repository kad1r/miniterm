//! Installed font families, for the sidebar font picker's "system font" option.
//!
//! On Windows the registry's font tables are the cheap source: one value per
//! installed face, machine-wide under HKLM and per-user under HKCU. Value names
//! are face names ("Segoe UI Semibold Italic (TrueType)"), so the family is
//! recovered by dropping the format suffix and trailing style words. Elsewhere
//! the list is empty and the picker falls back to a typed name.

/// Words that name a face's style rather than its family.
const STYLE_WORDS: [&str; 33] = [
    "regular", "demi", "normal", "roman", "book", "bold", "italic", "oblique", "light", "thin",
    "medium", "semibold", "semilight", "demibold", "demilight", "black", "heavy",
    "extrabold", "extralight", "ultrabold", "ultralight", "extrablack", "hairline",
    "condensed", "semicondensed", "extracondensed", "cond", "narrow", "extended",
    "expanded", "semiexpanded", "bolditalic", "lightitalic",
];

/// Families named by one registry value; `&` joins several in a collection.
pub fn families_of(value_name: &str) -> Vec<String> {
    let name = value_name.trim();
    let name = match name.rfind(" (") {
        Some(i) if name.ends_with(')') => {
            let kind = &name[i + 2..name.len() - 1];
            // Raster fonts ("Modern (All res)", "Courier 10,12,15") are not
            // usable in a web view.
            if !kind.eq_ignore_ascii_case("TrueType") && !kind.eq_ignore_ascii_case("OpenType") {
                return Vec::new();
            }
            &name[..i]
        }
        _ => name,
    };
    if name.contains(',') {
        return Vec::new();
    }
    name.split(" & ").filter_map(strip_style).collect()
}

fn strip_style(face: &str) -> Option<String> {
    let mut words: Vec<&str> = face.split_whitespace().collect();
    while words.len() > 1 {
        let last = words[words.len() - 1].to_ascii_lowercase();
        if STYLE_WORDS.contains(&last.as_str()) {
            words.pop();
        } else {
            break;
        }
    }
    let family = words.join(" ");
    (!family.is_empty() && !family.starts_with('@')).then_some(family)
}

/// Sorted, case-insensitively unique family names.
pub fn normalize(value_names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = value_names.into_iter().flat_map(|v| families_of(&v)).collect();
    out.sort_by_key(|f| f.to_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    out
}

#[cfg(windows)]
mod registry {
    use std::ffi::c_void;

    type Hkey = *mut c_void;
    const HKEY_CURRENT_USER: Hkey = 0x8000_0001_u32 as i32 as isize as Hkey;
    const HKEY_LOCAL_MACHINE: Hkey = 0x8000_0002_u32 as i32 as isize as Hkey;
    const KEY_READ: u32 = 0x20019;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_MORE_DATA: i32 = 234;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(key: Hkey, sub: *const u16, opts: u32, sam: u32, out: *mut Hkey) -> i32;
        fn RegEnumValueW(
            key: Hkey,
            index: u32,
            name: *mut u16,
            name_len: *mut u32,
            reserved: *mut u32,
            kind: *mut u32,
            data: *mut u8,
            data_len: *mut u32,
        ) -> i32;
        fn RegCloseKey(key: Hkey) -> i32;
    }

    const FONTS: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";

    fn value_names(root: Hkey, out: &mut Vec<String>) {
        let sub: Vec<u16> = FONTS.encode_utf16().chain(Some(0)).collect();
        let mut key: Hkey = std::ptr::null_mut();
        // SAFETY: valid NUL-terminated path and out-pointer; key closed below.
        if unsafe { RegOpenKeyExW(root, sub.as_ptr(), 0, KEY_READ, &mut key) } != ERROR_SUCCESS {
            return;
        }
        let mut buf = vec![0u16; 16_384];
        for index in 0.. {
            let mut len = buf.len() as u32;
            let none32: *mut u32 = std::ptr::null_mut();
            let none8: *mut u8 = std::ptr::null_mut();
            // SAFETY: buf holds `len` u16s; the data out-params are null, which
            // the API allows when only the name is wanted.
            let rc = unsafe {
                RegEnumValueW(key, index, buf.as_mut_ptr(), &mut len, none32, none32, none8, none32)
            };
            match rc {
                ERROR_SUCCESS => out.push(String::from_utf16_lossy(&buf[..len as usize])),
                ERROR_MORE_DATA => continue,
                _ => break,
            }
        }
        // SAFETY: key was opened above.
        unsafe { RegCloseKey(key) };
    }

    pub fn all() -> Vec<String> {
        let mut out = Vec::new();
        value_names(HKEY_LOCAL_MACHINE, &mut out);
        value_names(HKEY_CURRENT_USER, &mut out);
        out
    }
}

/// Families installed on this machine, sorted. Empty off Windows.
pub fn list() -> Vec<String> {
    #[cfg(windows)]
    {
        normalize(registry::all())
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_words_and_the_format_suffix_come_off() {
        assert_eq!(families_of("Segoe UI Semibold Italic (TrueType)"), vec!["Segoe UI"]);
        assert_eq!(families_of("Cascadia Code (TrueType)"), vec!["Cascadia Code"]);
        assert_eq!(families_of("Arial Nova Light (OpenType)"), vec!["Arial Nova"]);
        assert_eq!(families_of("Bahnschrift"), vec!["Bahnschrift"]);
        assert_eq!(families_of("Berlin Sans FB Demi Bold (TrueType)"), vec!["Berlin Sans FB"]);
    }

    #[test]
    fn collections_name_every_family() {
        assert_eq!(
            families_of("Cambria & Cambria Math (TrueType)"),
            vec!["Cambria", "Cambria Math"]
        );
    }

    #[test]
    fn raster_fonts_are_dropped() {
        assert!(families_of("Modern (All res)").is_empty());
        assert!(families_of("Courier 10,12,15").is_empty());
    }

    #[test]
    fn a_family_that_is_only_a_style_word_survives() {
        assert_eq!(families_of("Black (TrueType)"), vec!["Black"]);
    }

    #[test]
    fn normalize_sorts_and_dedupes_ignoring_case() {
        let names = ["Segoe UI (TrueType)", "segoe ui Bold (TrueType)", "Arial (TrueType)"];
        assert_eq!(normalize(names.map(String::from)), vec!["Arial", "Segoe UI"]);
    }

    #[cfg(windows)]
    #[test]
    fn this_machine_has_some_fonts() {
        assert!(!list().is_empty());
    }
}
