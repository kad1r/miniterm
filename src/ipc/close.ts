/**
 * Ordering for a user-initiated window close: save, then tear down.
 *
 * Both steps are contained. The handler that calls this has already called
 * `preventDefault()` on the OS close, so anything that escapes here leaves a
 * window the user cannot get rid of — a worse outcome than a lost config write.
 */
export async function closeSequence(
  flush: () => Promise<void>,
  destroy: () => Promise<void>,
): Promise<void> {
  try {
    await flush();
  } catch {
    // A failed save must never trap the user in the app.
  }
  try {
    await destroy();
  } catch {
    // Nothing left to fall back on; swallowing keeps the rejection from
    // vanishing into the event handler as an unhandled promise.
  }
}
