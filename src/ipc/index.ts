import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Config, LoadResult, ShellInfo } from "../store/types";

/** Active channels keyed by session id. One entry per id at most. */
const _channels = new Map<number, Channel<ArrayBuffer | Uint8Array | number[]>>();

export interface SpawnOpts {
  cwd: string;
  program: string;
  args: string[];
  initialCommand: string | null;
  cols: number;
  rows: number;
}

export function loadConfig(): Promise<LoadResult> {
  return invoke<LoadResult>("load_config");
}

export function saveConfig(config: Config): Promise<void> {
  return invoke<void>("save_config", { config });
}

export function detectShells(): Promise<ShellInfo[]> {
  return invoke<ShellInfo[]>("detect_shells");
}

export async function pickDirectory(): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}

export function spawnSession(opts: SpawnOpts): Promise<number> {
  return invoke<number>("spawn_session", { opts });
}

export function writeSession(id: number, data: Uint8Array): Promise<void> {
  return invoke<void>("write_session", { id, data: Array.from(data) });
}

export function resizeSession(id: number, cols: number, rows: number): Promise<void> {
  return invoke<void>("resize_session", { id, cols, rows });
}

export function killSession(id: number): Promise<void> {
  return invoke<void>("kill_session", { id });
}

/** Kanal `InvokeResponseBody::Raw` taşır; JSON serileştirme yoktur.
 *
 * Aynı id için ikinci kez çağrılırsa eski kanalın onmessage'ı no-op'a
 * yönlendirilir; böylece Rust'ın detach sonrası geç gönderebileceği tek
 * mesaj ne eski callback'e ulaşır ne de hata fırlatır.
 */
export async function attachSession(id: number, onData: (bytes: Uint8Array) => void): Promise<void> {
  // Silence any previously registered channel for this id before replacing it.
  const prev = _channels.get(id);
  if (prev !== undefined) {
    prev.onmessage = () => {};
  }

  const channel = new Channel<ArrayBuffer | Uint8Array | number[]>();
  channel.onmessage = (message) => onData(toBytes(message));
  _channels.set(id, channel);
  await invoke<void>("attach_session", { id, channel });
}

export async function detachSession(id: number): Promise<void> {
  const ch = _channels.get(id);
  if (ch !== undefined) {
    // Silence the channel first so any post-detach message from the Rust sink
    // reaches a no-op and never calls a stale callback.
    ch.onmessage = () => {};
    _channels.delete(id);
  }
  await invoke<void>("detach_session", { id });
}

export async function getBuffer(id: number): Promise<Uint8Array> {
  const buf = await invoke<ArrayBuffer>("get_buffer", { id });
  return new Uint8Array(buf);
}

export function onSessionExit(
  cb: (e: { id: number; code: number | null }) => void,
): Promise<() => void> {
  return listen<{ id: number; code: number | null }>("session-exit", (e) => cb(e.payload));
}

function toBytes(value: ArrayBuffer | Uint8Array | number[]): Uint8Array {
  if (value instanceof Uint8Array) return value;
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  return Uint8Array.from(value);
}
