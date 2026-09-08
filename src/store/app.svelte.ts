import * as ipc from "../ipc";
import { createSaver } from "./persist";
import type { Config, ShellInfo } from "./types";

const SAVE_DEBOUNCE_MS = 300;

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

/** Config'i değiştirir ve debounce'lu diske yazmayı kuyruğa alır. */
export function commit(mutate: (config: Config) => Config) {
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
    notify(`Başlatma hatası: ${e}`, "error");
  } finally {
    app.ready = true;
    window.addEventListener("beforeunload", () => void saver.flush());
  }
}
