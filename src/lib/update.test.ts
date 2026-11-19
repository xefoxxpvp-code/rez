import { describe, expect, it } from "vitest";
import { progressLabel, updateLabel } from "./update";

describe("update labels", () => {
  it("names the available version", () => {
    expect(updateLabel("0.3.0")).toBe("Mise à jour 0.3.0 disponible");
  });
  it("shows download progress as a percentage when the size is known", () => {
    expect(progressLabel(512, 1024)).toBe("Téléchargement… 50 %");
    expect(progressLabel(512, undefined)).toBe("Téléchargement…");
  });
});
