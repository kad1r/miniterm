/**
 * Tests for src/ipc/index.ts
 *
 * Proves Finding 1 fix: attachSession deduplicates channels so that a
 * double-attach leaves exactly one live channel, and a late post-detach
 * message from the Rust sink reaches no stale callback.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// ---------------------------------------------------------------------------
// Fake Channel — mimics the onmessage-only API surface used by attachSession.
// Each instance tracks whether its onmessage was silenced (set to no-op).
// ---------------------------------------------------------------------------
class FakeChannel {
  onmessage: (msg: unknown) => void = () => {};
  /** Fire a message as if Rust sent it. */
  fire(msg: unknown) {
    this.onmessage(msg);
  }
}

const channels: FakeChannel[] = [];

// Mock @tauri-apps/api/core before any import of the ipc module.
vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (msg: unknown) => void = () => {};
    constructor() {
      // Wrap in a FakeChannel proxy so we can call fire() in tests.
      const fake = new FakeChannel();
      // Mirror property access to the fake instance.
      Object.defineProperty(this, "onmessage", {
        get: () => fake.onmessage,
        set: (fn) => { fake.onmessage = fn; },
        enumerable: true,
        configurable: true,
      });
      (this as unknown as { _fake: FakeChannel })._fake = fake;
      channels.push(fake);
    }
  },
  invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue(null),
}));

// ---------------------------------------------------------------------------
// Import ipc AFTER mocks are registered.
// ---------------------------------------------------------------------------
import * as ipc from "./index";
import { invoke } from "@tauri-apps/api/core";

describe("attachSession / detachSession — channel deduplication", () => {
  beforeEach(() => {
    channels.length = 0;
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("first attach registers exactly one active channel", async () => {
    const onData = vi.fn();
    await ipc.attachSession(1, onData);

    // One channel created.
    expect(channels).toHaveLength(1);
    // Fire a message — callback should be called.
    channels[0].fire(new ArrayBuffer(3));
    expect(onData).toHaveBeenCalledTimes(1);
  });

  it("double-attach to the same id silences the first channel before registering the second", async () => {
    const onData1 = vi.fn();
    const onData2 = vi.fn();

    await ipc.attachSession(1, onData1);
    const firstChannel = channels[0];

    await ipc.attachSession(1, onData2);

    // Two Channel objects created in total.
    expect(channels).toHaveLength(2);

    // First channel's onmessage is now a no-op — firing it must NOT call onData1.
    firstChannel.fire(new ArrayBuffer(2));
    expect(onData1).not.toHaveBeenCalled();

    // Second channel is live — firing it calls onData2.
    channels[1].fire(new ArrayBuffer(4));
    expect(onData2).toHaveBeenCalledTimes(1);
  });

  it("detachSession silences the channel so a late post-detach Rust message is a no-op", async () => {
    const onData = vi.fn();
    await ipc.attachSession(2, onData);
    const liveChannel = channels[0];

    await ipc.detachSession(2);

    // Simulate the Rust sink firing one final message after detach returns.
    liveChannel.fire(new ArrayBuffer(5));

    // Must not reach the stale callback, must not throw.
    expect(onData).not.toHaveBeenCalled();
  });

  it("attaching again after detach creates a fresh active channel", async () => {
    const onData1 = vi.fn();
    const onData2 = vi.fn();

    await ipc.attachSession(3, onData1);
    await ipc.detachSession(3);
    await ipc.attachSession(3, onData2);

    // Three Channel objects total (first attach, detach silences it, second attach new one).
    expect(channels).toHaveLength(2);

    // Late message on the detached channel is silent.
    channels[0].fire(new ArrayBuffer(1));
    expect(onData1).not.toHaveBeenCalled();

    // New channel is live.
    channels[1].fire(new ArrayBuffer(1));
    expect(onData2).toHaveBeenCalledTimes(1);
  });

  it("bytes arrive zero-copy as Uint8Array wrapping the ArrayBuffer", async () => {
    const received: Uint8Array[] = [];
    await ipc.attachSession(4, (b) => received.push(b));

    const buf = new ArrayBuffer(3);
    new Uint8Array(buf).set([10, 20, 30]);
    channels[0].fire(buf);

    expect(received).toHaveLength(1);
    expect(received[0]).toBeInstanceOf(Uint8Array);
    expect(Array.from(received[0])).toEqual([10, 20, 30]);
    // Zero-copy: same underlying buffer.
    expect(received[0].buffer).toBe(buf);
  });
});
