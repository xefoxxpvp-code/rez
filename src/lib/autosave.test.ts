import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createAutosave } from "./autosave";

describe("createAutosave", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("saves the last value after the delay", () => {
    const save = vi.fn();
    const a = createAutosave(save, 400);
    a.schedule("x");
    a.schedule("y");
    vi.advanceTimersByTime(399);
    expect(save).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(save).toHaveBeenCalledExactlyOnceWith("y");
  });

  it("flush saves the pending value now, once", () => {
    const save = vi.fn();
    const a = createAutosave(save, 400);
    a.schedule("x");
    a.flush();
    vi.advanceTimersByTime(1000);
    expect(save).toHaveBeenCalledExactlyOnceWith("x");
  });

  it("cancel drops the pending value", () => {
    const save = vi.fn();
    const a = createAutosave(save, 400);
    a.schedule("x");
    expect(a.pending()).toBe("x");
    a.cancel();
    vi.advanceTimersByTime(1000);
    expect(save).not.toHaveBeenCalled();
    expect(a.pending()).toBeNull();
  });
});
