<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ElMessage } from "element-plus";

import type { ConnectionProfile, RegisterArea, SerialParity } from "../../api";
import { useCommDeviceContext } from "../../composables/useDeviceContext";

const { project, activeDevice, saveProject } = useCommDeviceContext();

const AREA_OPTIONS: RegisterArea[] = ["Holding", "Coil"];
const PARITY_OPTIONS: SerialParity[] = ["None", "Even", "Odd"];

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

function cloneProfile(profile: ConnectionProfile): ConnectionProfile {
  return JSON.parse(JSON.stringify(profile)) as ConnectionProfile;
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
</script>

<template>
  <section class="comm-panel comm-animate" style="--delay: 80ms">
    <div class="comm-panel-header">
      <div class="comm-section-title">连接配置</div>
      <el-button size="small" :disabled="!profileDraft" @click="resetProfileDraft">重置</el-button>
    </div>

    <div v-if="profileDraft" class="comm-connection-form">
      <el-form label-width="72px" class="comm-form-compact">
        <el-form-item label="协议">
          <el-radio-group v-model="profileDraft.protocolType" @change="switchProfileProtocol">
            <el-radio-button label="TCP">TCP</el-radio-button>
            <el-radio-button label="485">485</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="通道">
          <el-input v-model="profileDraft.channelName" />
        </el-form-item>
        <el-form-item label="站号">
          <el-input-number v-model="profileDraft.deviceId" :min="0" :max="255" />
        </el-form-item>
        <el-form-item label="区域">
          <el-select v-model="profileDraft.readArea">
            <el-option v-for="opt in AREA_OPTIONS" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>

        <template v-if="profileDraft.protocolType === 'TCP'">
          <el-form-item label="IP">
            <el-input v-model="profileDraft.ip" />
          </el-form-item>
          <el-form-item label="端口">
            <el-input-number v-model="profileDraft.port" :min="1" :max="65535" />
          </el-form-item>
        </template>

        <template v-else>
          <el-form-item label="串口">
            <el-input v-model="profileDraft.serialPort" />
          </el-form-item>
          <el-form-item label="波特率">
            <el-input-number v-model="profileDraft.baudRate" :min="300" />
          </el-form-item>
          <el-form-item label="校验">
            <el-select v-model="profileDraft.parity">
              <el-option v-for="opt in PARITY_OPTIONS" :key="opt" :label="opt" :value="opt" />
            </el-select>
          </el-form-item>
          <el-form-item label="数据位">
            <el-input-number v-model="profileDraft.dataBits" :min="5" :max="8" />
          </el-form-item>
          <el-form-item label="停止位">
            <el-input-number v-model="profileDraft.stopBits" :min="1" :max="2" />
          </el-form-item>
        </template>

        <el-form-item label="超时">
          <el-input-number v-model="profileDraft.timeoutMs" :min="1" />
        </el-form-item>
        <el-form-item label="重试">
          <el-input-number v-model="profileDraft.retryCount" :min="0" />
        </el-form-item>
        <el-form-item label="轮询">
          <el-input-number v-model="profileDraft.pollIntervalMs" :min="50" />
        </el-form-item>
      </el-form>

      <div class="comm-panel-actions">
        <el-button size="small" type="primary" :disabled="!profileDirty" @click="saveProfileDraft">保存连接</el-button>
      </div>
    </div>
    <el-empty v-else description="未选择设备，无法编辑连接" />
  </section>
</template>

<style scoped>
.comm-connection-form :deep(.el-input-number),
.comm-connection-form :deep(.el-select) {
  width: 100%;
}

.comm-connection-form :deep(.el-radio-group) {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
</style>
