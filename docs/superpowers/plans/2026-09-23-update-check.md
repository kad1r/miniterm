# In-App Update Check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let miniterm detect a newer GitHub release on startup and from a Settings→About button, then download and launch the NSIS installer on the user's confirmation.

**Architecture:** All network and process work lives in Rust (matching the existing split where Rust owns the external world). Three new Tauri commands — `check_update`, `download_update`, `install_update` — back a Svelte update store, a startup banner, and an About-tab button. The download is streamed with progress events. No signing, no CSP change, no release-recipe change.

**Tech Stack:** Tauri 2 (Rust) + `reqwest` (rustls-tls, stream) + Svelte 5 + vitest.

**Spec:** `docs/superpowers/specs/2026-09-23-update-check-design.md`

## Global Constraints

- Repo API base: `https://api.github.com/repos/kad1r/miniterm/releases/latest` (exact).
- NSIS asset is the release asset whose name ends with `_x64-setup.exe`.
- Version strings compare as semver `major.minor.patch`; strip a leading `v` from tags.
- Download URL host MUST be `github.com` or end with `.githubusercontent.com` — reject anything else before any request or process launch.
- Startup check is fully silent on error / up-to-date (no banner, no dialog). Every launch checks; no throttle.
- No cryptographic signature verification (accepted trade-off); integrity is HTTPS + GitHub only.
- CSP stays unchanged (no frontend fetch to external hosts).
- All new Rust deps use `rustls-tls`, never native-tls/OpenSSL (GNU toolchain).
- User-facing copy goes through the i18n `t(...)` system in both locales (`src/i18n/messages.ts`), never hard-coded strings.

---

### Task 1: Rust pure helpers (semver, asset pick, host validation)

**Files:**
- Create: `src-tauri/src/update.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod update;` near the other `pub mod` lines at top)
- Test: inline `#[cfg(test)]` module in `src-tauri/src/update.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub fn is_newer(current: &str, latest: &str) -> bool`
  - `pub fn strip_v(tag: &str) -> &str`
  - `pub fn pick_setup_asset(assets: &[Asset]) -> Option<&Asset>` where `pub struct Asset { pub name: String, pub browser_download_url: String }`
  - `pub fn is_allowed_host(url: &str) -> bool`

- [ ] **Step 1: Write the failing tests**

Add to `src-tauri/src/update.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> Asset {
        Asset { name: name.into(), browser_download_url: format!("https://github.com/x/{name}") }
    }

    #[test]
    fn strip_v_removes_leading_v() {
        assert_eq!(strip_v("v0.9.0"), "0.9.0");
        assert_eq!(strip_v("0.9.0"), "0.9.0");
    }

    #[test]
    fn is_newer_compares_semver() {
        assert!(is_newer("0.9.0", "0.10.0"));
        assert!(is_newer("0.9.0", "1.0.0"));
        assert!(is_newer("0.9.0", "0.9.1"));
        assert!(!is_newer("0.9.0", "0.9.0"));
        assert!(!is_newer("0.9.1", "0.9.0"));
        assert!(!is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn is_newer_tolerates_v_prefix_and_short_parts() {
        assert!(is_newer("v0.9.0", "v0.9.1"));
        assert!(is_newer("0.9", "0.9.1"));
        assert!(!is_newer("0.9.0", "0.9"));
    }

    #[test]
    fn pick_setup_asset_finds_nsis_exe() {
        let assets = vec![
            asset("miniterm_0.9.0_x64_en-US.msi"),
            asset("miniterm_0.9.0_x64-setup.exe"),
        ];
        assert_eq!(pick_setup_asset(&assets).unwrap().name, "miniterm_0.9.0_x64-setup.exe");
    }

    #[test]
    fn pick_setup_asset_none_when_absent() {
        let assets = vec![asset("miniterm_0.9.0_x64_en-US.msi")];
        assert!(pick_setup_asset(&assets).is_none());
    }

    #[test]
    fn host_validation_accepts_github_only() {
        assert!(is_allowed_host("https://github.com/kad1r/miniterm/releases/download/v1/x.exe"));
        assert!(is_allowed_host("https://objects.githubusercontent.com/foo/x.exe"));
        assert!(!is_allowed_host("https://evil.com/x.exe"));
        assert!(!is_allowed_host("https://github.com.evil.com/x.exe"));
        assert!(!is_allowed_host("not a url"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test update::`
