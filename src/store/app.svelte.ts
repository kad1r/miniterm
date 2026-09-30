import * as ipc from "../ipc";
import { t } from "../i18n/locale.svelte";
import { createSaver, SAVE_DEBOUNCE_MS } from "./persist";
import type { Config, ShellInfo } from "./types";
import { clearPaneNames, NAMES_RESET_KEY } from "./agents";

const emptyConfig: Config = {
  version: 1,
  aiTools: [],
  directories: [],
  recentDirs: [],
  defaultShellId: "",
  tree: [],
};

export const app = $state({
  config: emptyConfig as Config,
  shells: [] as ShellInfo[],
  activeWorkspaceId: null as string | null,
  view: "workspace" as "workspace" | "settings",
  ready: false,
  loadFailed: false,
  toast: null as { text: string; tone: "info" | "error" } | null,
});

const saver = createSaver<Config>(async (config) => {
  try {
    await ipc.saveConfig(config);
  } catch (e) {
    notify(t("app.saveFailed", { error: String(e) }), "error");
  }
}, SAVE_DEBOUNCE_MS);

const TOAST_TIMEOUT_MS = 3000;
let toastTimer: ReturnType<typeof setTimeout> | null = null;

/** Info messages fade on their own; errors stay until the user dismisses them. */
export function notify(text: string, tone: "info" | "error" = "info") {
  if (toastTimer !== null) clearTimeout(toastTimer);
  toastTimer = null;
  app.toast = { text, tone };
  if (tone === "info") {
    toastTimer = setTimeout(dismissToast, TOAST_TIMEOUT_MS);
  }
}

export function dismissToast() {
  if (toastTimer !== null) clearTimeout(toastTimer);
  toastTimer = null;
  app.toast = null;
}

/** Changes the config and queues a debounced write to disk.
 *  Writes nothing if startup failed — refusing silently beats overwriting
 *  the real file with an empty config. */
export function commit(mutate: (config: Config) => Config) {
  if (app.loadFailed) return;
  app.config = mutate(app.config);
  saver.schedule(app.config);
}

/** One-off: drop the agent names pinned by the old per-workspace pool so every
 *  pane is handed a fresh, app-wide unique god name. Runs before any workspace
 *  is activated, so no shell is spawned with a name that is about to change. */
function resetAgentNamesOnce() {
  if (typeof localStorage === "undefined") return;
  try {
    if (localStorage.getItem(NAMES_RESET_KEY)) return;
    const { tree, cleared } = clearPaneNames(app.config.tree);
    if (cleared.length > 0) {
      app.config = { ...app.config, tree };
      saver.schedule(app.config);
      // Their logs and inboxes are named after the old agents.
      for (const id of cleared) void ipc.removeAgentsDir(id).catch(() => {});
    }
    localStorage.setItem(NAMES_RESET_KEY, "1");
  } catch {
    // Storage blocked: skip the reset rather than repeat it every start.
  }
}

export async function bootstrap() {
  try {
    const [result, shells] = await Promise.all([ipc.loadConfig(), ipc.detectShells()]);
    app.config = result.config;
    app.shells = shells;
    resetAgentNamesOnce();
    if (result.status.kind === "recovered") {
      notify(t("app.recovered", { backup: result.status.backup }), "error");
    }
  } catch (e) {
    // app.config stays emptyConfig — the rest of the UI is safe against it.
    // loadFailed flag prevents commit() from overwriting the real config.json
    // with an empty tree during this session.
    app.loadFailed = true;
    saver.cancel();
    notify(t("app.bootFailed", { error: String(e) }), "error");
  } finally {
    // xterm derives its cell geometry by measuring a glyph once, at construction.
    // If the bundled woff2 arrives after that, every cell keeps the fallback
    // font's metrics and the text drifts out of its grid. The files are local,
    // so this resolves within a frame or two. `document` is absent in the node
    // test environment.
    if (typeof document !== "undefined") await document.fonts.ready;
    app.ready = true;
    // Tauri 2 CloseRequested: await the flush before the window is destroyed.
    // This is the primary path — it prevents the last change being lost if the
    // window closes within the 300 ms debounce window.
    void ipc.onCloseRequested(() => saver.flush());
    // beforeunload stays as a best-effort fallback (non-Tauri env, dev server).
    window.addEventListener("beforeunload", () => void saver.flush());
  }
}
