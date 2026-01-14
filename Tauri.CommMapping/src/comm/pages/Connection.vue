<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ElMessage } from "element-plus";

import type { ConnectionProfile, ProfilesV1, RegisterArea, SerialParity } from "../api";
import { commProfilesLoad, commProfilesSave } from "../api";
import { useCommDeviceContext } from "../composables/useDeviceContext";

const { projectId, activeDeviceId, activeDevice } = useCommDeviceContext();

const AREA_OPTIONS: RegisterArea[] = ["Holding", "Coil"];
const PARITY_OPTIONS: SerialParity[] = ["None", "Even", "Odd"];

const model = ref<ProfilesV1>({ schemaVersion: 1, profiles: [] });
const hasProfile = computed(() => model.value.profiles.length > 0);

const dialogOpen = ref(false);
const editingIndex = ref<number | null>(null);
const editing = ref<ConnectionProfile>({
  protocolType: "TCP",
  channelName: "tcp-ok",
  deviceId: 1,
  readArea: "Holding",
  startAddress: 0,
  length: 10,
  ip: "127.0.0.1",
  port: 502,
  timeoutMs: 200,
  retryCount: 0,
  pollIntervalMs: 500,
});

function openAddTcp() {
  editingIndex.value = hasProfile.value ? 0 : null;
  editing.value = {
    protocolType: "TCP",
    channelName: "tcp-1",
    deviceId: 1,
    readArea: "Holding",
    startAddress: 0,
    length: 20,
    ip: "127.0.0.1",
    port: 502,
    timeoutMs: 500,
    retryCount: 0,
    pollIntervalMs: 500,
  };
  dialogOpen.value = true;
}

function openAdd485() {
  editingIndex.value = hasProfile.value ? 0 : null;
  editing.value = {
    protocolType: "485",
    channelName: "485-1",
    deviceId: 1,
    readArea: "Coil",
    startAddress: 0,
    length: 20,
    serialPort: "COM1",
    baudRate: 9600,
    parity: "None",
    dataBits: 8,
    stopBits: 1,
    timeoutMs: 500,
    retryCount: 0,
    pollIntervalMs: 500,
  };
  dialogOpen.value = true;
}

function openEdit(index: number) {
  editingIndex.value = index;
  editing.value = JSON.parse(JSON.stringify(model.value.profiles[index])) as ConnectionProfile;
  dialogOpen.value = true;
}

function removeAt(index: number) {
  model.value.profiles.splice(index, 1);
}

function uiStartAddress(profile: ConnectionProfile) {
  return profile.startAddress + 1;
}

function setUiStartAddress(profile: ConnectionProfile, uiValue: number) {
  profile.startAddress = Math.max(0, Math.floor(uiValue) - 1);
}

const editingUiStartAddress = computed<number>({
  get() {
    return uiStartAddress(editing.value);
  },
  set(v) {
    setUiStartAddress(editing.value, v);
  },
});

async function load(silent = false) {
  try {
    const pid = projectId.value.trim();
    const did = activeDeviceId.value.trim();
    if (!pid || !did) {
      model.value = { schemaVersion: 1, profiles: [] };
      return;
    }

    model.value = await commProfilesLoad(pid, did);
    if (model.value.profiles.length > 1) {
      model.value = { ...model.value, profiles: [model.value.profiles[0]] };
      if (!silent) {
        ElMessage.warning("当前设备仅支持单通道，已保留第一个连接");
      }
    }
    if (!silent) ElMessage.success("已加载 profiles");
  } catch (e: unknown) {
    ElMessage.error(String((e as any)?.message ?? e ?? "加载失败"));
  }
}

async function save() {
  try {
    const pid = projectId.value.trim();
    const did = activeDeviceId.value.trim();
    if (!pid || !did) {
      ElMessage.error("未选择设备");
      return;
    }
    if (model.value.profiles.length > 1) {
      model.value = { ...model.value, profiles: [model.value.profiles[0]] };
    }
    if (model.value.profiles.length === 0) {
      ElMessage.error("连接配置为空");
      return;
    }
    await commProfilesSave(model.value, pid, did);
    ElMessage.success("已保存 profiles");
  } catch (e: unknown) {
    ElMessage.error(String((e as any)?.message ?? e ?? "保存失败"));
  }
}

function saveEditing() {
  if (!editing.value.channelName.trim()) {
    ElMessage.error("通道名称不能为空");
    return;
  }
  if (editing.value.length <= 0) {
    ElMessage.error("长度必须 > 0");
    return;
  }
  if (editing.value.protocolType === "TCP") {
    if (!editing.value.ip.trim()) {
      ElMessage.error("IP 不能为空");
      return;
    }
  }

  if (editingIndex.value === null) {
    model.value.profiles = [editing.value];
  } else {
    model.value.profiles[editingIndex.value] = editing.value;
  }
  dialogOpen.value = false;
}

