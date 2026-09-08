use crate::config::{self, Config, LoadResult};
use crate::pty::{SessionManager, SpawnOpts};
use crate::shell::{self, ShellInfo};
use std::path::PathBuf;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub sessions: SessionManager,
    pub config_dir: PathBuf,
}

fn map_err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> LoadResult {
    let shells = shell::detect();
    let default_shell_id = shells.first().map(|s| s.id.clone()).unwrap_or_default();
    config::load(&state.config_dir, default_shell_id)
}

#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> Result<(), String> {
    config::save(&state.config_dir, &config).map_err(map_err)
}

#[tauri::command]
pub fn detect_shells() -> Vec<ShellInfo> {
    shell::detect()
}

#[tauri::command]
pub fn spawn_session(state: State<'_, AppState>, opts: SpawnOpts) -> Result<u64, String> {
    state.sessions.spawn(opts).map_err(map_err)
}

#[tauri::command]
pub fn write_session(state: State<'_, AppState>, id: u64, data: Vec<u8>) -> Result<(), String> {
    state.sessions.write(id, &data).map_err(map_err)
}

#[tauri::command]
pub fn resize_session(
    state: State<'_, AppState>,
    id: u64,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    state.sessions.resize(id, cols, rows).map_err(map_err)
}

#[tauri::command]
pub fn kill_session(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    state.sessions.kill(id).map_err(map_err)
}

/// Raw byte stream. `InvokeResponseBody::Raw` is used — no JSON encoding per chunk.
#[tauri::command]
pub fn attach_session(
    state: State<'_, AppState>,
    id: u64,
    channel: Channel<InvokeResponseBody>,
) -> Result<(), String> {
    let sink = std::sync::Arc::new(move |bytes: &[u8]| {
        let _ = channel.send(InvokeResponseBody::Raw(bytes.to_vec()));
    });
    state.sessions.attach(id, sink).map_err(map_err)
}

#[tauri::command]
pub fn detach_session(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    state.sessions.detach(id).map_err(map_err)
}

#[tauri::command]
pub fn get_buffer(state: State<'_, AppState>, id: u64) -> Result<tauri::ipc::Response, String> {
    let bytes = state.sessions.snapshot(id).map_err(map_err)?;
    Ok(tauri::ipc::Response::new(bytes))
}

pub fn config_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .expect("no app config dir available on this platform")
}
