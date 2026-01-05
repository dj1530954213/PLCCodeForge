import type { ByteOrder32, CommPoint, DataType, RegisterArea } from "../api";
import { formatHumanAddressFrom0Based, parseHumanAddress, spanForArea } from "./address";

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
      message: `地址区域不匹配：profile.readArea=${params.profileReadArea}，输入=${parsed.area}`,
    };
  }

  const span = spanForArea(params.profileReadArea, params.dataType);
  if (span === null) {
    return { ok: false, message: `dataType=${params.dataType} 与 readArea=${params.profileReadArea} 不匹配` };
  }

  const start0 = parsed.start0Based;
  if (start0 < params.profileStartAddress) {
    return { ok: false, message: "地址小于连接起始地址" };
  }

  const channelEnd0 = params.profileStartAddress + params.profileLength;
  const end0 =
    params.mode === "increment"
      ? start0 + span * count
      : start0 + span;
  if (end0 > channelEnd0) {
    return { ok: false, message: `地址越界：end=${end0} > channelEnd=${channelEnd0}` };
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

function renderTemplate(raw: string, vars: { i: number; addr: string }): string {
  return raw.split("{{i}}").join(String(vars.i)).split("{{addr}}").join(vars.addr);
}

function renderScale(raw: string, vars: { i: number }): { ok: true; value: number } | { ok: false; message: string } {
  const trimmed = raw.trim();
  if (!trimmed) return { ok: false, message: "scale 模板不能为空" };
  const replaced = trimmed.split("{{i}}").join(String(vars.i));
  if (replaced.includes("{{")) return { ok: false, message: "scale 模板仅支持 {{i}} 占位符" };
  const n = Number(replaced);
  if (!Number.isFinite(n)) return { ok: false, message: `scale 模板不是有效数字: ${trimmed}` };
  return { ok: true, value: n };
}

export function buildBatchPointsTemplate(
  params: BatchAddTemplateParams
): { ok: true; points: CommPoint[]; preview: BatchAddPreviewRow[]; span: number } | { ok: false; message: string } {
  if (!params.hmiNameTemplate.trim()) {
    return { ok: false, message: "变量名称（HMI）模板不能为空" };
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
