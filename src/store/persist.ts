export interface Saver<T> {
  schedule(value: T): void;
  flush(): Promise<void>;
  cancel(): void;
}

/**
 * Debounce'lu yazıcı. Ard arda gelen değişiklikler tek bir yazmada birleşir;
 * yalnız en son değer diske gider.
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
