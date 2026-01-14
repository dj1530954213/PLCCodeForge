import { describe, expect, it } from "vitest";

import { nextAddress, parseHumanAddress, spanForArea } from "../../src/comm/services/address";

describe("comm address", () => {
  it("parses 1 as Holding start1Based=1", () => {
    expect(parseHumanAddress("1", "Holding")).toEqual({
      ok: true,
      area: "Holding",
      start0Based: 0,
      start1Based: 1,
    });
  });

  it("advances by dtype span (UInt16 = 1)", () => {
    const start = "1";
    expect(spanForArea("Holding", "UInt16")).toBe(1);

    const a1 = start;
    const a2 = nextAddress(a1, "UInt16", "Holding");
    expect(a2).toEqual({ ok: true, nextHumanAddr: "2" });

    const a3 = nextAddress((a2 as any).nextHumanAddr, "UInt16", "Holding");
    expect(a3).toEqual({ ok: true, nextHumanAddr: "3" });
  });

  it("advances by dtype span (Float32 = 2)", () => {
    const start = "1";
    expect(spanForArea("Holding", "Float32")).toBe(2);

    const a1 = start;
    const a2 = nextAddress(a1, "Float32", "Holding");
    expect(a2).toEqual({ ok: true, nextHumanAddr: "3" });

    const a3 = nextAddress((a2 as any).nextHumanAddr, "Float32", "Holding");
    expect(a3).toEqual({ ok: true, nextHumanAddr: "5" });
  });
});
