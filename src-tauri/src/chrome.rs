//! Native title bar tinting.
//!
//! The window keeps its system frame (snap layouts, drag, double-click maximise
//! all stay native); only the colours DWM paints it with are replaced, so the
//! caption reads as part of the app instead of a stock Windows bar.

/// `#rrggbb` → the `0x00bbggrr` COLORREF layout DWM expects.
pub fn colorref(hex: &str) -> Option<u32> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let rgb = u32::from_str_radix(digits, 16).ok()?;
    Some(((rgb & 0xff) << 16) | (rgb & 0xff00) | (rgb >> 16))
}

#[cfg(windows)]
mod dwm {
    use std::ffi::c_void;

    // Windows 11 (build 22000+). Older builds answer E_INVALIDARG and keep the
    // stock caption, which is the right fallback.
    pub const BORDER_COLOR: u32 = 34;
    pub const CAPTION_COLOR: u32 = 35;
    pub const TEXT_COLOR: u32 = 36;

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(hwnd: *mut c_void, attr: u32, value: *const c_void, size: u32) -> i32;
    }

    pub fn set_color(hwnd: *mut c_void, attr: u32, color: u32) -> Result<(), String> {
        // SAFETY: `color` outlives the call and `size` matches its type; DWM
        // validates the handle and returns a failure HRESULT for a stale one.
        let hr = unsafe {
            DwmSetWindowAttribute(hwnd, attr, (&color as *const u32).cast(), std::mem::size_of::<u32>() as u32)
        };
        if hr < 0 {
            return Err(format!("DwmSetWindowAttribute({attr}) failed: {hr:#010x}"));
        }
        Ok(())
    }
}

/// Paint the caption, its text and the 1px frame of `hwnd` in the given colours.
#[cfg(windows)]
pub fn apply(hwnd: *mut std::ffi::c_void, caption: &str, text: &str, border: &str) -> Result<(), String> {
    let parse = |hex: &str| colorref(hex).ok_or_else(|| format!("not a #rrggbb colour: {hex}"));
    let (caption, text, border) = (parse(caption)?, parse(text)?, parse(border)?);
    dwm::set_color(hwnd, dwm::CAPTION_COLOR, caption)?;
    dwm::set_color(hwnd, dwm::TEXT_COLOR, text)?;
    dwm::set_color(hwnd, dwm::BORDER_COLOR, border)
}

#[cfg(test)]
mod tests {
    use super::colorref;

    #[test]
    fn swaps_red_and_blue() {
        assert_eq!(colorref("#0e1116"), Some(0x0016110e));
        assert_eq!(colorref("#FF0000"), Some(0x000000ff));
        assert_eq!(colorref("#0000ff"), Some(0x00ff0000));
    }

    #[test]
    fn rejects_anything_but_six_hex_digits() {
        for bad in ["", "#fff", "0e1116", "#0e11161", "#gg0000", "#+e1116", "rgb(0,0,0)"] {
            assert_eq!(colorref(bad), None, "{bad}");
        }
    }
}
