import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createSaver, SAVE_DEBOUNCE_MS } from "./persist";

// Global Constraint lock: SAVE_DEBOUNCE_MS controls the window in which a config
// change can be lost if the app closes before onCloseRequested fires. Any change
// must be deliberate — raise this test with it.
it("SAVE_DEBOUNCE_MS is 300", () => {
  expect(SAVE_DEBOUNCE_MS).toBe(300)
})

describe("createSaver", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("coalesces rapid changes into a single write", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const saver = createSaver(save, 300);

    saver.schedule("a");
    saver.schedule("b");
    saver.schedule("c");
    expect(save).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(300);
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith("c");
  });

  it("writes again after the window closes", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const saver = createSaver(save, 300);

    saver.schedule("a");
    await vi.advanceTimersByTimeAsync(300);
    saver.schedule("b");
    await vi.advanceTimersByTimeAsync(300);

    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith("b");
  });

  it("flush writes immediately and cancels the pending timer", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const saver = createSaver(save, 300);

    saver.schedule("a");
    await saver.flush();
    expect(save).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(300);
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("flush is a no-op when nothing is pending", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const saver = createSaver(save, 300);
    await saver.flush();
    expect(save).not.toHaveBeenCalled();
  });

  it("cancel drops the pending write", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const saver = createSaver(save, 300);

    saver.schedule("a");
    saver.cancel();
    await vi.advanceTimersByTimeAsync(300);
    expect(save).not.toHaveBeenCalled();
  });
});
