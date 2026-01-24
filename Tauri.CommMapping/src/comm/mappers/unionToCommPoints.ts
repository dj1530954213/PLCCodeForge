import type { CommPoint, CommWarning, PointsV1, ProfilesV1 } from "../api";
import { newPointKey } from "../services/ids";

export interface UnionToCommPointsOutcome {
  points: PointsV1;
  warnings: CommWarning[];
  decisions?: Array<{
    hmiName: string;
    channelName: string;
    deviceId?: number;
    pointKey: string;
    reuseDecision: ReuseDecision;
  }>;
  conflictReport?: {
    generatedAtUtc: string;
    conflicts: Array<{
      keyType: "keyV1" | "keyV2NoDevice";
      hmiName: string;
      channelName?: string;
      pointKeys: string[];
      points: Array<{
        pointKey: string;
        hmiName: string;
        channelName: string;
        deviceId?: number;
      }>;
    }>;
  };
  reusedPointKeys: number;
  createdPointKeys: number;
  skipped: number;
}

export type ReuseDecision = "reused:keyV2" | "reused:keyV2NoDevice" | "reused:keyV1" | "created:new";

function yieldToUi(): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, 0));
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function normalizeText(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function parseDeviceIdFromChannelName(channelName: string): number | undefined {
  const match = channelName.match(/@(\d+)$/);
  if (!match) return undefined;
  const parsed = Number(match[1]);
  if (!Number.isFinite(parsed)) return undefined;
  return parsed;
}

type ChannelDeviceIndex = Map<string, number | "ambiguous">;

function buildChannelDeviceIndex(profiles?: ProfilesV1): ChannelDeviceIndex {
  const out: ChannelDeviceIndex = new Map();
  for (const p of profiles?.profiles ?? []) {
    const channelName = normalizeText(p.channelName);
    if (!channelName) continue;
    const deviceId = p.deviceId;
    if (!Number.isFinite(deviceId)) continue;

    const prev = out.get(channelName);
    if (!prev) {
      out.set(channelName, deviceId);
      continue;
    }
    if (prev === "ambiguous") continue;
    if (prev !== deviceId) out.set(channelName, "ambiguous");
  }
  return out;
}

function resolveDeviceId(channelName: string, index: ChannelDeviceIndex): number | undefined {
  const parsed = parseDeviceIdFromChannelName(channelName);
  if (typeof parsed === "number") return parsed;

  const found = index.get(channelName);
  if (!found || found === "ambiguous") return undefined;
  return found;
}

function buildKeyV2(hmiName: string, channelName: string, deviceId: number | undefined): string {
  return `${hmiName}|${channelName}|${deviceId ?? ""}`;
}

function buildKeyV2NoDevice(hmiName: string, channelName: string): string {
  return `${hmiName}|${channelName}`;
}

function buildKeyV1(hmiName: string): string {
  return hmiName;
}

function buildConflictReport(existing: PointsV1, deviceIndex: ChannelDeviceIndex): UnionToCommPointsOutcome["conflictReport"] {
  const byKeyV1 = new Map<string, Array<{ pointKey: string; hmiName: string; channelName: string; deviceId?: number }>>();
  const byKeyV2NoDevice = new Map<
    string,
    Array<{ pointKey: string; hmiName: string; channelName: string; deviceId?: number }>
  >();

  for (const p of existing.points ?? []) {
    const hmiName = normalizeText(p.hmiName);
    const channelName = normalizeText(p.channelName);
    if (!hmiName || !channelName) continue;
    const deviceId = resolveDeviceId(channelName, deviceIndex);
    const row = { pointKey: p.pointKey, hmiName, channelName, deviceId };

    const listV1 = byKeyV1.get(hmiName) ?? [];
    listV1.push(row);
    byKeyV1.set(hmiName, listV1);

    const keyV2NoDevice = buildKeyV2NoDevice(hmiName, channelName);
    const listV2 = byKeyV2NoDevice.get(keyV2NoDevice) ?? [];
    listV2.push(row);
    byKeyV2NoDevice.set(keyV2NoDevice, listV2);
  }

  const conflicts: NonNullable<UnionToCommPointsOutcome["conflictReport"]>["conflicts"] = [];

  for (const [hmiName, rows] of byKeyV1.entries()) {
    const keys = Array.from(new Set(rows.map((r) => r.pointKey)));
    if (keys.length <= 1) continue;
    conflicts.push({
      keyType: "keyV1",
      hmiName,
      pointKeys: keys,
      points: rows,
    });
  }

  for (const [key, rows] of byKeyV2NoDevice.entries()) {
    const keys = Array.from(new Set(rows.map((r) => r.pointKey)));
    if (keys.length <= 1) continue;
    const [hmiName, channelName] = key.split("|");
    conflicts.push({
      keyType: "keyV2NoDevice",
      hmiName,
      channelName,
      pointKeys: keys,
      points: rows,
    });
  }

  if (conflicts.length === 0) {
    return { generatedAtUtc: new Date().toISOString(), conflicts: [] };
  }

  conflicts.sort((a, b) => a.keyType.localeCompare(b.keyType) || a.hmiName.localeCompare(b.hmiName));
  return { generatedAtUtc: new Date().toISOString(), conflicts };
}

export async function unionToCommPoints(params: {
  imported: PointsV1;
  importedProfiles?: ProfilesV1;
  existing?: PointsV1;
  existingProfiles?: ProfilesV1;
  yieldEvery?: number;
}): Promise<UnionToCommPointsOutcome> {
  const yieldEvery = Math.max(1, Math.floor(params.yieldEvery ?? 500));

  const warnings: CommWarning[] = [];
  const existing = params.existing ?? { schemaVersion: 1, points: [] };

  const existingDeviceIndex = buildChannelDeviceIndex(params.existingProfiles);
  const importedDeviceIndex = buildChannelDeviceIndex(params.importedProfiles);

  const conflictReport = buildConflictReport(existing, existingDeviceIndex);

  const existingPointKeyByKeyV2 = new Map<string, string>();
  const existingPointKeyByKeyV2NoDevice = new Map<string, string>();
  const existingPointKeyByKeyV1 = new Map<string, string>();

  const existingKeyV2NoDeviceConflicts = new Set<string>();
  const existingKeyV1Conflicts = new Set<string>();

  for (const p of existing.points) {
    const hmiName = normalizeText(p.hmiName);
    const channelName = normalizeText(p.channelName);
    if (!hmiName || !channelName) continue;

    const deviceId = resolveDeviceId(channelName, existingDeviceIndex);

    const keyV2 = buildKeyV2(hmiName, channelName, deviceId);
    if (existingPointKeyByKeyV2.has(keyV2)) {
      warnings.push({
        code: "EXISTING_DUPLICATE_KEYV2_FIRST_WINS",
        message: `existing points has duplicated keyV2='${keyV2}', keep first pointKey`,
        hmiName,
        channelName,
        deviceId,
      });
    } else {
      existingPointKeyByKeyV2.set(keyV2, p.pointKey);
    }

    const keyV2NoDevice = buildKeyV2NoDevice(hmiName, channelName);
    const prevV2NoDevice = existingPointKeyByKeyV2NoDevice.get(keyV2NoDevice);
    if (!prevV2NoDevice) {
      existingPointKeyByKeyV2NoDevice.set(keyV2NoDevice, p.pointKey);
    } else if (prevV2NoDevice !== p.pointKey) {
      existingKeyV2NoDeviceConflicts.add(keyV2NoDevice);
    }

    const keyV1 = buildKeyV1(hmiName);
    const prevV1 = existingPointKeyByKeyV1.get(keyV1);
    if (!prevV1) {
      existingPointKeyByKeyV1.set(keyV1, p.pointKey);
    } else if (prevV1 !== p.pointKey) {
      existingKeyV1Conflicts.add(keyV1);
    }
  }

  const usedPointKeys = new Set<string>();
  let reusedPointKeys = 0;
  let createdPointKeys = 0;
  let skipped = 0;

  const outPoints: CommPoint[] = [];
  const decisions: NonNullable<UnionToCommPointsOutcome["decisions"]> = [];
  for (let i = 0; i < params.imported.points.length; i += 1) {
    if (i > 0 && i % yieldEvery === 0) {
      await yieldToUi();
    }

    const src = params.imported.points[i];
    const hmiName = normalizeText(src.hmiName);
    const channelName = normalizeText(src.channelName);
    if (!hmiName) {
      skipped += 1;
      warnings.push({
        code: "IMPORTED_POINT_MISSING_HMI_SKIPPED",
        message: `imported point[${i}] missing hmiName; skipped`,
        channelName,
      });
      continue;
    }
    if (!channelName) {
      skipped += 1;
      warnings.push({
        code: "IMPORTED_POINT_MISSING_CHANNEL_SKIPPED",
        message: `imported point hmiName='${hmiName}' missing channelName; skipped`,
        hmiName,
      });
      continue;
    }
    if (src.dataType === "Unknown") {
      skipped += 1;
      warnings.push({
        code: "IMPORTED_POINT_DATATYPE_UNKNOWN_SKIPPED",
        message: `imported point hmiName='${hmiName}' dataType is Unknown; skipped`,
        hmiName,
        channelName,
      });
      continue;
    }
    if (src.byteOrder === "Unknown") {
      skipped += 1;
      warnings.push({
        code: "IMPORTED_POINT_BYTEORDER_UNKNOWN_SKIPPED",
        message: `imported point hmiName='${hmiName}' byteOrder is Unknown; skipped`,
        hmiName,
        channelName,
      });
      continue;
    }

    const deviceId = resolveDeviceId(channelName, importedDeviceIndex);
    const keyV2 = buildKeyV2(hmiName, channelName, deviceId);
    const keyV2NoDevice = buildKeyV2NoDevice(hmiName, channelName);
    const keyV1 = buildKeyV1(hmiName);

    let pointKey: string | undefined;
    let reuseDecision: ReuseDecision = "created:new";

    // 优先：hmiName + channelName + deviceId
    if (deviceId !== undefined) {
      const candidate = existingPointKeyByKeyV2.get(keyV2);
      if (candidate && !usedPointKeys.has(candidate)) {
        pointKey = candidate;
        reuseDecision = "reused:keyV2";
        reusedPointKeys += 1;
      }
    }

    // 其次：若 deviceId 缺失则降级为 hmiName + channelName（或旧数据无 deviceId 时）
    if (!pointKey) {
      const candidate = existingPointKeyByKeyV2NoDevice.get(keyV2NoDevice);
      if (candidate) {
        if (existingKeyV2NoDeviceConflicts.has(keyV2NoDevice)) {
          warnings.push({
            code: "EXISTING_KEYV2NODEVICE_CONFLICT_NO_FALLBACK",
            message: `existing points has conflict on key(hmiName+channelName)='${keyV2NoDevice}', cannot safely reuse pointKey`,
            hmiName,
            channelName,
            deviceId,
          });
        } else if (!usedPointKeys.has(candidate)) {
          pointKey = candidate;
          reuseDecision = "reused:keyV2NoDevice";
          reusedPointKeys += 1;
          if (deviceId !== undefined && !existingPointKeyByKeyV2.has(keyV2)) {
            warnings.push({
              code: "POINTKEY_REUSE_FALLBACK_NO_DEVICEID_MATCH",
              message: `reused pointKey by (hmiName+channelName) because existing point has no deviceId match; key='${keyV2NoDevice}'`,
              hmiName,
              channelName,
              deviceId,
            });
          }
        }
      }
    }

    // 最后：keyV1=hmiName（仅当旧数据不存在冲突）
    if (!pointKey) {
      const candidate = existingPointKeyByKeyV1.get(keyV1);
      if (candidate) {
        if (existingKeyV1Conflicts.has(keyV1)) {
          warnings.push({
            code: "EXISTING_KEYV1_CONFLICT_NO_FALLBACK",
            message: `existing points has conflict on key(hmiName)='${hmiName}', disable v1 fallback; please provide unique channelName/deviceId`,
            hmiName,
            channelName,
            deviceId,
          });
        } else if (!usedPointKeys.has(candidate)) {
          pointKey = candidate;
          reuseDecision = "reused:keyV1";
          reusedPointKeys += 1;
          warnings.push({
            code: "POINTKEY_REUSE_FALLBACK_HMI_ONLY",
            message: `reused pointKey by hmiName only (v1 fallback) for hmiName='${hmiName}'`,
            hmiName,
            channelName,
            deviceId,
          });
        }
      }
    }

    if (!pointKey || usedPointKeys.has(pointKey)) {
      if (pointKey) {
        warnings.push({
          code: "IMPORTED_DUPLICATE_MATCHKEY_REGEN_POINTKEY",
          message: `imported points has duplicated matchKey, generated a new pointKey for later occurrence (hmiName='${hmiName}', channelName='${channelName}')`,
          hmiName,
          channelName,
          deviceId,
        });
      }
        pointKey = newPointKey();
      reuseDecision = "created:new";
      createdPointKeys += 1;
    }
    usedPointKeys.add(pointKey);

    const scale = isFiniteNumber(src.scale) ? src.scale : 1.0;

    outPoints.push({
      pointKey,
      hmiName,
      dataType: src.dataType,
      byteOrder: src.byteOrder,
      channelName,
      addressOffset: src.addressOffset,
      scale,
    });
    decisions.push({ hmiName, channelName, deviceId, pointKey, reuseDecision });
  }

  return {
    points: { schemaVersion: 1, points: outPoints },
    warnings,
    decisions,
    conflictReport,
    reusedPointKeys,
    createdPointKeys,
    skipped,
  };
}
