import { computed, ref, watch } from "vue";
import { ElMessage } from "element-plus";

import type { ConnectionProfile, RegisterArea, SerialParity } from "../api";
import { useCommDeviceContext } from "./useDeviceContext";

export function useConnectionPanel() {
  const { project, activeDevice, saveProject } = useCommDeviceContext();

  const AREA_OPTIONS: RegisterArea[] = ["Holding", "Coil"];
  const PARITY_OPTIONS: SerialParity[] = ["None", "Even", "Odd"];

  const profileDraft = ref<ConnectionProfile | null>(null);
  const profileBaseline = ref("");
  const profileDirty = computed(() => {
    if (!profileDraft.value) return false;
    return JSON.stringify(profileDraft.value) !== profileBaseline.value;
  });

  function cloneProfile(profile: ConnectionProfile): ConnectionProfile {
    return JSON.parse(JSON.stringify(profile)) as ConnectionProfile;
  }

  function resetProfileDraft() {
    if (!activeDevice.value) {
      profileDraft.value = null;
      profileBaseline.value = "";
      return;
    }
    profileDraft.value = cloneProfile(activeDevice.value.profile);
    profileBaseline.value = JSON.stringify(activeDevice.value.profile);
  }

  function switchProfileProtocol(next: "TCP" | "485") {
    const current = profileDraft.value;
    if (!current) return;
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

  async function saveProfileDraft() {
    const current = project.value;
    const active = activeDevice.value;
    const draft = profileDraft.value;
    if (!current || !active || !draft) {
      ElMessage.error("未选择设备");
      return;
    }
    if (!draft.channelName.trim()) {
      ElMessage.error("通道名称不能为空");
      return;
    }
    if (draft.protocolType === "TCP" && !draft.ip.trim()) {
      ElMessage.error("IP 不能为空");
      return;
    }
    if (draft.protocolType === "485" && !draft.serialPort.trim()) {
      ElMessage.error("串口不能为空");
      return;
    }
    const devices = current.devices ?? [];
    const idx = devices.findIndex((d) => d.deviceId === active.deviceId);
    if (idx < 0) return;
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
    ElMessage.success("连接配置已保存");
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

  return {
    AREA_OPTIONS,
    PARITY_OPTIONS,
    profileDraft,
    profileDirty,
    resetProfileDraft,
    switchProfileProtocol,
    saveProfileDraft,
  };
}
