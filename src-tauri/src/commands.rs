use crate::agents;
use crate::config::{self, Config, LoadResult};
use crate::pty::{SessionManager, SpawnOpts};
use crate::shell::{self, ShellInfo};
use crate::update::{self, UpdateInfo};
use std::path::PathBuf;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub sessions: SessionManager,
    pub config_dir: PathBuf,
    /// The installer `download_update` wrote. `install_update` launches this
    /// file and nothing else, whatever path the frontend hands it.
    pub downloaded_installer: std::sync::Mutex<Option<PathBuf>>,
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
    // Clamp before persisting so the on-disk file obeys the same depth/shape
    // invariants that load enforces, even if the frontend sends something out
    // of bounds.
    let (config, _) = config::clamp_config(config);
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

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    update::fetch_latest(&current).await
}

#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> Result<String, String> {
    let path = update::download(&app, &url).await?;
    *state.downloaded_installer.lock().unwrap_or_else(|e| e.into_inner()) = Some(path.clone());
    Ok(path.to_string_lossy().into_owned())
}

/// Launch the downloaded installer, then quit so NSIS can replace locked files.
/// Called only after the user confirms the close-and-install prompt. Only the
/// file this process downloaded may be launched — never an arbitrary path.
#[tauri::command]
pub fn install_update(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<(), String> {
    let expected = state
        .downloaded_installer
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or("no installer has been downloaded")?;
    if PathBuf::from(&path) != expected {
        return Err("refusing to launch a file that was not downloaded by the updater".into());
    }
    if !expected.is_file() {
        return Err("installer file not found".into());
    }
    std::process::Command::new(&expected).spawn().map_err(map_err)?;
    app.exit(0);
    Ok(())
}

/// The workspace's shared agent folder (created on demand), as an absolute path.
#[tauri::command]
pub fn agents_dir(state: State<'_, AppState>, workspace_id: String) -> Result<String, String> {
    agents::dir(&state.config_dir, &workspace_id).map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn write_agent_file(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
    content: String,
) -> Result<(), String> {
    agents::write(&state.config_dir, &workspace_id, &name, &content)
}

#[tauri::command]
pub fn rename_agent_file(
    state: State<'_, AppState>,
    workspace_id: String,
    from: String,
    to: String,
) -> Result<(), String> {
    agents::rename(&state.config_dir, &workspace_id, &from, &to)
}

#[tauri::command]
pub fn remove_agent_file(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
) -> Result<(), String> {
    agents::remove(&state.config_dir, &workspace_id, &name)
}

#[tauri::command]
pub fn remove_agents_dir(state: State<'_, AppState>, workspace_id: String) -> Result<(), String> {
    agents::remove_dir(&state.config_dir, &workspace_id)
}
