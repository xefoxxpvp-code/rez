import { describe, expect, it } from "vitest";
import { aspectLabel, formatResolution } from "./format";

describe("aspectLabel", () => {
  it("reduces common resolutions to their ratio", () => {
    expect(aspectLabel(1440, 1080)).toBe("4:3");
    expect(aspectLabel(1280, 1024)).toBe("5:4");
    expect(aspectLabel(1920, 1080)).toBe("16:9");
  });
  it("keeps 16:10 instead of 8:5", () => {
    expect(aspectLabel(1680, 1050)).toBe("16:10");
  });
});

describe("formatResolution", () => {
  it("uses a multiplication sign with spaces", () => {
    expect(formatResolution(1440, 1080)).toBe("1440 × 1080");
  });
});
