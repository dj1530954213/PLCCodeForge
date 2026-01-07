import type { ByteOrder32, CommPoint, DataType, RegisterArea } from "../api";
import { formatHumanAddressFrom0Based, parseHumanAddress, spanForArea, validateAddressRange } from "./address";
import { isValidForArea } from "./dataTypes";

export type BatchAddMode = "increment" | "fixed";

export type BuildBatchPointsParams = {
  channelName: string;
  count: number;
  startAddressHuman: string;
  dataType: DataType;
  byteOrder: ByteOrder32;
  scale: number;
  mode: BatchAddMode;
  profileReadArea: RegisterArea;
  profileStartAddress: number;
  profileLength: number;
  pointKeyFactory?: () => string;
};

export function buildBatchPoints(
  params: BuildBatchPointsParams
): { ok: true; points: CommPoint[]; span: number } | { ok: false; message: string } {
  const count = Math.max(1, Math.min(500, Math.floor(params.count)));
  const startRaw = params.startAddressHuman.trim();
  if (!startRaw) return { ok: false, message: "起始地址不能为空" };

  const parsed = parseHumanAddress(startRaw);
  if (!parsed.ok) return parsed;
  if (parsed.area !== params.profileReadArea) {
    return {
      ok: false,
      message: `地址区域不匹配：连接配置区域为 ${params.profileReadArea}，输入地址区域为 ${parsed.area}`,
    };
  }

  // 验证数据类型与区域的兼容性
  if (!isValidForArea(params.dataType, params.profileReadArea)) {
    return {
      ok: false,
      message: `数据类型 ${params.dataType} 不适用于 ${params.profileReadArea} 区域`,
    };
  }

  const span = spanForArea(params.profileReadArea, params.dataType);
  if (span === null) {
    return { ok: false, message: `dataType=${params.dataType} 与 readArea=${params.profileReadArea} 不匹配` };
  }

  const start0 = parsed.start0Based;
  
  // 使用新的地址范围验证函数
  const rangeValidation = validateAddressRange(
    start0,
    count,
    params.dataType,
    params.profileReadArea,
    params.profileStartAddress,
    params.profileLength
  );
  
  if (!rangeValidation.ok) {
    return rangeValidation;
  }

  const pointKeyFactory = params.pointKeyFactory ?? (() => crypto.randomUUID());
  const points: CommPoint[] = [];
  for (let i = 0; i < count; i++) {
    const rowStart0 = start0 + (params.mode === "increment" ? i * span : 0);
    points.push({
      pointKey: pointKeyFactory(),
      hmiName: "",
      dataType: params.dataType,
      byteOrder: params.byteOrder,
      channelName: params.channelName,
      addressOffset: rowStart0 - params.profileStartAddress,
      scale: Number(params.scale),
    });
  }

  return { ok: true, points, span };
}

export type BatchAddTemplateParams = {
  channelName: string;
  count: number;
  startAddressHuman: string;
  dataType: DataType;
  byteOrder: ByteOrder32;
  mode: BatchAddMode;
  hmiNameTemplate: string;
  scaleTemplate: string; // number or {{i}}
  profileReadArea: RegisterArea;
  profileStartAddress: number;
  profileLength: number;
  pointKeyFactory?: () => string;
};

export type BatchAddPreviewRow = {
  i: number;
  hmiName: string;
  modbusAddress: string;
  dataType: DataType;
  byteOrder: ByteOrder32;
  scale: number;
};

/**
 * 渲染模板字符串
 * 支持占位符：{{number}} 或 {{i}}（从1开始的序号）、{{addr}}（当前地址）
 * 遵循SRP：只负责模板字符串渲染
 * 
 * @param raw 原始模板字符串
 * @param vars 变量对象
 * @returns 渲染后的字符串
 */
function renderTemplate(raw: string, vars: { i: number; addr: string }): string {
  return raw
    .split("{{number}}").join(String(vars.i))
    .split("{{i}}").join(String(vars.i))
    .split("{{addr}}").join(vars.addr);
}

/**
 * 验证模板语法
 * 检查模板中是否只包含支持的占位符
 * 
 * @param template 模板字符串
 * @returns 验证结果
 */
