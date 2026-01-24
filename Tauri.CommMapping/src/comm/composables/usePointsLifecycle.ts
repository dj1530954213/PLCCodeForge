import { nextTick, onBeforeUnmount, onMounted, watch, type ComputedRef, type Ref } from "vue";

import type { CommDeviceV1, ConnectionProfile, ProfilesV1 } from "../api";
import { patchProjectUiState } from "../services/projects";
import { createStandardShortcuts, useKeyboardShortcuts } from "./useKeyboardShortcuts";
import type { CommWorkspaceRuntime } from "./useWorkspaceRuntime";

type LogLevel = "info" | "success" | "warning" | "error";
type RowSpan = { rowStart: number; rowEnd: number };

export interface UsePointsLifecycleOptions {
  projectId: Ref<string>;
  activeDeviceId: Ref<string>;
  activeDevice: ComputedRef<CommDeviceV1 | null>;
  activeChannelName: Ref<string>;
  profiles: Ref<ProfilesV1>;
  selectedRangeRows: Ref<RowSpan | null>;
  suppressChannelWatch: Ref<boolean>;
  rebuildPlan: () => Promise<void>;
  loadAll: () => Promise<void>;
  pushLog: (scope: string, level: LogLevel, message: string) => void;
  workspaceRuntime: CommWorkspaceRuntime;
  attachGridSelectionListeners: () => void;
  detachGridSelectionListeners: () => void;
  disposeRun: () => void;
  shortcuts: {
    onBatchAdd: () => void;
    onBatchEdit: () => void;
    onDelete: () => void;
    onUndo: () => void;
    onRedo: () => void;
    onSave: () => void;
  };
}

export function usePointsLifecycle(options: UsePointsLifecycleOptions) {
  onMounted(() => {
    nextTick(() => {
      options.attachGridSelectionListeners();
    });
  });

  watch(options.activeChannelName, async (value) => {
    if (options.suppressChannelWatch.value) return;
    options.selectedRangeRows.value = null;
    const pid = options.projectId.value.trim();
    if (pid) {
      patchProjectUiState(pid, { activeChannelName: value }).catch((e: unknown) => {
        options.pushLog("ui_state", "warning", `当前通道保存失败：${String((e as any)?.message ?? e ?? "")}`);
      });
    }
    await options.rebuildPlan();
  });

  watch(options.activeDevice, (device) => {
    if (!device) {
      options.profiles.value = { schemaVersion: 1, profiles: [] };
      options.activeChannelName.value = "";
      return;
    }
    options.profiles.value = {
      schemaVersion: 1,
      profiles: [JSON.parse(JSON.stringify(device.profile)) as ConnectionProfile],
    };
    if (options.activeChannelName.value !== device.profile.channelName) {
      options.activeChannelName.value = device.profile.channelName;
    }
    void options.rebuildPlan();
  });

  watch(options.activeDeviceId, () => {
    options.workspaceRuntime.stats.value = null;
    options.workspaceRuntime.updatedAtUtc.value = "";
  });

  watch([options.projectId, options.activeDeviceId], () => void options.loadAll(), { immediate: true });

  useKeyboardShortcuts(createStandardShortcuts(options.shortcuts));

  onBeforeUnmount(() => {
    options.disposeRun();
    options.detachGridSelectionListeners();
  });
}
