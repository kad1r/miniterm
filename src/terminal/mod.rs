pub mod pty;
pub mod session;

/// Pick the default shell to spawn inside a pane.
///
/// Windows Terminal (`wt.exe`) is a terminal *host*, not a shell, so it cannot
/// be embedded as a ConPTY child. Instead we prefer PowerShell 7 (`pwsh.exe`),
/// then Windows PowerShell (`powershell.exe`), and fall back to `cmd.exe`.
pub fn default_shell() -> String {
    for cand in ["pwsh.exe", "powershell.exe"] {
        if exists_in_path(cand) {
            return cand.to_string();
        }
    }
    "cmd.exe".to_string()
}

/// True if `exe` is found in any `PATH` directory.
fn exists_in_path(exe: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(exe).is_file())
}
