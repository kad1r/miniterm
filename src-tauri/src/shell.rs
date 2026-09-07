use serde::Serialize;

pub struct ShellCandidate {
    pub id: &'static str,
    pub name: &'static str,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShellInfo {
    pub id: String,
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
}

/// Windows adayları, tercih sırasıyla.
///
/// `wt.exe` kasıtlı olarak yoktur: Windows Terminal bir shell değil, shell
/// barındıran bir pencere uygulamasıdır. Alt süreç olarak başlatılırsa kendi
/// penceresini açar ve miniterm'in grid'inde hiç görünmez.
pub fn windows_candidates() -> Vec<ShellCandidate> {
    let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());

    vec![
        ShellCandidate {
            id: "pwsh",
            name: "PowerShell 7",
            program: format!(r"{program_files}\PowerShell\7\pwsh.exe"),
            args: vec!["-NoLogo".into()],
        },
        ShellCandidate {
            id: "powershell",
            name: "Windows PowerShell",
            program: format!(r"{system_root}\System32\WindowsPowerShell\v1.0\powershell.exe"),
            args: vec!["-NoLogo".into()],
        },
        ShellCandidate {
            id: "cmd",
            name: "Command Prompt",
            program: format!(r"{system_root}\System32\cmd.exe"),
            args: vec![],
        },
        ShellCandidate {
            id: "gitbash",
            name: "Git Bash",
            program: format!(r"{program_files}\Git\bin\bash.exe"),
            args: vec!["-i".into()],
        },
        ShellCandidate {
            id: "wsl",
            name: "WSL",
            program: format!(r"{system_root}\System32\wsl.exe"),
            args: vec![],
        },
    ]
}

/// Unix adayları. `$SHELL` varsa başa alınır, ardından `/etc/shells`.
pub fn unix_candidates(shell_env: Option<&str>, etc_shells: &str) -> Vec<ShellCandidate> {
    let mut programs: Vec<String> = Vec::new();

    if let Some(s) = shell_env.map(str::trim).filter(|s| !s.is_empty()) {
        programs.push(s.to_string());
    }
    for line in etc_shells.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !programs.iter().any(|p| p == line) {
            programs.push(line.to_string());
        }
    }

    programs
        .into_iter()
        .map(|program| {
            let leaked: &'static str = Box::leak(
                program.rsplit('/').next().unwrap_or("shell").to_string().into_boxed_str(),
            );
            ShellCandidate { id: leaked, name: leaked, program, args: vec!["-i".into()] }
        })
        .collect()
}

pub fn resolve(candidates: Vec<ShellCandidate>, exists: &dyn Fn(&str) -> bool) -> Vec<ShellInfo> {
    candidates
        .into_iter()
        .filter(|c| exists(&c.program))
        .map(|c| ShellInfo {
            id: c.id.to_string(),
            name: c.name.to_string(),
            program: c.program,
            args: c.args,
        })
        .collect()
}

pub fn detect() -> Vec<ShellInfo> {
    let exists = |p: &str| std::path::Path::new(p).exists();

    #[cfg(windows)]
    {
        resolve(windows_candidates(), &exists)
    }

    #[cfg(not(windows))]
    {
        let shell_env = std::env::var("SHELL").ok();
        let etc = std::fs::read_to_string("/etc/shells").unwrap_or_default();
        resolve(unix_candidates(shell_env.as_deref(), &etc), &exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_order_prefers_pwsh_then_powershell_then_cmd() {
        let ids: Vec<_> = windows_candidates().iter().map(|c| c.id).collect();
        let pwsh = ids.iter().position(|i| *i == "pwsh").unwrap();
        let ps5 = ids.iter().position(|i| *i == "powershell").unwrap();
        let cmd = ids.iter().position(|i| *i == "cmd").unwrap();
        assert!(pwsh < ps5 && ps5 < cmd);
    }

    #[test]
    fn windows_candidates_never_include_windows_terminal() {
        for c in windows_candidates() {
            assert!(!c.program.to_lowercase().contains("wt.exe"), "wt.exe leaked in as {}", c.id);
            assert_ne!(c.id, "wt");
        }
    }

    #[test]
    fn resolve_drops_candidates_that_do_not_exist() {
        let cands = vec![
            ShellCandidate { id: "a", name: "A", program: "a.exe".into(), args: vec![] },
            ShellCandidate { id: "b", name: "B", program: "b.exe".into(), args: vec![] },
        ];
        let found = resolve(cands, &|p| p == "b.exe");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "b");
    }

    #[test]
    fn unix_puts_the_shell_env_value_first_and_deduplicates() {
        let etc = "/bin/sh\n# a comment\n/bin/bash\n/bin/zsh\n";
        let cands = unix_candidates(Some("/bin/zsh"), etc);
        assert_eq!(cands[0].program, "/bin/zsh");
        let zsh_count = cands.iter().filter(|c| c.program == "/bin/zsh").count();
        assert_eq!(zsh_count, 1);
    }

    #[test]
    fn unix_ignores_comments_and_blank_lines() {
        let cands = unix_candidates(None, "\n# comment\n/bin/bash\n\n");
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].program, "/bin/bash");
    }
}
