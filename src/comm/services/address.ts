import type { DataType, RegisterArea } from "../api";
import { getRegisterSpan, isValidForArea } from "./dataTypes";

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
// - Holding/Input: 16-bit types => 1 register; 32-bit types => 2 registers; 64-bit types => 4 registers
// - Coil/Discrete: Bool => 1 coil; others unsupported (null)
export function spanForArea(area: RegisterArea, dataType: DataType): number | null {
  if (!isValidForArea(dataType, area)) return null;
  const span = getRegisterSpan(dataType);
  return span > 0 ? span : null;
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


/**
 * 智能推断下一个地址
 * 根据上一行的地址和数据类型，自动计算下一个可用地址
 * 遵循SRP：只负责地址推断逻辑
 * 
 * @param lastRowAddress 上一行的Modbus地址（人类可读格式，如"40001"）
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
  const nextResult = nextAddress(lastRowAddress, lastRowDataType);
  if (nextResult.ok) {
    return nextResult.nextHumanAddr;
  }

  // 如果计算失败，返回连接配置的起始地址
  return formatHumanAddressFrom0Based(profileArea, profileStartAddress);
}

/**
 * 验证地址范围是否有效
 * 检查批量生成的地址是否都在连接配置的范围内
 * 遵循SRP：只负责地址范围验证
 * 
 * @param startAddress 起始地址（0-based）
 * @param count 生成数量
 * @param dataType 数据类型
 * @param profileArea 连接配置的寄存器区域
 * @param profileStartAddress 连接配置的起始地址（0-based）
 * @param profileLength 连接配置的长度
 * @returns 验证结果
 */
export function validateAddressRange(
  startAddress: number,
  count: number,
  dataType: DataType,
  profileArea: RegisterArea,
  profileStartAddress: number,
  profileLength: number
): { ok: true } | { ok: false; message: string } {
  // 验证数据类型与区域的兼容性
  const span = spanForArea(profileArea, dataType);
  if (span === null) {
    return {
      ok: false,
      message: `数据类型 ${dataType} 不适用于 ${profileArea} 区域`,
    };
  }

  // 验证起始地址是否在范围内
  if (startAddress < profileStartAddress) {
    return {
      ok: false,
      message: `起始地址 ${formatHumanAddressFrom0Based(profileArea, startAddress)} 小于连接起始地址 ${formatHumanAddressFrom0Based(profileArea, profileStartAddress)}`,
    };
  }

  // 计算结束地址
  const endAddress = startAddress + span * count;
  const profileEndAddress = profileStartAddress + profileLength;

  // 验证结束地址是否在范围内
  if (endAddress > profileEndAddress) {
    return {
      ok: false,
      message: `地址范围越界：结束地址 ${formatHumanAddressFrom0Based(profileArea, endAddress - 1)} 超出连接范围 ${formatHumanAddressFrom0Based(profileArea, profileEndAddress - 1)}`,
    };
  }

  return { ok: true };
}
