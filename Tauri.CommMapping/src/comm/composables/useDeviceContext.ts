import { computed, inject, provide, ref, watch, type ComputedRef, type Ref } from "vue";

import type { CommDeviceV1, CommProjectDataV1 } from "../api";
import { commProjectLoadV1, commProjectSaveV1, commProjectUiStatePatchV1 } from "../api";

export interface CommDeviceContext {
  projectId: Ref<string>;
  project: Ref<CommProjectDataV1 | null>;
  devices: ComputedRef<CommDeviceV1[]>;
  activeDeviceId: Ref<string>;
  activeDevice: ComputedRef<CommDeviceV1 | null>;
  loading: Ref<boolean>;
  reloadProject: () => Promise<void>;
  saveProject: (next: CommProjectDataV1) => Promise<void>;
}

const COMM_DEVICE_CONTEXT_KEY = Symbol("comm-device-context");

export function provideCommDeviceContext(projectId: Ref<string>): CommDeviceContext {
  const project = ref<CommProjectDataV1 | null>(null);
  const activeDeviceId = ref<string>("");
  const loading = ref(false);
  let suppressPatch = false;

  const devices = computed(() => project.value?.devices ?? []);
  const activeDevice = computed(() => {
    const id = activeDeviceId.value;
    if (!id) return null;
    return devices.value.find((d) => d.deviceId === id) ?? null;
  });

  const chooseActiveDeviceId = (next: CommProjectDataV1): string => {
    const list = next.devices ?? [];
    if (list.length === 0) return "";
    const stored = next.uiState?.activeDeviceId ?? "";
    if (stored && list.some((d) => d.deviceId === stored)) return stored;
    return list[0].deviceId;
  };

  const patchActiveDevice = async (pid: string, nextId: string) => {
    if (!pid || !nextId) return;
    try {
      await commProjectUiStatePatchV1(pid, { activeDeviceId: nextId });
    } catch {
      // Ignore UI-state patch errors to avoid blocking core flows.
    }
  };

  const loadProject = async () => {
    const pid = projectId.value.trim();
    if (!pid) {
      project.value = null;
      activeDeviceId.value = "";
      return;
    }
    loading.value = true;
    try {
      const data = await commProjectLoadV1(pid);
      project.value = data;
      const nextId = chooseActiveDeviceId(data);
      suppressPatch = true;
      activeDeviceId.value = nextId;
      suppressPatch = false;
      if (nextId && nextId !== data.uiState?.activeDeviceId) {
        void patchActiveDevice(pid, nextId);
      }
    } catch {
      project.value = null;
      activeDeviceId.value = "";
    } finally {
      loading.value = false;
    }
  };

  const saveProject = async (next: CommProjectDataV1) => {
    await commProjectSaveV1(next);
    project.value = next;
    const nextId = chooseActiveDeviceId(next);
    suppressPatch = true;
    activeDeviceId.value = nextId;
    suppressPatch = false;
  };

  watch(projectId, () => void loadProject(), { immediate: true });

  watch(activeDeviceId, (next) => {
    if (suppressPatch) return;
    const pid = projectId.value.trim();
    if (!pid || !next) return;
    if (project.value) {
      const ui = project.value.uiState ?? {};
      project.value = {
        ...project.value,
        uiState: {
          ...ui,
          activeDeviceId: next,
        },
      };
    }
    void patchActiveDevice(pid, next);
  });

  const ctx: CommDeviceContext = {
    projectId,
    project,
    devices,
    activeDeviceId,
    activeDevice,
    loading,
    reloadProject: loadProject,
    saveProject,
  };

  provide(COMM_DEVICE_CONTEXT_KEY, ctx);
  return ctx;
}

export function useCommDeviceContext(): CommDeviceContext {
  const ctx = inject<CommDeviceContext>(COMM_DEVICE_CONTEXT_KEY);
  if (!ctx) {
    throw new Error("CommDeviceContext is missing. Ensure ProjectWorkspace provides it.");
  }
  return ctx;
}
