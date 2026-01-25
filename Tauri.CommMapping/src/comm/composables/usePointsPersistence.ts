import type { ComputedRef, Ref } from "vue";

import type { CommDeviceV1, CommProjectDataV1, ConnectionProfile, PointsV1, ProfilesV1 } from "../api";
import { notifyError, notifySuccess, resolveErrorMessage } from "../services/notify";
import { loadPoints as loadPointsData, savePoints as savePointsData } from "../services/points";
import type { BatchAddTemplate } from "./usePointsBatchOps";
import type { PointRowLike } from "./usePointsRows";

type RowSpan = { rowStart: number; rowEnd: number };

export interface UsePointsPersistenceOptions<T extends PointRowLike> {
  projectId: Ref<string>;
  activeDeviceId: Ref<string>;
  project: Ref<CommProjectDataV1 | null>;
  activeDevice: ComputedRef<CommDeviceV1 | null>;
  profiles: Ref<ProfilesV1>;
  points: Ref<PointsV1>;
  gridRows: Ref<T[]>;
  activeChannelName: Ref<string>;
  batchAddTemplate: Ref<BatchAddTemplate>;
  showAllValidation: Ref<boolean>;
  touchedRowKeys: Ref<Record<string, boolean>>;
  selectedRangeRows: Ref<RowSpan | null>;
  suppressChannelWatch: Ref<boolean>;
  markPointsChanged: () => void;
  rebuildPlan: () => Promise<void>;
  validateRowForRun: (row: T) => string | null;
  syncFromGridAndMapAddresses: () => Promise<void>;
}

export function usePointsPersistence<T extends PointRowLike>(options: UsePointsPersistenceOptions<T>) {
  const {
    projectId,
    activeDeviceId,
    project,
    activeDevice,
    profiles,
    points,
    gridRows,
    activeChannelName,
    batchAddTemplate,
    showAllValidation,
    touchedRowKeys,
    selectedRangeRows,
    suppressChannelWatch,
    markPointsChanged,
    rebuildPlan,
    validateRowForRun,
    syncFromGridAndMapAddresses,
  } = options;

  async function loadAll() {
    try {
      const pid = projectId.value.trim();
      const did = activeDeviceId.value.trim();
      if (!pid || !did) {
        profiles.value = { schemaVersion: 1, profiles: [] };
        points.value = { schemaVersion: 1, points: [] };
        activeChannelName.value = "";
        return;
      }

      const device = activeDevice.value;
      profiles.value = device
        ? { schemaVersion: 1, profiles: [JSON.parse(JSON.stringify(device.profile)) as ConnectionProfile] }
        : { schemaVersion: 1, profiles: [] };
      points.value = await loadPointsData(pid, did);
      showAllValidation.value = false;
      touchedRowKeys.value = {};
      selectedRangeRows.value = null;
      markPointsChanged();

      suppressChannelWatch.value = true;
      const profile = profiles.value.profiles[0];
      if (profile) {
        activeChannelName.value = profile.channelName;
      } else if (points.value.points.length > 0) {
        activeChannelName.value = points.value.points[0].channelName;
      } else {
        activeChannelName.value = "";
      }

      const t = project.value?.uiState?.pointsBatchTemplate;
      if (t && t.schemaVersion === 1) {
        batchAddTemplate.value = {
          count: Math.max(1, Math.min(500, Math.floor(t.count || 10))),
          startAddressHuman: String(t.startAddressHuman ?? "").trim(),
          dataType: t.dataType ?? batchAddTemplate.value.dataType,
          byteOrder: t.byteOrder ?? batchAddTemplate.value.byteOrder,
          hmiNameTemplate: String(t.hmiNameTemplate ?? batchAddTemplate.value.hmiNameTemplate),
          scaleTemplate: String(t.scaleTemplate ?? batchAddTemplate.value.scaleTemplate),
          insertMode: t.insertMode === "afterSelection" ? "afterSelection" : "append",
        };
      }

      suppressChannelWatch.value = false;
      await rebuildPlan();
      notifySuccess("已加载点位与连接配置");
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "加载失败"));
    }
  }

  async function savePoints() {
    showAllValidation.value = true;
    await syncFromGridAndMapAddresses();
    const invalid = gridRows.value.map(validateRowForRun).find((v) => Boolean(v));
    if (invalid) {
      notifyError(invalid);
      return;
    }
    if (!activeDeviceId.value.trim()) {
      notifyError("未选择设备");
      return;
    }
    await savePointsData(points.value, projectId.value, activeDeviceId.value);
    if (project.value) {
      const devices = project.value.devices ?? [];
      const idx = devices.findIndex((d) => d.deviceId === activeDeviceId.value);
      if (idx >= 0) {
        const nextDevices = [...devices];
        nextDevices[idx] = {
          ...nextDevices[idx],
          points: { ...points.value, points: [...points.value.points] },
        };
        project.value = { ...project.value, devices: nextDevices };
      }
    }
    notifySuccess("已保存点位");
    showAllValidation.value = false;
    touchedRowKeys.value = {};
  }

  return {
    loadAll,
    savePoints,
  };
}
