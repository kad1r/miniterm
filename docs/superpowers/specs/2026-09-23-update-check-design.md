# miniterm In-App Update Check — Design

**Date:** 2026-09-23
**Status:** Approved, ready for implementation planning

## Goal

Let miniterm tell the user when a newer release exists and, on the
user's request, download and run the installer. Two entry points:

1. **Automatic check on every app startup** (silent — surfaces only when
   an update is found).
2. **Manual "Check for updates" button** in Settings → About (always
   checks immediately, reports the result either way).

## Chosen approach: custom GitHub-releases check (approach "B")

The check queries the public GitHub Releases API, compares the latest
release tag against the running version, and — if newer — offers to
download the NSIS `-setup.exe` asset and launch it.

Rejected alternative: the official `tauri-plugin-updater` (approach
"A"). It gives a cleaner in-app install/relaunch UX, but requires a
minisign signing key pair plus a `latest.json` manifest on every
release. With no CI and every release cut by hand on the developer's
machine, that adds two manual steps per release and a long-lived
secret whose loss or leak is a real risk. It also does **not** clear
Windows SmartScreen (that needs Authenticode, a separate concern), so
it buys no advantage there. Approach B adds **nothing** to the release
recipe.

## Non-goals

- **No cryptographic signature verification of the download.** This is
  the accepted trade-off of approach B. Integrity relies on HTTPS and
  GitHub hosting the asset. Documented here so it is a deliberate
  choice, not an oversight.
- **Does not fix SmartScreen.** Running the downloaded unsigned
  installer still shows the SmartScreen / UAC prompt, exactly as a
  manual download does today. Out of scope; only Authenticode signing
  fixes it.
- No delta/partial updates. Full installer download only.
- No auto-install without explicit user confirmation.

## Architecture

All network and process work lives in **Rust**, matching the existing
split where Rust owns the external world (PTYs, config, filesystem) and
TypeScript owns the tree and UI state.

Consequences of keeping HTTP in Rust:

- **CSP is unchanged.** No frontend `fetch` to external hosts, so
  `connect-src` stays as-is (`ipc:` only).
- The installer is launched from Rust via `std::process::Command`, so
  no shell plugin and **no new capability/permission** is required.
- New Rust dependency: `reqwest` with `default-features = false` and
  the `rustls-tls` + `stream` features. `rustls` avoids the OpenSSL
  build friction of the GNU toolchain this project uses.

## Rust commands (`src-tauri/src/commands.rs`)

### `check_update() -> UpdateInfo`

1. `GET https://api.github.com/repos/kad1r/miniterm/releases/latest`
   with a `User-Agent` header (GitHub requires one).
2. Parse `tag_name` (`"v0.9.0"` → `0.9.0`) and compare against the
   running app version using a small semver comparison.
3. From the release `assets`, pick the one whose name ends with
   `_x64-setup.exe` (the NSIS installer).

Returns:

```
UpdateInfo {
  current: String,       // running version, e.g. "0.9.0"
  latest: String,        // latest tag stripped of leading "v"
  isNewer: bool,         // latest > current
  downloadUrl: String,   // browser_download_url of the NSIS asset
  notes: String,         // release body (changelog text)
}
```

Errors (network failure, rate limit, missing asset) surface as a
command error the caller can distinguish from "no update".

### `download_update(url: String) -> String`

1. **Validate the URL host** before any request: it must be
   `github.com` or a `*.githubusercontent.com` host. Reject anything
   else — never download or launch an arbitrary URL, even though it
   came from the API response.
2. Stream the body to a file in the OS temp dir
   (`miniterm_<version>_x64-setup.exe`).
3. Emit an `update-progress` event as bytes arrive:
   `{ downloaded: u64, total: Option<u64> }` (total from
   `Content-Length` when present).
4. Return the temp file path.

### `install_update(path: String)`

Called only after the user confirms the "close and install" prompt.
Launches the installer (`std::process::Command::new(path).spawn()`),
then the app exits so NSIS can replace the locked files.

## Frontend

- **`src/ipc/index.ts`** — thin wrappers: `checkUpdate()`,
  `downloadUpdate(url)`, `installUpdate(path)`, and an
  `onUpdateProgress(cb)` event listener.
- **`src/store/update.ts`** — a small state machine:
  `idle → checking → upToDate | available → downloading → ready → error`.
  Holds the current `UpdateInfo`, download progress, and last error.
- **Startup (`App.svelte` `onMount`)** — fire `checkUpdate()` silently.
  If `isNewer`, show the banner. On any error or up-to-date result,
  show nothing (fully silent).
- **Banner component** — "Yeni sürüm X.Y.Z var" with **İndir & Kur**
  and **Kapat** actions. Dismiss hides it for the session.
- **About tab (`Settings.svelte`)** — a **Güncellemeleri denetle**
  button that checks immediately and reports the outcome inline:
  checking / "günceldsin" / "yeni sürüm var" + download action. Errors
  are shown here (unlike the silent startup path).
- **Download & install flow** — clicking İndir & Kur runs
  `downloadUpdate`, shows a progress bar driven by `update-progress`,
  then on completion asks the user to confirm closing the app to
  install. On confirm, calls `install_update`; the app then quits.

## Error handling

- **Startup check:** any failure is swallowed silently (no banner, no
  dialog). The app must never block or nag on a failed network check.
- **Manual check (About):** failures are shown as an inline message
  ("Güncelleme denetlenemedi").
- **GitHub rate limit:** unauthenticated requests allow 60/hour per IP,
  ample for per-launch checks; a 403 is treated as a check failure.
- **Missing NSIS asset** on the latest release: treated as "no update
  available" for the startup path, and an error for the manual path.

## Testing

- **Rust (unit, pure functions):**
  - semver comparison (equal, newer, older, differing lengths)
  - asset selection (`_x64-setup.exe` picked; other assets ignored;
    none present)
  - download-URL host validation (github hosts accepted, others
    rejected)
  - The HTTP call itself is not unit-tested.
- **TypeScript (vitest):**
  - `update.ts` state-machine transitions.

## Dependencies

- `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream"] }`
  (plus `futures-util` if needed for the byte stream).
- Bump the `miniterm` entry in `Cargo.lock` accordingly (part of the
  normal build, not a manual release step).

## Impact on the release recipe

**None.** The existing hand-cut flow is unchanged. The only standing
requirement is that each release attach the NSIS `*_x64-setup.exe`
asset — which it already does.