Expected: FAIL — `Asset`, `is_newer`, `strip_v`, `pick_setup_asset`, `is_allowed_host` not found.

- [ ] **Step 3: Write the implementation**

At the top of `src-tauri/src/update.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// Strip a single leading `v`/`V` from a release tag.
pub fn strip_v(tag: &str) -> &str {
    tag.strip_prefix('v').or_else(|| tag.strip_prefix('V')).unwrap_or(tag)
}

fn parts(v: &str) -> [u64; 3] {
    let v = strip_v(v.trim());
    let mut out = [0u64; 3];
    for (i, seg) in v.split('.').take(3).enumerate() {
        // Stop at the first non-digit so "1.0.0-rc1" degrades to 1.0.0.
        let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
        out[i] = digits.parse().unwrap_or(0);
    }
    out
}

/// True when `latest` is a strictly higher semver than `current`.
pub fn is_newer(current: &str, latest: &str) -> bool {
    parts(latest) > parts(current)
}

/// The NSIS installer asset, if the release carries one.
pub fn pick_setup_asset(assets: &[Asset]) -> Option<&Asset> {
    assets.iter().find(|a| a.name.ends_with("_x64-setup.exe"))
}

/// Only GitHub-owned hosts may be downloaded or launched.
pub fn is_allowed_host(url: &str) -> bool {
    match reqwest::Url::parse(url) {
        Ok(u) => match u.host_str() {
            Some(h) => h == "github.com" || h.ends_with(".githubusercontent.com"),
            None => false,
        },
        Err(_) => false,
    }
}
```

Add `pub mod update;` to `src-tauri/src/lib.rs` alongside the existing `pub mod config;` etc. (Note: `reqwest` is added as a dependency in Task 2; this task's `cargo test` will not link until Task 2 adds it. If running Task 1 standalone, add the `reqwest` line from Task 2 Step 1 first.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test update::`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/update.rs src-tauri/src/lib.rs
git commit -m "feat(update): semver, asset-pick, and host-validation helpers"
```

---

### Task 2: `check_update` command + reqwest dependency

**Files:**
- Modify: `src-tauri/Cargo.toml` (add deps)
- Modify: `src-tauri/src/update.rs` (add async `fetch_latest` + `UpdateInfo`)
- Modify: `src-tauri/src/commands.rs` (add `check_update` command)
- Modify: `src-tauri/src/lib.rs` (register `check_update` in `generate_handler!`)

**Interfaces:**
- Consumes: `update::{Asset, is_newer, pick_setup_asset, strip_v}` (Task 1).
- Produces:
  - `pub struct UpdateInfo { pub current: String, pub latest: String, pub is_newer: bool, pub download_url: String, pub notes: String }` (serialized camelCase).
  - `#[tauri::command] pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String>`

- [ ] **Step 1: Add dependencies**

In `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream"] }
futures-util = "0.3"
```

Run once to populate the lockfile: `cd src-tauri && cargo fetch`.

- [ ] **Step 2: Add `UpdateInfo` + `fetch_latest` to `update.rs`**

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub is_newer: bool,
    pub download_url: String,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

const LATEST_URL: &str = "https://api.github.com/repos/kad1r/miniterm/releases/latest";

/// Query GitHub for the latest release and fold it against `current`.
/// `Err` on any network/parse failure or when the release has no NSIS asset.
pub async fn fetch_latest(current: &str) -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("miniterm/{current}"))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("github returned {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let rel: Release = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let latest = strip_v(&rel.tag_name).to_string();
    let asset = pick_setup_asset(&rel.assets).ok_or("no NSIS installer asset on latest release")?;
    Ok(UpdateInfo {
        current: current.to_string(),
        latest: latest.clone(),
        is_newer: is_newer(current, &latest),
        download_url: asset.browser_download_url.clone(),
        notes: rel.body,
    })
}
```

- [ ] **Step 3: Add the command to `commands.rs`**

```rust
use crate::update::{self, UpdateInfo};

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    update::fetch_latest(&current).await
}
```

Register it in `src-tauri/src/lib.rs` inside `tauri::generate_handler![ ... ]`:

```rust
            commands::check_update,
```

- [ ] **Step 4: Verify it builds**

Run: `cd src-tauri && cargo build`
Expected: compiles clean (no test — the network call is not unit-tested per spec).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/update.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(update): check_update command querying GitHub releases"
```

---

### Task 3: `download_update` command with progress events

