import { describe, expect, it } from "vitest";

import { buildBatchPoints } from "../../src/comm/services/batchAdd";
import { formatHumanAddressFrom0Based } from "../../src/comm/services/address";

describe("comm batchAdd", () => {
  it("Holding + UInt16: 40001 start, 3 rows => 40001/40002/40003", () => {
    const built = buildBatchPoints({
      channelName: "tcp-1",
      count: 3,
      startAddressHuman: "40001",
      dataType: "UInt16",
      byteOrder: "ABCD",
      scale: 1.0,
      mode: "increment",
      profileReadArea: "Holding",
      profileStartAddress: 0,
      profileLength: 100,
      pointKeyFactory: (() => {
        let i = 0;
        return () => `p-${++i}`;
      })(),
    });
    expect(built.ok).toBe(true);
    if (!built.ok) return;

    const addrs = built.points.map((p) => formatHumanAddressFrom0Based("Holding", 0 + (p.addressOffset ?? 0)));
    expect(addrs).toEqual(["40001", "40002", "40003"]);
  });

  it("Holding + Float32: 40001 start, 3 rows => 40001/40003/40005; byteOrder applies to all", () => {
    const built = buildBatchPoints({
      channelName: "tcp-1",
      count: 3,
      startAddressHuman: "40001",
      dataType: "Float32",
      byteOrder: "DCBA",
      scale: 1.0,
      mode: "increment",
      profileReadArea: "Holding",
      profileStartAddress: 0,
      profileLength: 100,
      pointKeyFactory: (() => {
        let i = 0;
        return () => `p-${++i}`;
      })(),
    });
    expect(built.ok).toBe(true);
    if (!built.ok) return;

    const addrs = built.points.map((p) => formatHumanAddressFrom0Based("Holding", 0 + (p.addressOffset ?? 0)));
    expect(addrs).toEqual(["40001", "40003", "40005"]);
    expect(built.points.map((p) => p.byteOrder)).toEqual(["DCBA", "DCBA", "DCBA"]);
  });
});

