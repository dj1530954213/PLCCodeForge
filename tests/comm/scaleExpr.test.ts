import { describe, expect, it } from "vitest";

import { compileScaleExpr } from "../../src/comm/services/scaleExpr";

describe("comm scaleExpr", () => {
  it("supports constant number", () => {
    const compiled = compileScaleExpr("2");
    expect(compiled.ok).toBe(true);
    if (!compiled.ok) return;
    expect(compiled.apply(1)).toBe(2);
    expect(compiled.apply(10)).toBe(2);
  });

  it("supports {{x}} placeholder and arithmetic", () => {
    const compiled = compileScaleExpr("{{x}}*10");
    expect(compiled.ok).toBe(true);
    if (!compiled.ok) return;
    expect(compiled.apply(1)).toBe(10);
    expect(compiled.apply(10)).toBe(100);
  });

  it("supports parentheses", () => {
    const compiled = compileScaleExpr("({{x}}+1)*0.5");
    expect(compiled.ok).toBe(true);
    if (!compiled.ok) return;
    expect(compiled.apply(9)).toBe(5);
  });

  it("rejects unsupported placeholders", () => {
    const compiled = compileScaleExpr("{{y}}*10");
    expect(compiled.ok).toBe(false);
    if (compiled.ok) return;
    expect(compiled.message).toContain("{{x}}");
  });
});

