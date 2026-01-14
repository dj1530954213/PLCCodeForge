import { describe, expect, it } from "vitest";

import { buildBatchPoints } from "../../src/comm/services/batchAdd";
import { formatHumanAddressFrom0Based } from "../../src/comm/services/address";

describe("comm batchAdd", () => {
  it("Holding + UInt16: 1 start, 3 rows => 1/2/3", () => {
    const built = buildBatchPoints({
      channelName: "tcp-1",
      count: 3,
      startAddressHuman: "1",
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
    expect(addrs).toEqual(["1", "2", "3"]);
  });

  it("Holding + Float32: 1 start, 3 rows => 1/3/5; byteOrder applies to all", () => {
    const built = buildBatchPoints({
      channelName: "tcp-1",
      count: 3,
      startAddressHuman: "1",
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
    expect(addrs).toEqual(["1", "3", "5"]);
    expect(built.points.map((p) => p.byteOrder)).toEqual(["DCBA", "DCBA", "DCBA"]);
  });
});
