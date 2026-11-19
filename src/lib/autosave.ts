/** Debounced save that can be flushed (switching items) or cancelled (deleting the item). */
export function createAutosave<T>(save: (value: T) => unknown, delayMs: number) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let value: T | null = null;

  const clear = () => {
    if (timer !== undefined) clearTimeout(timer);
    timer = undefined;
  };

  const flush = () => {
    clear();
    if (value !== null) {
      const v = value;
      value = null;
      save(v);
    }
  };

  return {
    schedule(v: T) {
      value = v;
      clear();
      timer = setTimeout(flush, delayMs);
    },
    flush,
    cancel() {
      clear();
      value = null;
    },
    pending(): T | null {
      return value;
    },
  };
}
