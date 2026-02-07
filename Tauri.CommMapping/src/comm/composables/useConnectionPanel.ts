import { computed, ref, watch } from "vue";
import type { ConnectionProfile, RegisterArea, SerialParity } from "../api";
import { notifyError, notifySuccess, resolveErrorMessage } from "../services/notify";
import { cloneProfile } from "../services/profiles";
import { listSerialPorts } from "../services/serial";
import { useCommDeviceContext } from "./useDeviceContext";

export function useConnectionPanel() {
  const { project, activeDevice, saveProject } = useCommDeviceContext();

  const AREA_OPTIONS: RegisterArea[] = ["Holding", "Coil"];
  const PARITY_OPTIONS: SerialParity[] = ["None", "Even", "Odd"];
  const BAUD_OPTIONS = [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200];
  const DATA_BITS_OPTIONS = [5, 6, 7, 8];
  const STOP_BITS_OPTIONS = [1, 2];

  const profileDraft = ref<ConnectionProfile | null>(null);
  const profileBaseline = ref("");
  const profileDirty = computed(() => {
    if (!profileDraft.value) return false;
    return JSON.stringify(profileDraft.value) !== profileBaseline.value;
  });

  function resetProfileDraft() {
    if (!activeDevice.value) {
      profileDraft.value = null;
      profileBaseline.value = "";
      return;
    }
    profileDraft.value = cloneProfile(activeDevice.value.profile);
    profileBaseline.value = JSON.stringify(activeDevice.value.profile);
  }

  function switchProfileProtocol(next: string | number | boolean | undefined) {
    const current = profileDraft.value;
    if (!current) return;
    if (next !== "TCP" && next !== "485") return;
    if (current.protocolType === next) return;
    const base = {
      protocolType: next,
      channelName: current.channelName || (next === "TCP" ? "tcp-1" : "485-1"),
      deviceId: current.deviceId ?? 1,
      readArea: current.readArea ?? "Holding",
      startAddress: current.startAddress ?? 0,
      length: current.length ?? 1,
      timeoutMs: current.timeoutMs ?? 500,
      retryCount: current.retryCount ?? 0,
      pollIntervalMs: current.pollIntervalMs ?? 500,
    };
    if (next === "TCP") {
      profileDraft.value = {
        ...base,
        protocolType: "TCP",
        ip: current.protocolType === "TCP" ? current.ip : "127.0.0.1",
        port: current.protocolType === "TCP" ? current.port : 502,
      };
    } else {
      profileDraft.value = {
        ...base,
        protocolType: "485",
        serialPort: current.protocolType === "485" ? current.serialPort : "COM1",
        baudRate: current.protocolType === "485" ? current.baudRate : 9600,
        parity: current.protocolType === "485" ? current.parity : "None",
        dataBits: current.protocolType === "485" ? current.dataBits : 8,
        stopBits: current.protocolType === "485" ? current.stopBits : 1,
      };
    }
  }

  const serialPorts = ref<string[]>([]);
  const serialPortsLoading = ref(false);
  const serialPortsLoaded = ref(false);
  const serialPortOptions = computed(() => {
    const set = new Set(serialPorts.value);
    const current = profileDraft.value?.protocolType === "485" ? profileDraft.value.serialPort.trim() : "";
    if (current) set.add(current);
    return Array.from(set);
  });

  async function refreshSerialPorts(force = false) {
    if (serialPortsLoading.value) return;
    if (!force && serialPortsLoaded.value) return;
    serialPortsLoading.value = true;
    try {
      const ports = await listSerialPorts();
      const normalized = ports.map((p) => p.trim()).filter((p) => p.length > 0);
      normalized.sort((a, b) => a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }));
      serialPorts.value = normalized;
      serialPortsLoaded.value = true;
      if (profileDraft.value?.protocolType === "485") {
        const current = profileDraft.value.serialPort.trim();
        if (!current && normalized.length > 0) {
          profileDraft.value.serialPort = normalized[0];
        }
      }
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "读取串口失败"));
      serialPorts.value = [];
      serialPortsLoaded.value = false;
    } finally {
      serialPortsLoading.value = false;
    }
  }

  async function saveProfileDraft(options?: { silent?: boolean }): Promise<boolean> {
    const current = project.value;
    const active = activeDevice.value;
    const draft = profileDraft.value;
    if (!current || !active || !draft) {
      notifyError("未选择设备");
      return false;
    }
    if (!draft.channelName.trim()) {
      notifyError("通道名称不能为空");
      return false;
    }
    if (draft.protocolType === "TCP" && !draft.ip.trim()) {
      notifyError("IP 不能为空");
      return false;
    }
    if (draft.protocolType === "485" && !draft.serialPort.trim()) {
      notifyError("串口不能为空");
      return false;
    }
    const devices = current.devices ?? [];
    const idx = devices.findIndex((d) => d.deviceId === active.deviceId);
    if (idx < 0) return false;
    const nextDevices = [...devices];
    nextDevices[idx] = {
      ...nextDevices[idx],
      profile: cloneProfile(draft),
    };
    const isFirst = nextDevices[0]?.deviceId === active.deviceId;
    const next = {
      ...current,
      devices: nextDevices,
      connections: isFirst
        ? { schemaVersion: 1, profiles: [cloneProfile(draft)] }
        : current.connections,
    };
    await saveProject(next);
    profileBaseline.value = JSON.stringify(draft);
    if (!options?.silent) {
      notifySuccess("连接配置已保存");
    }
    return true;
  }

  watch(project, (next) => {
    if (!next) {
      profileDraft.value = null;
      profileBaseline.value = "";
      return;
    }
    resetProfileDraft();
  });

  watch(activeDevice, () => {
    resetProfileDraft();
  });

  watch(
    () => profileDraft.value?.protocolType,
    (next) => {
      if (next === "485") {
        void refreshSerialPorts();
      }
    }
  );

  return {
    AREA_OPTIONS,
    PARITY_OPTIONS,
    BAUD_OPTIONS,
    DATA_BITS_OPTIONS,
    STOP_BITS_OPTIONS,
    profileDraft,
    profileDirty,
    serialPorts,
    serialPortsLoading,
    serialPortOptions,
    resetProfileDraft,
    switchProfileProtocol,
    refreshSerialPorts,
    saveProfileDraft,
  };
}