**Files:**
- Modify: `src-tauri/src/update.rs` (add `download` helper)
- Modify: `src-tauri/src/commands.rs` (add `download_update` command)
- Modify: `src-tauri/src/lib.rs` (register command)

**Interfaces:**
- Consumes: `update::is_allowed_host` (Task 1).
- Produces:
  - Event `update-progress` payload `{ downloaded: u64, total: Option<u64> }` (camelCase, serialized).
  - `#[tauri::command] pub async fn download_update(app: AppHandle, url: String) -> Result<String, String>` — returns the temp file path.

- [ ] **Step 1: Add the progress payload + download helper to `update.rs`**

```rust
use futures_util::StreamExt;
use std::io::Write;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Stream `url` to a temp file, emitting `update-progress` as bytes arrive.
/// Rejects any non-GitHub host before opening a connection.
pub async fn download(app: &AppHandle, url: &str) -> Result<std::path::PathBuf, String> {
    if !is_allowed_host(url) {
        return Err("refusing to download a non-GitHub url".into());
    }
    let file_name = url.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or("miniterm-setup.exe");
    let dest = std::env::temp_dir().join(file_name);

    let client = reqwest::Client::builder()
        .user_agent("miniterm-updater")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("download failed: {}", resp.status()));
    }
    let total = resp.content_length();
    let mut file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        let _ = app.emit("update-progress", Progress { downloaded, total });
    }
    file.flush().map_err(|e| e.to_string())?;
    Ok(dest)
}
```

- [ ] **Step 2: Add the command to `commands.rs`**

```rust
#[tauri::command]
pub async fn download_update(app: AppHandle, url: String) -> Result<String, String> {
    let path = update::download(&app, &url).await?;
    Ok(path.to_string_lossy().into_owned())
}
```

Register `commands::download_update,` in `lib.rs`.

- [ ] **Step 3: Verify it builds**

Run: `cd src-tauri && cargo build`
Expected: compiles clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/update.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(update): streaming download_update command with progress events"
```

---

### Task 4: `install_update` command

**Files:**
- Modify: `src-tauri/src/commands.rs` (add `install_update` command)
- Modify: `src-tauri/src/lib.rs` (register command)

**Interfaces:**
- Consumes: `update::is_allowed_host` indirectly (path already validated at download; re-check the file exists).
- Produces: `#[tauri::command] pub fn install_update(app: AppHandle, path: String) -> Result<(), String>` — spawns the installer and exits the app.

- [ ] **Step 1: Add the command to `commands.rs`**

```rust
/// Launch the downloaded installer, then quit so NSIS can replace locked files.
/// Called only after the user confirms the close-and-install prompt.
#[tauri::command]
pub fn install_update(app: AppHandle, path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.is_file() {
        return Err("installer file not found".into());
    }
    std::process::Command::new(&p).spawn().map_err(map_err)?;
    app.exit(0);
    Ok(())
}
```

Register `commands::install_update,` in `lib.rs`.

- [ ] **Step 2: Verify it builds**

Run: `cd src-tauri && cargo build`
Expected: compiles clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(update): install_update command that launches installer and exits"
```

---

### Task 5: TypeScript IPC wrappers + update store

**Files:**
- Modify: `src/ipc/index.ts` (add wrappers + progress listener)
- Create: `src/store/update.svelte.ts`
- Test: `src/store/update.test.ts`

**Interfaces:**
- Consumes: the four Rust commands + `update-progress` event.
- Produces:
  - ipc: `checkUpdate(): Promise<UpdateInfo>`, `downloadUpdate(url: string): Promise<string>`, `installUpdate(path: string): Promise<void>`, `onUpdateProgress(cb: (p: Progress) => void): Promise<() => void>`.
  - store: `update` reactive object with `status`, `info`, `progress`, `error`; and functions `runCheck(silent: boolean)`, `runDownloadInstall()`, `dismiss()`.
  - types: `UpdateInfo { current; latest; isNewer; downloadUrl; notes }`, `Progress { downloaded; total: number | null }`, `UpdateStatus = "idle" | "checking" | "upToDate" | "available" | "downloading" | "ready" | "error"`.

- [ ] **Step 1: Write the failing store tests**

`src/store/update.test.ts`:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest"

const ipc = {
  checkUpdate: vi.fn(),
  downloadUpdate: vi.fn(),
  installUpdate: vi.fn(),
  onUpdateProgress: vi.fn(async () => () => {}),
}
vi.mock("../ipc", () => ipc)

import { update, runCheck, dismiss } from "./update.svelte"

const info = (isNewer: boolean) => ({
  current: "0.9.0", latest: isNewer ? "0.10.0" : "0.9.0",
  isNewer, downloadUrl: "https://github.com/x.exe", notes: "notes",
})

beforeEach(() => {
  vi.clearAllMocks()
  dismiss()
})

describe("runCheck", () => {
  it("goes available when a newer release exists", async () => {
    ipc.checkUpdate.mockResolvedValue(info(true))
    await runCheck(false)
    expect(update.status).toBe("available")
    expect(update.info?.latest).toBe("0.10.0")
  })

  it("goes upToDate when current is latest", async () => {
    ipc.checkUpdate.mockResolvedValue(info(false))
    await runCheck(false)
    expect(update.status).toBe("upToDate")
  })

  it("stays idle on silent error", async () => {
    ipc.checkUpdate.mockRejectedValue(new Error("offline"))
    await runCheck(true)
    expect(update.status).toBe("idle")
    expect(update.error).toBeNull()
  })

  it("records error on non-silent failure", async () => {
    ipc.checkUpdate.mockRejectedValue(new Error("offline"))
    await runCheck(false)
    expect(update.status).toBe("error")
    expect(update.error).toBe("offline")
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- update.test`
Expected: FAIL — `./update.svelte` does not exist.

