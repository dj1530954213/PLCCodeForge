import { ref, type ComputedRef, type Ref } from "vue";

import type { CommPoint, ConnectionProfile, PointsV1, ProfilesV1, SampleResult } from "../api";
import { formatHumanAddressFrom0Based, parseHumanAddress } from "../services/address";
import { buildPlan } from "../services/run";
import { formatQualityLabel } from "./usePointsValidation";

export type PointRowLike = CommPoint & {
  __selected: boolean;
  modbusAddress: string;
  quality: string;
  valueDisplay: string;
  errorMessage: string;
  timestamp: string;
  durationMs: number | "";
};

export interface UsePointsRowsOptions<T extends PointRowLike> {
  gridRows: Ref<T[]>;
  points: Ref<PointsV1>;
  activeChannelName: Ref<string>;
  activeProfile: ComputedRef<ConnectionProfile | null>;
  projectId: Ref<string>;
  activeDeviceId: Ref<string>;
  gridApi: () => any;
  colIndexByProp: ComputedRef<Record<string, number>>;
  onTouched?: (keys: string[]) => void;
}

export function usePointsRows<T extends PointRowLike>(options: UsePointsRowsOptions<T>) {
  const {
    gridRows,
    points,
    activeChannelName,
    activeProfile,
    projectId,
    activeDeviceId,
    gridApi,
    colIndexByProp,
    onTouched,
  } = options;

  const start0ByPointKey = ref<Record<string, number>>({});
  const runtimeByPointKey = ref<Record<string, SampleResult>>({});

  function makeRowFromPoint(point: CommPoint): T {
    const profile = activeProfile.value;
    let addr = "";
    if (profile && profile.channelName === point.channelName) {
      if (typeof point.addressOffset === "number") {
        addr = formatHumanAddressFrom0Based(profile.readArea, profile.startAddress + point.addressOffset);
      } else if (start0ByPointKey.value[point.pointKey] !== undefined) {
        addr = formatHumanAddressFrom0Based(profile.readArea, start0ByPointKey.value[point.pointKey]);
      }
    }
    const runtime = runtimeByPointKey.value[point.pointKey];

    // 保留现有的选中状态
    const existingRow = gridRows.value.find((row) => row.pointKey === point.pointKey);
    const isSelected = existingRow?.__selected ?? false;

    return {
      ...(point as T),
      __selected: isSelected,
      modbusAddress: addr,
      quality: formatQualityLabel(runtime?.quality),
      valueDisplay: runtime?.valueDisplay ?? "",
      errorMessage: runtime?.errorMessage ?? "",
      timestamp: runtime?.timestamp ?? "",
      durationMs: runtime?.durationMs ?? "",
    } as T;
  }

  function rebuildGridRows() {
    const channel = activeChannelName.value;
    gridRows.value = points.value.points.filter((p) => p.channelName === channel).map(makeRowFromPoint);
  }

  async function rebuildPlan() {
    const profile = activeProfile.value;
    if (!profile) {
      start0ByPointKey.value = {};
      rebuildGridRows();
      return;
    }

    try {
      const filteredProfiles: ProfilesV1 = { schemaVersion: 1, profiles: [profile] };
      const filteredPoints: PointsV1 = {
        schemaVersion: 1,
        points: points.value.points.filter((p) => p.channelName === profile.channelName),
      };
      const built = await buildPlan(
        { profiles: filteredProfiles, points: filteredPoints },
        projectId.value,
        activeDeviceId.value
      );
      const map: Record<string, number> = {};
      for (const job of built.jobs) {
        for (const point of job.points) {
          map[point.pointKey] = job.startAddress + point.offset;
        }
      }
      start0ByPointKey.value = map;
    } catch {
      start0ByPointKey.value = {};
    } finally {
      rebuildGridRows();
    }
  }

  async function syncFromGridAndMapAddresses(touchedKeys?: string[]) {
    const grid = gridApi();
    if (!grid) return;
    const source = (await grid.getSource()) as T[];
    gridRows.value = source;

    const profile = activeProfile.value;
    if (!profile) return;

    for (const row of gridRows.value) {
      const point = points.value.points.find((p) => p.pointKey === row.pointKey);
      if (!point) continue;

      point.hmiName = row.hmiName;
      point.dataType = row.dataType;
      point.byteOrder = row.byteOrder;
      point.scale = Number(row.scale);

      const addrRaw = row.modbusAddress.trim();
      if (!addrRaw) {
        point.addressOffset = undefined;
        continue;
      }

      const parsed = parseHumanAddress(addrRaw, profile.readArea);
      if (!parsed.ok) continue;

      const offset = parsed.start0Based - profile.startAddress;
      if (offset < 0) continue;
      point.addressOffset = offset;
    }

    if (touchedKeys && touchedKeys.length > 0) {
      onTouched?.(touchedKeys);
    }
  }

  function applyLatestToGridRows(results: SampleResult[]) {
    const byKey: Record<string, SampleResult> = {};
    for (const r of results) byKey[r.pointKey] = r;
    runtimeByPointKey.value = byKey;

    const grid = gridApi();
    if (!grid) return;
    const idx = colIndexByProp.value;

    for (let rowIndex = 0; rowIndex < gridRows.value.length; rowIndex++) {
      const row = gridRows.value[rowIndex];
      const res = byKey[row.pointKey];
      if (!res) continue;

      const nextQuality = formatQualityLabel(res.quality);
      const nextValue = res.valueDisplay;
      const nextErr = res.errorMessage ?? "";
      const nextTs = res.timestamp;
      const nextMs = res.durationMs;

      const changed =
        row.quality !== nextQuality ||
        row.valueDisplay !== nextValue ||
        row.errorMessage !== nextErr ||
        row.timestamp !== nextTs ||
        row.durationMs !== nextMs;
      if (!changed) continue;

      row.quality = nextQuality;
      row.valueDisplay = nextValue;
      row.errorMessage = nextErr;
      row.timestamp = nextTs;
      row.durationMs = nextMs;

      void grid.setDataAt({ row: rowIndex, col: idx["quality"], rowType: "rgRow", colType: "rgCol", val: nextQuality });
      void grid.setDataAt({ row: rowIndex, col: idx["valueDisplay"], rowType: "rgRow", colType: "rgCol", val: nextValue });
      void grid.setDataAt({ row: rowIndex, col: idx["errorMessage"], rowType: "rgRow", colType: "rgCol", val: nextErr });
      void grid.setDataAt({ row: rowIndex, col: idx["timestamp"], rowType: "rgRow", colType: "rgCol", val: nextTs });
      void grid.setDataAt({ row: rowIndex, col: idx["durationMs"], rowType: "rgRow", colType: "rgCol", val: nextMs });
    }
  }

  return {
    rebuildPlan,
    syncFromGridAndMapAddresses,
    applyLatestToGridRows,
  };
}