export function validateTemplate(template: string): { ok: true } | { ok: false; message: string } {
  const trimmed = template.trim();
  if (!trimmed) {
    return { ok: false, message: "模板不能为空" };
  }

  // 检查是否包含不支持的占位符
  const placeholderPattern = /\{\{([^}]+)\}\}/g;
  const matches = trimmed.matchAll(placeholderPattern);
  
  for (const match of matches) {
    const placeholder = match[1].trim();
    if (placeholder !== 'number' && placeholder !== 'i' && placeholder !== 'addr') {
      return {
        ok: false,
        message: `不支持的占位符: {{${placeholder}}}。支持的占位符：{{number}}, {{i}}, {{addr}}`,
      };
    }
  }

  return { ok: true };
}

/**
 * 渲染缩放倍数模板
 * 支持固定值（如 "2"）或表达式（如 "{{i}}" 或 "{{number}}"）
 * 
 * @param raw 原始模板字符串
 * @param vars 变量对象
 * @returns 渲染结果
 */
function renderScale(raw: string, vars: { i: number }): { ok: true; value: number } | { ok: false; message: string } {
  const trimmed = raw.trim();
  if (!trimmed) return { ok: false, message: "scale 模板不能为空" };
  
  // 替换所有支持的占位符
  const replaced = trimmed
    .split("{{number}}").join(String(vars.i))
    .split("{{i}}").join(String(vars.i));
  
  // 检查是否还有未替换的占位符
  if (replaced.includes("{{")) {
    return { ok: false, message: "scale 模板仅支持 {{number}} 或 {{i}} 占位符" };
  }
  
  const n = Number(replaced);
  if (!Number.isFinite(n)) {
    return { ok: false, message: `scale 模板不是有效数字: ${trimmed}` };
  }
  
  return { ok: true, value: n };
}

export function buildBatchPointsTemplate(
  params: BatchAddTemplateParams
): { ok: true; points: CommPoint[]; preview: BatchAddPreviewRow[]; span: number } | { ok: false; message: string } {
  // 验证HMI名称模板
  if (!params.hmiNameTemplate.trim()) {
    return { ok: false, message: "变量名称（HMI）模板不能为空" };
  }
  
  const templateValidation = validateTemplate(params.hmiNameTemplate);
  if (!templateValidation.ok) {
    return templateValidation;
  }

  const built = buildBatchPoints({
    channelName: params.channelName,
    count: params.count,
    startAddressHuman: params.startAddressHuman,
    dataType: params.dataType,
    byteOrder: params.byteOrder,
    scale: 1.0,
    mode: params.mode,
    profileReadArea: params.profileReadArea,
    profileStartAddress: params.profileStartAddress,
    profileLength: params.profileLength,
    pointKeyFactory: params.pointKeyFactory,
  });
  if (!built.ok) return built;

  const points: CommPoint[] = [];
  const preview: BatchAddPreviewRow[] = [];

  for (let idx = 0; idx < built.points.length; idx++) {
    const i = idx + 1;
    const point = built.points[idx];
    const start0 = params.profileStartAddress + (point.addressOffset ?? 0);
    const addr = formatHumanAddressFrom0Based(params.profileReadArea, start0);

    const scale = renderScale(params.scaleTemplate, { i });
    if (!scale.ok) return scale;

    const hmiName = renderTemplate(params.hmiNameTemplate, { i, addr });
    points.push({
      ...point,
      hmiName,
      scale: scale.value,
    });
    preview.push({
      i,
      hmiName,
      modbusAddress: addr,
      dataType: point.dataType,
      byteOrder: point.byteOrder,
      scale: scale.value,
    });
  }

  return { ok: true, points, preview, span: built.span };
}

export function previewBatchPointsTemplate(
  params: BatchAddTemplateParams,
  limit = 10
): { ok: true; preview: BatchAddPreviewRow[]; span: number } | { ok: false; message: string } {
  const built = buildBatchPointsTemplate(params);
  if (!built.ok) return built;
  return { ok: true, preview: built.preview.slice(0, Math.max(1, limit)), span: built.span };
}