- [ ] **Step 3: Add ipc wrappers to `src/ipc/index.ts`**

```ts
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"

export interface UpdateInfo {
  current: string
  latest: string
  isNewer: boolean
  downloadUrl: string
  notes: string
}

export interface Progress {
  downloaded: number
  total: number | null
}

export function checkUpdate(): Promise<UpdateInfo> {
  return invoke<UpdateInfo>("check_update")
}

export function downloadUpdate(url: string): Promise<string> {
  return invoke<string>("download_update", { url })
}

export function installUpdate(path: string): Promise<void> {
  return invoke<void>("install_update", { path })
}

export function onUpdateProgress(cb: (p: Progress) => void): Promise<() => void> {
  return listen<Progress>("update-progress", (e) => cb(e.payload))
}
```

(`invoke` and `listen` are already imported at the top of the file — reuse the existing imports rather than duplicating them.)

- [ ] **Step 4: Write the store `src/store/update.svelte.ts`**

```ts
import { checkUpdate, downloadUpdate, installUpdate, onUpdateProgress, type Progress, type UpdateInfo } from "../ipc"

export type UpdateStatus =
  | "idle" | "checking" | "upToDate" | "available" | "downloading" | "ready" | "error"

export const update = $state<{
  status: UpdateStatus
  info: UpdateInfo | null
  progress: Progress | null
  error: string | null
}>({ status: "idle", info: null, progress: null, error: null })

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

/** Check GitHub. `silent` = startup path: swallow errors and up-to-date quietly. */
export async function runCheck(silent: boolean): Promise<void> {
  update.status = "checking"
  update.error = null
  try {
    const info = await checkUpdate()
    update.info = info
    if (info.isNewer) update.status = "available"
    else update.status = silent ? "idle" : "upToDate"
  } catch (e) {
    if (silent) {
      update.status = "idle"
    } else {
      update.status = "error"
      update.error = message(e)
    }
  }
}

/** Download the available update, reporting progress; leaves status "ready". */
export async function runDownload(): Promise<string | null> {
  if (!update.info) return null
  update.status = "downloading"
  update.progress = null
  const stop = await onUpdateProgress((p) => (update.progress = p))
  try {
    const path = await downloadUpdate(update.info.downloadUrl)
    update.status = "ready"
    return path
  } catch (e) {
    update.status = "error"
    update.error = message(e)
    return null
  } finally {
    stop()
  }
}

/** Launch the installer; the app exits on success so this never resolves in practice. */
export async function runInstall(path: string): Promise<void> {
  try {
    await installUpdate(path)
  } catch (e) {
    update.status = "error"
    update.error = message(e)
  }
}

export function dismiss(): void {
  update.status = "idle"
  update.info = null
  update.progress = null
  update.error = null
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `npm test -- update.test`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add src/ipc/index.ts src/store/update.svelte.ts src/store/update.test.ts
git commit -m "feat(update): ipc wrappers and update store state machine"
```

---

### Task 6: i18n strings

**Files:**
- Modify: `src/i18n/messages.ts` (add keys to both locales)

