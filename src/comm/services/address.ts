import type { DataType, RegisterArea } from "../api";

export type HumanAddress = {
  area: RegisterArea;
  start1Based: number;
};

export type ParseHumanAddressResult =
  | { ok: true; area: RegisterArea; start1Based: number; start0Based: number }
  | { ok: false; message: string };

export function modbusHumanBase(area: RegisterArea): number {
  switch (area) {
    case "Coil":
      return 1;
    case "Discrete":
      return 10001;
    case "Input":
      return 30001;
    case "Holding":
      return 40001;
    default:
      return 40001;
  }
}

export function formatHumanAddress(addr: HumanAddress): string {
  const start1Based = Math.floor(Number(addr.start1Based));
  const start0Based = Math.max(0, start1Based - 1);
  return formatHumanAddressFrom0Based(addr.area, start0Based);
}

export function formatHumanAddressFrom0Based(area: RegisterArea, start0Based: number): string {
  const base = modbusHumanBase(area);
  const n = base + Math.max(0, Math.floor(Number(start0Based)));
  return String(n);
}

export function parseHumanAddress(input: string): ParseHumanAddressResult {
  const raw = input.trim();
  if (!raw) return { ok: false, message: "地址不能为空" };
  if (!/^[0-9]+$/.test(raw)) return { ok: false, message: "地址必须为纯数字（例如 40001）" };

  const n = Number(raw);
  if (!Number.isFinite(n) || n <= 0) return { ok: false, message: "地址必须为正整数" };

  const area: RegisterArea =
    n >= 40001 ? "Holding" : n >= 30001 ? "Input" : n >= 10001 ? "Discrete" : "Coil";
  const base = modbusHumanBase(area);
  const start0Based = n - base;
  const start1Based = start0Based + 1;
  return { ok: true, area, start0Based, start1Based };
}

// Span (unit length) derived from dataType, for Modbus address advancement.
// - Holding/Input: 16-bit types => 1 register; 32-bit types => 2 registers
// - Coil/Discrete: Bool => 1 coil; others unsupported (null)
export function spanForArea(area: RegisterArea, dataType: DataType): number | null {
  if (area === "Holding" || area === "Input") {
    if (dataType === "Int16" || dataType === "UInt16") return 1;
    if (dataType === "Int32" || dataType === "UInt32" || dataType === "Float32") return 2;
    return null;
  }
  if (area === "Coil" || area === "Discrete") {
    if (dataType === "Bool") return 1;
    return null;
  }
  return null;
}

export function dtypeRegisterSpan(dataType: DataType): 1 | 2 {
  return dataType === "Int32" || dataType === "UInt32" || dataType === "Float32" ? 2 : 1;
}

export function nextAddress(
  currentHumanAddr: string,
  dataType: DataType
): { ok: true; nextHumanAddr: string } | { ok: false; message: string } {
  const parsed = parseHumanAddress(currentHumanAddr);
  if (!parsed.ok) return parsed;
  const span = spanForArea(parsed.area, dataType);
  if (span === null) {
    return { ok: false, message: `dataType=${dataType} 与 readArea=${parsed.area} 不匹配` };
  }
  return { ok: true, nextHumanAddr: formatHumanAddressFrom0Based(parsed.area, parsed.start0Based + span) };
}

