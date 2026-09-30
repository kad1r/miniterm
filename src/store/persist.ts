/** Global Constraint: config-write debounce window in milliseconds.
 *  Changing this affects how long a rename can be lost if the window closes
 *  immediately after; the onCloseRequested flush is the primary guard. */
export const SAVE_DEBOUNCE_MS = 300;

export interface Saver<T> {
  schedule(value: T): void;
  flush(): Promise<void>;
  cancel(): void;
}

/**
 * Debounced writer. Back-to-back changes merge into a single write; only
 * the latest value reaches the disk.
 */
export function createSaver<T>(save: (value: T) => Promise<void>, delayMs: number): Saver<T> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: { value: T } | null = null;

  function clear() {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  async function write() {
    if (!pending) return;
    const { value } = pending;
    pending = null;
    await save(value);
  }

  return {
    schedule(value: T) {
      pending = { value };
      clear();
      timer = setTimeout(() => {
        timer = null;
        void write();
      }, delayMs);
    },
    async flush() {
      clear();
      await write();
    },
    cancel() {
      clear();
      pending = null;
    },
  };
}