**Interfaces:**
- Consumes: nothing.
- Produces: message keys used by Tasks 7 and 8 — `update.bannerAvailable`, `update.download`, `update.dismiss`, `update.downloading`, `update.installReady`, `update.installConfirm`, `update.installNow`, `update.checkButton`, `update.checking`, `update.upToDate`, `update.failed`.

- [ ] **Step 1: Add the keys**

Open `src/i18n/messages.ts` and add to BOTH the English and Turkish maps (match the existing structure — inspect a nearby key such as `settings.about.*` to copy the exact object shape and interpolation syntax). Values:

English:
```
"update.bannerAvailable": "New version {version} available",
"update.download": "Download & install",
"update.dismiss": "Dismiss",
"update.downloading": "Downloading… {percent}%",
"update.installReady": "Download complete.",
"update.installConfirm": "Close miniterm and install now?",
"update.installNow": "Close & install",
"update.checkButton": "Check for updates",
"update.checking": "Checking…",
"update.upToDate": "You're on the latest version.",
"update.failed": "Could not check for updates.",
```

Turkish:
```
"update.bannerAvailable": "Yeni sürüm {version} mevcut",
"update.download": "İndir & kur",
"update.dismiss": "Kapat",
"update.downloading": "İndiriliyor… %{percent}",
"update.installReady": "İndirme tamamlandı.",
"update.installConfirm": "miniterm kapatılıp şimdi kurulsun mu?",
"update.installNow": "Kapat & kur",
"update.checkButton": "Güncellemeleri denetle",
"update.checking": "Denetleniyor…",
"update.upToDate": "En güncel sürümü kullanıyorsun.",
"update.failed": "Güncelleme denetlenemedi.",
```

Use whatever interpolation placeholder style the file already uses; if it differs from `{version}`/`{percent}`, adapt these to match.

- [ ] **Step 2: Verify the locale test still passes**

Run: `npm test -- messages.test`
Expected: PASS — the test that both locales carry identical key sets stays green (that is exactly what it guards).

- [ ] **Step 3: Commit**

```bash
git add src/i18n/messages.ts
git commit -m "feat(update): i18n strings for the update banner and About button"
```

---

### Task 7: Startup check + banner

**Files:**
- Create: `src/lib/UpdateBanner.svelte`
- Modify: `src/App.svelte` (fire `runCheck(true)` on mount; render the banner)

**Interfaces:**
- Consumes: `update`, `runCheck`, `runDownload`, `runInstall`, `dismiss` (Task 5); i18n keys (Task 6).
- Produces: the banner UI. No new exports.

- [ ] **Step 1: Write the banner component**

`src/lib/UpdateBanner.svelte`:

```svelte
<script lang="ts">
  import { update, runDownload, runInstall, dismiss } from "../store/update.svelte"
  import { t } from "../i18n/locale.svelte"

  const percent = $derived(
    update.progress?.total
      ? Math.floor((update.progress.downloaded / update.progress.total) * 100)
      : 0,
  )

  async function onDownload() {
    const path = await runDownload()
    if (path && confirm(t("update.installConfirm"))) await runInstall(path)
  }
</script>

{#if update.status === "available" || update.status === "downloading" || update.status === "ready"}
  <div class="update-banner">
    {#if update.status === "downloading"}
      <span>{t("update.downloading", { percent })}</span>
    {:else if update.status === "ready"}
      <span>{t("update.installReady")}</span>
    {:else}
      <span>{t("update.bannerAvailable", { version: update.info?.latest ?? "" })}</span>
      <button onclick={onDownload}>{t("update.download")}</button>
      <button class="ghost" onclick={dismiss}>{t("update.dismiss")}</button>
    {/if}
  </div>
{/if}

<style>
  .update-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: var(--bg-raised);
    border-bottom: 1px solid var(--border);
    color: var(--text);
    font-size: calc(12.5px * var(--font-scale, 1));
  }
  .update-banner button {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--accent, var(--bg-app));
    color: var(--text);
    cursor: pointer;
    font: inherit;
  }
  .update-banner button.ghost {
    background: transparent;
    color: var(--text-dim);
  }
</style>
```

- [ ] **Step 2: Wire it into `App.svelte`**

Add the import beside the other `./lib/*` imports:

```ts
import UpdateBanner from "./lib/UpdateBanner.svelte"
```

Add the startup check inside the existing first `onMount` (the async one that calls `bootstrap()`), after `await bootstrap()`:

```ts
    void runCheck(true)
```

and import it:

