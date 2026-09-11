import { describe, it, expect, vi } from "vitest";
import capability from "../../src-tauri/capabilities/default.json";
import { closeSequence } from "./close";

describe("closeSequence", () => {
  it("flushes before tearing the window down", async () => {
    const order: string[] = [];
    await closeSequence(
      async () => void order.push("flush"),
      async () => void order.push("destroy"),
    );
    expect(order).toEqual(["flush", "destroy"]);
  });

  it("still tears the window down when the flush fails", async () => {
    // A config write can fail on a full or read-only disk. Losing the last
    // rename is acceptable; trapping the user in a window that will not close
    // is not.
    const destroy = vi.fn(async () => {});
    await closeSequence(async () => {
      throw new Error("ENOSPC");
    }, destroy);
    expect(destroy).toHaveBeenCalledOnce();
  });

  it("does not reject when the teardown itself fails", async () => {
    // The caller is a Tauri event handler; an unhandled rejection there is
    // silent, so the failure must be contained here.
    await expect(
      closeSequence(
        async () => {},
        async () => {
          throw new Error("denied");
        },
      ),
    ).resolves.toBeUndefined();
  });
});

// Global Constraint lock: destroying the window is an IPC command, and Tauri's
// `core:default` set grants only the read-only window getters. Without this
// permission the teardown is rejected by the ACL and the close button does
// nothing at all.
it("grants the window-destroy permission the close button depends on", () => {
  expect(capability.permissions).toContain("core:window:allow-destroy");
});