watch([projectId, activeDeviceId], () => void load(true), { immediate: true });
</script>

<template>
  <div class="comm-subpage comm-subpage--connection">
    <header class="comm-hero comm-animate" style="--delay: 0ms">
      <div class="comm-hero-title">
        <div class="comm-title">连接配置</div>
        <div class="comm-subtitle">
          设备：{{ activeDevice?.deviceName ?? "未选择" }} <span v-if="activeDevice">· deviceId {{ activeDevice.deviceId }}</span>
        </div>
      </div>
      <div class="comm-hero-actions">
        <el-button type="primary" @click="openAddTcp">新增 TCP</el-button>
        <el-button type="primary" @click="openAdd485">新增 485</el-button>
        <el-button @click="load(false)">加载</el-button>
        <el-button @click="save">保存</el-button>
      </div>
    </header>

    <section class="comm-panel comm-animate" style="--delay: 80ms">
      <div class="comm-panel-header">
        <div class="comm-section-title">通道列表</div>
        <div class="comm-inline-meta">MVP：单设备单通道</div>
      </div>

      <el-alert
        v-if="model.profiles.length > 1"
        type="warning"
        show-icon
        :closable="false"
        title="当前设备仅支持单通道，保存时将仅保留第一个连接"
        style="margin-bottom: 12px"
      />

      <el-table :data="model.profiles" style="width: 100%">
        <el-table-column label="协议类型" width="90">
          <template #default="{ row }">{{ row.protocolType }}</template>
        </el-table-column>
        <el-table-column prop="channelName" label="通道名称" min-width="160" />
        <el-table-column prop="readArea" label="读取区域" width="100" />
        <el-table-column label="起始地址（从 1 开始）" width="160">
          <template #default="{ row }">{{ row.startAddress + 1 }}</template>
        </el-table-column>
        <el-table-column prop="length" label="长度" width="80" />
        <el-table-column label="操作" width="160">
          <template #default="{ $index }">
            <el-button size="small" @click="openEdit($index)">编辑</el-button>
            <el-button size="small" type="danger" @click="removeAt($index)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>
  </div>

  <el-dialog v-model="dialogOpen" width="720px">
    <template #header>
      <span>{{ editingIndex === null ? "新增" : "编辑" }}连接</span>
    </template>

    <el-form label-width="140px">
      <el-form-item label="协议类型">
        <el-tag>{{ editing.protocolType }}</el-tag>
      </el-form-item>

      <el-form-item label="通道名称">
        <el-input v-model="editing.channelName" />
      </el-form-item>

      <el-form-item label="设备标识">
        <el-input-number v-model="editing.deviceId" :min="0" :max="255" />
      </el-form-item>

      <el-form-item label="读取区域（MVP）">
        <el-select v-model="editing.readArea" style="width: 220px">
          <el-option v-for="opt in AREA_OPTIONS" :key="opt" :label="opt" :value="opt" />
        </el-select>
      </el-form-item>

      <el-form-item label="起始地址（从 1 开始）">
        <el-input-number v-model="editingUiStartAddress" :min="1" />
      </el-form-item>

      <el-form-item label="长度">
        <el-input-number v-model="editing.length" :min="1" />
      </el-form-item>

      <template v-if="editing.protocolType === 'TCP'">
        <el-form-item label="TCP: IP">
          <el-input v-model="editing.ip" />
        </el-form-item>
        <el-form-item label="TCP: 端口">
          <el-input-number v-model="editing.port" :min="1" :max="65535" />
        </el-form-item>
      </template>

      <template v-else>
        <el-form-item label="485: 串口">
          <el-input v-model="editing.serialPort" />
        </el-form-item>
        <el-form-item label="485: 波特率">
          <el-input-number v-model="editing.baudRate" :min="300" />
        </el-form-item>
        <el-form-item label="485: 校验">
          <el-select v-model="editing.parity" style="width: 220px">
            <el-option v-for="opt in PARITY_OPTIONS" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>
        <el-form-item label="485: 数据位">
          <el-input-number v-model="editing.dataBits" :min="5" :max="8" />
        </el-form-item>
        <el-form-item label="485: 停止位">
          <el-input-number v-model="editing.stopBits" :min="1" :max="2" />
        </el-form-item>
      </template>

      <el-form-item label="超时 ms">
        <el-input-number v-model="editing.timeoutMs" :min="1" />
      </el-form-item>
      <el-form-item label="重试次数">
        <el-input-number v-model="editing.retryCount" :min="0" />
      </el-form-item>
      <el-form-item label="轮询周期 ms">
        <el-input-number v-model="editing.pollIntervalMs" :min="50" />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="dialogOpen = false">取消</el-button>
      <el-button type="primary" @click="saveEditing">确定</el-button>
    </template>
  </el-dialog>
</template>
