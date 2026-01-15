import type { DataType, RegisterArea } from "../api";
import { getRegisterSpan, isValidForArea } from "./dataTypes";

export type HumanAddress = {
  area: RegisterArea;
  start1Based: number;
};

export type ParseHumanAddressResult =
  | { ok: true; area: RegisterArea; start1Based: number; start0Based: number }
  | { ok: false; message: string };

const MAX_ADDRESS_1_BASED = 65536;

export function modbusHumanBase(_area: RegisterArea): number {
  return 1;
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

export function parseHumanAddress(input: string, area: RegisterArea): ParseHumanAddressResult {
  const raw = input.trim();
  if (!raw) return { ok: false, message: "地址不能为空" };
  if (!/^[0-9]+$/.test(raw)) return { ok: false, message: "地址必须为纯数字（例如 1）" };

  const n = Number(raw);
  if (!Number.isFinite(n) || n <= 0) return { ok: false, message: "地址必须为正整数（从 1 开始）" };
  if (n > MAX_ADDRESS_1_BASED) return { ok: false, message: `地址不能超过 ${MAX_ADDRESS_1_BASED}` };

  const start1Based = Math.floor(n);
  const start0Based = start1Based - 1;
  return { ok: true, area, start0Based, start1Based };
}

// Span (unit length) derived from dataType, for Modbus address advancement.
// - Holding/Input: 16-bit types => 1 register; 32-bit types => 2 registers
// - Coil/Discrete: Bool => 1 coil; others unsupported (null)
export function spanForArea(area: RegisterArea, dataType: DataType): number | null {
  if (!isValidForArea(dataType, area)) return null;
  const span = getRegisterSpan(dataType);
  return span > 0 ? span : null;
}

export function nextAddress(
  currentHumanAddr: string,
  dataType: DataType,
  area: RegisterArea
): { ok: true; nextHumanAddr: string } | { ok: false; message: string } {
  const parsed = parseHumanAddress(currentHumanAddr, area);
  if (!parsed.ok) return parsed;
  const span = spanForArea(parsed.area, dataType);
  if (span === null) {
    return { ok: false, message: `数据类型 ${dataType} 与读取区域 ${parsed.area} 不匹配` };
  }
  return { ok: true, nextHumanAddr: formatHumanAddressFrom0Based(parsed.area, parsed.start0Based + span) };
}

/**
 * 智能推断下一个地址
 * 根据上一行的地址和数据类型，自动计算下一个可用地址
 * 遵循SRP：只负责地址推断逻辑
 * 
 * @param lastRowAddress 上一行的地址（人类可读格式，1-based）
 * @param lastRowDataType 上一行的数据类型
 * @param profileArea 连接配置的寄存器区域
 * @param profileStartAddress 连接配置的起始地址（0-based）
 * @returns 推断的下一个地址（人类可读格式）或错误信息
 */
export function inferNextAddress(
  lastRowAddress: string | null | undefined,
  lastRowDataType: DataType | null | undefined,
  profileArea: RegisterArea,
  profileStartAddress: number
): string {
  // 如果没有上一行数据，返回连接配置的起始地址
  if (!lastRowAddress || !lastRowDataType) {
    return formatHumanAddressFrom0Based(profileArea, profileStartAddress);
  }

  // 尝试计算下一个地址
  const nextResult = nextAddress(lastRowAddress, lastRowDataType, profileArea);
  if (nextResult.ok) {
    return nextResult.nextHumanAddr;
  }

  // 如果计算失败，返回连接配置的起始地址
  return formatHumanAddressFrom0Based(profileArea, profileStartAddress);
}
