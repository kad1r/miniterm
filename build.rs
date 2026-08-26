// Embed the app icon as a Windows resource so Explorer / the taskbar show it
// for the .exe itself. Best-effort: if the active toolchain has no resource
// compiler (e.g. a minimal gnu setup without windres), skip it silently —
// the window icon set at runtime still applies.
fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        let _ = res.compile();
    }
}