```ts
import { runCheck } from "./store/update.svelte"
```

Render the banner at the top of the `.body` block, immediately inside `{:else}` before `<div class="body">` becomes visible — place it just under the `{#if !app.ready}...{:else}` so it spans above the sidebar/main row. Concretely, wrap the existing `<div class="body">` so the banner sits above it:

```svelte
  {:else}
    <UpdateBanner />
    <div class="body">
```

- [ ] **Step 3: Type-check and build the frontend**

Run: `npm run check && npm run build`
Expected: no type errors; build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/lib/UpdateBanner.svelte src/App.svelte
git commit -m "feat(update): silent startup check with an update banner"
```

---

### Task 8: About-tab "Check for updates" button

**Files:**
- Modify: `src/lib/Settings.svelte` (add button + status line to the About tab)

**Interfaces:**
- Consumes: `update`, `runCheck`, `runDownload`, `runInstall` (Task 5); i18n keys (Task 6).
- Produces: About-tab UI. No new exports.

- [ ] **Step 1: Add imports to `Settings.svelte`**

Beside the existing store imports:

```ts
import { update, runCheck, runDownload, runInstall } from "../store/update.svelte"
import { t } from "../i18n/locale.svelte"  // already imported — do not duplicate
```

Add a handler in the `<script>`:

```ts
  async function checkForUpdates() {
    await runCheck(false)
  }
  async function downloadAndInstall() {
    const path = await runDownload()
    if (path && confirm(t("update.installConfirm"))) await runInstall(path)
  }
```

- [ ] **Step 2: Add the UI under the About source line**

In the About tab markup, after `<p class="repo"><code>{REPO}</code></p>` (around line 324), add:

```svelte
      <h3>{t("update.checkButton")}</h3>
      <div class="update-check">
        <button onclick={checkForUpdates} disabled={update.status === "checking" || update.status === "downloading"}>
          {update.status === "checking" ? t("update.checking") : t("update.checkButton")}
        </button>
        {#if update.status === "upToDate"}
          <span class="hint">{t("update.upToDate")}</span>
        {:else if update.status === "error"}
          <span class="hint err">{t("update.failed")}</span>
        {:else if update.status === "available"}
          <span class="hint">{t("update.bannerAvailable", { version: update.info?.latest ?? "" })}</span>
          <button onclick={downloadAndInstall}>{t("update.download")}</button>
        {:else if update.status === "downloading"}
          <span class="hint">{t("update.downloading", { percent: update.progress?.total ? Math.floor((update.progress.downloaded / update.progress.total) * 100) : 0 })}</span>
        {:else if update.status === "ready"}
          <span class="hint">{t("update.installReady")}</span>
        {/if}
      </div>
```

Add matching styles in the `<style>` block (reuse existing `.hint` / `.err` if present; otherwise):

```css
  .update-check {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
```

- [ ] **Step 3: Type-check and build**

Run: `npm run check && npm run build`
Expected: no type errors; build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/lib/Settings.svelte
git commit -m "feat(update): check-for-updates button in Settings About tab"
```

---

## Self-Review

**Spec coverage:**
- Startup silent check → Task 7. Manual About button → Task 8. `check_update`/`download_update`/`install_update` → Tasks 2/3/4. HTTP-in-Rust / CSP unchanged / no capability → Tasks 2–4 (no CSP or capability edits anywhere). Host validation → Task 1 + enforced in Task 3. rustls-tls → Task 2. Progress events → Task 3/5/7. State machine → Task 5. i18n → Task 6. No-signature / SmartScreen / release-recipe-unchanged → design decisions, nothing to implement. All covered.

**Placeholder scan:** No TBD/TODO; every code step carries real code. i18n values spelled out for both locales.

**Type consistency:** `UpdateInfo` fields (`current/latest/isNewer/downloadUrl/notes`) match between Rust `#[serde(rename_all="camelCase")]` and the TS interface. `Progress { downloaded, total }` matches (Rust `Option<u64>` ↔ TS `number | null`). Command names (`check_update`/`download_update`/`install_update`) and event name (`update-progress`) are identical across Rust and TS. Store functions referenced by Tasks 7/8 (`runCheck`, `runDownload`, `runInstall`, `dismiss`, `update`) all exist in Task 5.

**Note for the executor:** end-to-end download/install can only be verified against a real GitHub release newer than the running build; the automated tests cover the pure logic and store transitions, and `cargo build` / `npm run build` cover wiring.
