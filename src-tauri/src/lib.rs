pub mod config;
pub mod pty;
pub mod shell;

#[cfg(not(test))]
pub mod commands;

#[cfg(not(test))]
use commands::AppState;
#[cfg(not(test))]
use serde::Serialize;
#[cfg(not(test))]
use tauri::{Emitter, Manager};

#[cfg(not(test))]
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionExit {
    id: u64,
    code: Option<i32>,
}

#[cfg(not(test))]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = AppState {
                sessions: pty::SessionManager::new(),
                config_dir: commands::config_dir(&handle),
            };

            let emit_handle = handle.clone();
            state.sessions.set_exit_handler(move |id, code| {
                let _ = emit_handle.emit("session-exit", SessionExit { id, code });
            });

            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    state.sessions.shutdown_all();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::detect_shells,
            commands::spawn_session,
            commands::write_session,
            commands::resize_session,
            commands::kill_session,
            commands::attach_session,
            commands::detach_session,
            commands::get_buffer,
        ])
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
