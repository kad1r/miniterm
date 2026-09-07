pub mod config;
pub mod pty;
pub mod shell;

#[cfg(not(test))]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("error while running miniterm");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_runner_runs() {
        assert_eq!(1 + 1, 2);
    }
}
