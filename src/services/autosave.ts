export interface AutosaveScheduler {
  markDirty(): void;
  flush(): Promise<void>;
  dispose(): void;
}

interface AutosaveTiming {
  debounceMs: number;
  forceMs: number;
}

const DEFAULT_TIMING: AutosaveTiming = {
  debounceMs: 500,
  forceMs: 5_000,
};

export function createAutosaveScheduler(
  save: () => Promise<void>,
  timing: AutosaveTiming = DEFAULT_TIMING,
): AutosaveScheduler {
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let forceTimer: ReturnType<typeof setTimeout> | null = null;
  let dirty = false;
  let disposed = false;
  let activeSave: Promise<void> | null = null;

  function clearTimers(): void {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    if (forceTimer !== null) {
      clearTimeout(forceTimer);
      forceTimer = null;
    }
  }

  async function runSave(): Promise<void> {
    clearTimers();
    if (!dirty || disposed) {
      return activeSave ?? Promise.resolve();
    }
    if (activeSave) {
      return activeSave;
    }

    dirty = false;
    activeSave = save().finally(() => {
      activeSave = null;
      if (dirty && !disposed) {
        schedule();
      }
    });
    return activeSave;
  }

  function schedule(): void {
    if (disposed) {
      return;
    }
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => {
      void runSave();
    }, timing.debounceMs);
    forceTimer ??= setTimeout(() => {
      void runSave();
    }, timing.forceMs);
  }

  return {
    markDirty(): void {
      dirty = true;
      schedule();
    },
    async flush(): Promise<void> {
      if (activeSave) {
        await activeSave;
      }
      await runSave();
      if (activeSave) {
        await activeSave;
      }
    },
    dispose(): void {
      disposed = true;
      clearTimers();
    },
  };
}
