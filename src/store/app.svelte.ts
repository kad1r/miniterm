import * as ipc from "../ipc";
import { createSaver, SAVE_DEBOUNCE_MS } from "./persist";
import type { Config, ShellInfo } from "./types";

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
    notify(`Ayarlar kaydedilemedi: ${e}`, "error");
  }
}, SAVE_DEBOUNCE_MS);

export function notify(text: string, tone: "info" | "error" = "info") {
  app.toast = { text, tone };
}

export function dismissToast() {
  app.toast = null;
}

/** Config'i değiştirir ve debounce'lu diske yazmayı kuyruğa alır.
 *  Başlatma sırasında hata oluştuysa hiçbir şey yazmaz — boş config'i diske
 *  basmaktansa sessizce reddetmek tercih edilir. */
export function commit(mutate: (config: Config) => Config) {
  if (app.loadFailed) return;
  app.config = mutate(app.config);
  saver.schedule(app.config);
}

export async function bootstrap() {
  try {
    const [result, shells] = await Promise.all([ipc.loadConfig(), ipc.detectShells()]);
    app.config = result.config;
    app.shells = shells;
    if (result.status.kind === "recovered") {
      notify(
        `Ayar dosyası okunamadı, ${result.status.backup} olarak kenara alındı. Boş bir çalışma alanıyla başlatıldı.`,
        "error",
      );
    }
  } catch (e) {
    // app.config stays emptyConfig — the rest of the UI is safe against it.
    // loadFailed flag prevents commit() from overwriting the real config.json
    // with an empty tree during this session.
    app.loadFailed = true;
    saver.cancel();
    notify(`Başlatma hatası: ${e} — bu oturumda değişiklikler kaydedilmeyecek.`, "error");
  } finally {
    app.ready = true;
    // Tauri 2 CloseRequested: await the flush before the window is destroyed.
    // This is the primary path — it prevents the last change being lost if the
    // window closes within the 300 ms debounce window.
    void ipc.onCloseRequested(() => saver.flush());
    // beforeunload stays as a best-effort fallback (non-Tauri env, dev server).
    window.addEventListener("beforeunload", () => void saver.flush());
  }
}
