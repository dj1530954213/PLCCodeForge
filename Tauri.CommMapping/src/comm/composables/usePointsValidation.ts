import { computed, type ComputedRef, type Ref } from "vue";

import type {
  CommProjectDataV1,
  ConnectionProfile,
  DataType,
  PointsV1,
  Quality,
} from "../api";
import { parseHumanAddress, spanForArea } from "../services/address";

export type PointRowLike = {
  pointKey: string;
  hmiName: string;
  modbusAddress: string;
  dataType: DataType;
  scale?: string | number | null;
};

export interface ValidationIssue {
  pointKey: string;
  hmiName: string;
  modbusAddress: string;
  message: string;
  field?: string;
}

export interface UsePointsValidationOptions {
  gridRows: Ref<PointRowLike[]>;
  points: Ref<PointsV1>;
  project: Ref<CommProjectDataV1 | null>;
  activeDeviceId: Ref<string>;
  activeProfile: ComputedRef<ConnectionProfile | null>;
}

const FIELD_LABEL_MAP: Record<string, string> = {
  hmiName: "变量名称（HMI）",
  modbusAddress: "点位地址",
  dataType: "数据类型",
  byteOrder: "字节序",
  scale: "缩放倍数",
  channelName: "通道名称",
  pointKey: "pointKey（稳定键）",
  "profiles.channelName": "连接通道名称",
};

export function formatFieldLabel(field?: string): string {
  if (!field) return "未知字段";
  return FIELD_LABEL_MAP[field] ?? field;
}

export function formatBackendReason(reason?: string): string {
  if (!reason) return "未知原因";
  const trimmed = reason.trim();
  if (!trimmed) return "未知原因";
  if (/[\u4e00-\u9fa5]/.test(trimmed)) return trimmed;
  if (trimmed === "duplicate") return "重复";
  if (trimmed === "empty") return "不能为空";
  if (trimmed === "Unknown") return "未知";
  if (trimmed === "not finite") return "不是有效数字";
  if (trimmed === "dataType/readArea mismatch") return "数据类型与读取区域不匹配";
  if (trimmed === "address conflict") return "地址冲突";
  if (trimmed === "out of range") return "地址超出连接范围";

  const duplicateChannel = trimmed.match(/^duplicate channelName:\s*(.+)$/i);
  if (duplicateChannel) return `通道名称重复：${duplicateChannel[1]}`;

  const unknownChannel = trimmed.match(/^unknown channelName:\s*(.+)$/i);
  if (unknownChannel) return `未知通道名称：${unknownChannel[1]}`;

  const duplicateWith = trimmed.match(/^duplicate with\s*(.+)$/i);
  if (duplicateWith) return `与${duplicateWith[1]}重复`;

  return trimmed;
}

export function formatQualityLabel(quality?: Quality | string | null): string {
  switch (quality) {
    case "Ok":
      return "正常";
    case "Timeout":
      return "超时";
    case "CommError":
      return "通讯错误";
    case "DecodeError":
      return "解析错误";
    case "ConfigError":
      return "配置错误";
    case "":
    case null:
    case undefined:
      return "";
    default:
      return String(quality);
  }
}

function normalizeHmiName(name: string): string {
  return String(name ?? "").trim();
}

