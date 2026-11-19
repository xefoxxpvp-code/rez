import { describe, expect, it } from "vitest";
import { detectCodeKind } from "./crosshair";

describe("detectCodeKind", () => {
  it("recognises each supported code", () => {
    expect(detectCodeKind("REZ1-eyJjb2xvciI6IiMwMEZGRkYifQ")).toBe("rez");
    expect(detectCodeKind("CSGO-Cn37R-YE7vo-pLCAL-aURmZ-z6zkG")).toBe("cs2");
    expect(detectCodeKind("CSvbPubOq37zTGqtsPTP5QTrp5CB4xFXiKRLfzJsm49ZRe")).toBe("cs2");
    expect(detectCodeKind("  0;P;c;5;h;0;0l;4  ")).toBe("valorant");
  });

  it("returns null for anything else", () => {
    expect(detectCodeKind("")).toBeNull();
    expect(detectCodeKind("bonjour")).toBeNull();
    expect(detectCodeKind("CS-short")).toBeNull();
  });
});
