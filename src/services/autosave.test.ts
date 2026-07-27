import { afterEach, describe, expect, it, vi } from "vitest";

import { createAutosaveScheduler } from "./autosave";

afterEach(() => {
  vi.useRealTimers();
});

describe("createAutosaveScheduler", () => {
  it("debounces edits by 500 milliseconds", async () => {
    vi.useFakeTimers();
    const save = vi.fn(async () => undefined);
    const autosave = createAutosaveScheduler(save);

    autosave.markDirty();
    await vi.advanceTimersByTimeAsync(300);
    autosave.markDirty();
    await vi.advanceTimersByTimeAsync(499);
    expect(save).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(save).toHaveBeenCalledOnce();
  });

  it("forces a save within five seconds of continuous edits", async () => {
    vi.useFakeTimers();
    const save = vi.fn(async () => undefined);
    const autosave = createAutosaveScheduler(save);

    autosave.markDirty();
    for (let elapsed = 0; elapsed < 4_500; elapsed += 300) {
      await vi.advanceTimersByTimeAsync(300);
      autosave.markDirty();
    }
    await vi.advanceTimersByTimeAsync(500);
    expect(save).toHaveBeenCalledOnce();
  });
});
