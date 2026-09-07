import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Config, LoadResult, ShellInfo } from "../store/types";

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

/** Kanal `InvokeResponseBody::Raw` taşır; JSON serileştirme yoktur. */
export function attachSession(id: number, onData: (bytes: Uint8Array) => void): Promise<void> {
  const channel = new Channel<ArrayBuffer | Uint8Array | number[]>();
  channel.onmessage = (message) => onData(toBytes(message));
  return invoke<void>("attach_session", { id, channel });
}

export function detachSession(id: number): Promise<void> {
  return invoke<void>("detach_session", { id });
}

export async function getBuffer(id: number): Promise<Uint8Array> {
  return toBytes(await invoke<number[]>("get_buffer", { id }));
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
