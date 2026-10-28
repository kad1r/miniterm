pub mod agent_status;
pub mod agents;
pub mod claude_hooks;
pub mod config;
pub mod pty;
pub mod shell;
pub mod update;

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
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = AppState {
                sessions: pty::SessionManager::new(),
                config_dir: commands::config_dir(&handle),
                downloaded_installer: std::sync::Mutex::new(None),
            };

            let emit_handle = handle.clone();
            state.sessions.set_exit_handler(move |id, code| {
                let _ = emit_handle.emit("session-exit", SessionExit { id, code });
            });

            // No folder means no hook variables in the shells and no watcher:
            // agent status is a nicety, never a reason to refuse to start.
            let status_dir = state.config_dir.join("status");
            let watch = agent_status::reset_dir(&status_dir).is_ok();
            if watch {
                state.sessions.set_status_dir(status_dir.clone());
            }

            app.manage(state);

            if watch {
                let handle = handle.clone();
                std::thread::spawn(move || {
                    let mut watcher = agent_status::Watcher::default();
                    loop {
                        std::thread::sleep(agent_status::TICK);
                        let Some(state) = handle.try_state::<AppState>() else { continue };
                        let ages = state.sessions.output_ages();
                        for update in watcher.tick(&status_dir, &ages) {
                            let _ = handle.emit("agent-status", update);
                        }
                    }
                });
            }
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
            commands::check_update,
            commands::download_update,
            commands::install_update,
            commands::agents_dir,
            commands::write_agent_file,
            commands::rename_agent_file,
            commands::remove_agent_file,
            commands::remove_agents_dir,
            commands::claude_hook_state,
            commands::install_claude_hook,
            commands::uninstall_claude_hook,
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
