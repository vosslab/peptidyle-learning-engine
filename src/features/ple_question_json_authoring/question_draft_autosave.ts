/** Coordinates debounced Draft writes without interpreting or validating backend source. */
export type QuestionDraftAutosaveStatus = "saved" | "unsaved" | "saving" | "error";

export type QuestionDraftAutosaveState = {
  readonly status: QuestionDraftAutosaveStatus;
  readonly generation: number;
  readonly acknowledgedGeneration: number;
  readonly error?: unknown;
};

export type QuestionDraftAutosaveOptions<T> = {
  readonly save: (snapshot: T) => Promise<unknown>;
  readonly debounceMs?: number;
  readonly onStateChange?: (state: QuestionDraftAutosaveState) => void;
};

/**
 * Debounces edits, sends only one write at a time, and retains the newest snapshot after failure.
 * The source may be empty or invalid; validation belongs to preview and publication.
 */
export function createQuestionDraftAutosave<T>(options: QuestionDraftAutosaveOptions<T>): {
  readonly edit: (snapshot: T) => void;
  readonly reset: (snapshot: T) => void;
  readonly flush: () => Promise<void>;
  readonly state: () => QuestionDraftAutosaveState;
  readonly dispose: () => void;
} {
  const debounceMs = options.debounceMs ?? 500;
  if (!Number.isSafeInteger(debounceMs) || debounceMs < 0)
    throw new Error("Draft autosave debounce must be a non-negative integer");

  let generation = 0;
  let acknowledgedGeneration = 0;
  let latestSnapshot: T | undefined;
  let hasSnapshot = false;
  let lastError: unknown;
  let savePromise: Promise<void> | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let disposed = false;

  function state(): QuestionDraftAutosaveState {
    const status: QuestionDraftAutosaveStatus =
      savePromise !== undefined
        ? "saving"
        : lastError !== undefined
          ? "error"
          : generation === acknowledgedGeneration
            ? "saved"
            : "unsaved";
    return { status, generation, acknowledgedGeneration, error: lastError };
  }

  function notify(): void {
    if (!disposed) options.onStateChange?.(state());
  }

  async function drain(): Promise<void> {
    if (!hasSnapshot || generation <= acknowledgedGeneration) return;
    const writeGeneration = generation;
    const snapshot = latestSnapshot as T;
    try {
      await options.save(snapshot);
    } catch (error: unknown) {
      lastError = error;
      notify();
      throw error;
    }
    acknowledgedGeneration = writeGeneration;
    lastError = undefined;
    notify();
  }

  function flush(): Promise<void> {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
    if (savePromise !== undefined) return savePromise.then(() => flush());
    if (!hasSnapshot || generation === acknowledgedGeneration) return Promise.resolve();

    const operation = Promise.resolve().then(drain);
    savePromise = operation;
    notify();
    return operation.finally(() => {
      if (savePromise === operation) savePromise = undefined;
      notify();
    });
  }

  function edit(snapshot: T): void {
    if (disposed) throw new Error("Draft autosave has been disposed");
    generation += 1;
    latestSnapshot = snapshot;
    hasSnapshot = true;
    lastError = undefined;
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      void flush().catch(() => undefined);
    }, debounceMs);
    notify();
  }

  /** Replaces queued state with a freshly loaded server snapshot after a deliberate reload. */
  function reset(snapshot: T): void {
    if (savePromise !== undefined) throw new Error("Cannot reset Draft autosave during a write");
    if (timer !== undefined) clearTimeout(timer);
    timer = undefined;
    generation += 1;
    acknowledgedGeneration = generation;
    latestSnapshot = snapshot;
    hasSnapshot = true;
    lastError = undefined;
    notify();
  }

  function dispose(): void {
    disposed = true;
    if (timer !== undefined) clearTimeout(timer);
    timer = undefined;
  }

  return { edit, reset, flush, state, dispose };
}
