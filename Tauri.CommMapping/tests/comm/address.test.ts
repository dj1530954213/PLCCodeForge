import { describe, expect, it } from "vitest";

import { nextAddress, parseHumanAddress, spanForArea } from "../../src/comm/services/address";

describe("comm address", () => {
  it("parses 40001 as Holding start1Based=1", () => {
    expect(parseHumanAddress("40001")).toEqual({
      ok: true,
      area: "Holding",
      start0Based: 0,
      start1Based: 1,
    });
  });

  it("advances by dtype span (UInt16 = 1)", () => {
    const start = "40001";
    expect(spanForArea("Holding", "UInt16")).toBe(1);

    const a1 = start;
    const a2 = nextAddress(a1, "UInt16");
    expect(a2).toEqual({ ok: true, nextHumanAddr: "40002" });

    const a3 = nextAddress((a2 as any).nextHumanAddr, "UInt16");
    expect(a3).toEqual({ ok: true, nextHumanAddr: "40003" });
  });

  it("advances by dtype span (Float32 = 2)", () => {
    const start = "40001";
    expect(spanForArea("Holding", "Float32")).toBe(2);

    const a1 = start;
    const a2 = nextAddress(a1, "Float32");
    expect(a2).toEqual({ ok: true, nextHumanAddr: "40003" });

    const a3 = nextAddress((a2 as any).nextHumanAddr, "Float32");
    expect(a3).toEqual({ ok: true, nextHumanAddr: "40005" });
  });
});