export function usePointsValidation(options: UsePointsValidationOptions) {
  const { gridRows, points, project, activeDeviceId, activeProfile } = options;

  const hmiDuplicateByPointKey = computed<Record<string, string>>(() => {
    const out: Record<string, string> = {};
    const devices = project.value?.devices ?? [];
    if (devices.length === 0) return out;

    const byName = new Map<
      string,
      Array<{ deviceId: string; deviceName: string; pointKey: string }>
    >();

    for (const device of devices) {
      const devicePoints =
        device.deviceId === activeDeviceId.value ? points.value.points : device.points.points;
      for (const point of devicePoints) {
        const name = normalizeHmiName(point.hmiName);
        if (!name) continue;
        const list = byName.get(name) ?? [];
        list.push({
          deviceId: device.deviceId,
          deviceName: device.deviceName,
          pointKey: point.pointKey,
        });
        byName.set(name, list);
      }
    }

    for (const list of byName.values()) {
      if (list.length < 2) continue;
      const deviceLabel = Array.from(new Set(list.map((v) => v.deviceName))).join(" / ");
      const message = `HMI 重名（跨设备）：${deviceLabel}`;
      for (const item of list) {
        if (item.deviceId === activeDeviceId.value) {
          out[item.pointKey] = message;
        }
      }
    }

    return out;
  });

  const addressConflictByPointKey = computed<Record<string, string>>(() => {
    const out: Record<string, string> = {};
    const profile = activeProfile.value;
    if (!profile) return out;

    type Segment = { pointKey: string; start: number; end: number };
    const segments: Segment[] = [];

    for (const row of gridRows.value) {
      const addrRaw = String(row.modbusAddress ?? "").trim();
      if (!addrRaw) continue;
      const parsed = parseHumanAddress(addrRaw, profile.readArea);
      if (!parsed.ok) continue;
      const span = spanForArea(profile.readArea, row.dataType);
      if (span === null) continue;
      const start = parsed.start0Based;
      const end = start + span;
      segments.push({ pointKey: row.pointKey, start, end });
    }

    for (let i = 0; i < segments.length; i++) {
      for (let j = i + 1; j < segments.length; j++) {
        if (segments[i].start < segments[j].end && segments[j].start < segments[i].end) {
          out[segments[i].pointKey] = "地址冲突";
          out[segments[j].pointKey] = "地址冲突";
        }
      }
    }

    return out;
  });

  function validateHmiName(row: PointRowLike): string | null {
    return normalizeHmiName(row.hmiName) ? null : "变量名称（HMI）不能为空";
  }

  function validateScale(row: PointRowLike): string | null {
    const raw = String(row.scale ?? "").trim();
    if (!raw) return "缩放倍数不能为空";
    return Number.isFinite(Number(raw)) ? null : "缩放倍数必须为有效数字";
  }

  function validateModbusAddress(row: PointRowLike): string | null {
    const profile = activeProfile.value;
    if (!profile) return "请先选择连接";

    const len = spanForArea(profile.readArea, row.dataType);
    if (len === null) return `数据类型 ${row.dataType} 与读取区域 ${profile.readArea} 不匹配`;

    const addrRaw = String(row.modbusAddress ?? "").trim();
    if (!addrRaw) return null;

    const parsed = parseHumanAddress(addrRaw, profile.readArea);
    if (!parsed.ok) return parsed.message;
    return null;
  }

  function validateRowForRunDetailed(
    row: PointRowLike
  ): { message: string; field?: keyof PointRowLike } | null {
    if (!activeProfile.value) return { message: "请先选择连接" };

    const hmiErr = validateHmiName(row) ?? hmiDuplicateByPointKey.value[row.pointKey];
    if (hmiErr) return { message: hmiErr, field: "hmiName" };

    const scaleErr = validateScale(row);
    if (scaleErr) return { message: scaleErr, field: "scale" };

    const addrErr = validateModbusAddress(row) ?? addressConflictByPointKey.value[row.pointKey];
    if (addrErr) return { message: addrErr, field: "modbusAddress" };

    return null;
  }

  function validateRowForRun(row: PointRowLike): string | null {
    return validateRowForRunDetailed(row)?.message ?? null;
  }

  const validationIssues = computed<ValidationIssue[]>(() => {
    const out: ValidationIssue[] = [];
    for (const row of gridRows.value) {
      const result = validateRowForRunDetailed(row);
      if (!result) continue;
      out.push({
        pointKey: row.pointKey,
        hmiName: row.hmiName,
        modbusAddress: row.modbusAddress,
        message: result.message,
        field: result.field,
      });
    }
    return out;
  });

  const validationIssuesView = computed(() =>
    validationIssues.value.map((issue) => ({
      ...issue,
      fieldLabel: formatFieldLabel(issue.field),
    }))
  );

  const validationIssueByPointKey = computed<Record<string, string>>(() => {
    const out: Record<string, string> = {};
    for (const issue of validationIssues.value) {
      out[issue.pointKey] = issue.message;
    }
    return out;
  });

  const hasValidationIssues = computed(() => validationIssues.value.length > 0);

  return {
    addressConflictByPointKey,
    hmiDuplicateByPointKey,
    validateHmiName,
    validateScale,
    validateModbusAddress,
    validateRowForRunDetailed,
    validateRowForRun,
    validationIssues,
    validationIssuesView,
    validationIssueByPointKey,
    hasValidationIssues,
  };
}
