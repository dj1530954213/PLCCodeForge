<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";

import type {
  CommDeviceCopyRuleV1,
  CommDeviceCopyTemplateV1,
  CommDeviceV1,
  CommProjectDataV1,
  ConnectionProfile,
} from "../../api";
import { useCommDeviceContext } from "../../composables/useDeviceContext";

const { project, devices, activeDeviceId, activeDevice, saveProject } = useCommDeviceContext();

const deviceEdit = ref({ name: "", workbookName: "" });
const deviceDirty = computed(() => {
  const current = activeDevice.value;
  if (!current) return false;
  return (
    deviceEdit.value.name.trim() !== current.deviceName ||
    deviceEdit.value.workbookName.trim() !== current.workbookName
  );
});

const addDialogOpen = ref(false);
const addDeviceName = ref("");
const addUseActiveProfile = ref(true);

const copyDialogOpen = ref(false);
const copySourceDeviceId = ref("");
const copyDeviceName = ref("");
const copyRules = ref<CommDeviceCopyRuleV1[]>([]);
const copyTemplateId = ref("");
const copyTemplateName = ref("");

const copyTemplates = computed<CommDeviceCopyTemplateV1[]>(() => {
  return project.value?.uiState?.deviceCopyTemplates ?? [];
});

function sanitizeWorkbookName(name: string): string {
  const sanitized = name.replace(/[\\/:*?"<>|]/g, "_").trim();
  return sanitized ? sanitized : "Device";
}

function defaultProfile(): ConnectionProfile {
  return {
    protocolType: "TCP",
    channelName: "tcp-1",
    deviceId: 1,
    readArea: "Holding",
    startAddress: 0,
    length: 1,
    ip: "127.0.0.1",
    port: 502,
    timeoutMs: 1000,
    retryCount: 0,
    pollIntervalMs: 500,
  };
}

function cloneProfile(profile: ConnectionProfile): ConnectionProfile {
  return JSON.parse(JSON.stringify(profile)) as ConnectionProfile;
}

function newPointKey(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `pt-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function normalizedCopyRules(): CommDeviceCopyRuleV1[] {
  return copyRules.value
    .map((r) => ({ find: r.find.trim(), replace: r.replace ?? "" }))
    .filter((r) => r.find.length > 0);
}

function applyCopyRules(value: string, rules: CommDeviceCopyRuleV1[]): string {
  let out = value;
  for (const rule of rules) {
    if (!rule.find) continue;
    out = out.split(rule.find).join(rule.replace);
  }
  return out;
}

function resetCopyRules() {
  copyRules.value = [{ find: "", replace: "" }];
  copyTemplateId.value = "";
  copyTemplateName.value = "";
}

function selectDevice(deviceId: string) {
  activeDeviceId.value = deviceId;
}

function openAddDialog() {
  addDeviceName.value = "";
  addUseActiveProfile.value = true;
  addDialogOpen.value = true;
}

function openCopyDialog() {
  const active = activeDevice.value;
  if (!active) return;
  copySourceDeviceId.value = active.deviceId;
  copyDeviceName.value = `${active.deviceName}-copy`;
  resetCopyRules();
  copyDialogOpen.value = true;
}

async function confirmDeleteDevice() {
  const current = project.value;
  const active = activeDevice.value;
  if (!current || !active) {
    ElMessage.warning("未选择设备");
    return;
  }

  await ElMessageBox.confirm(
    `确认删除设备「${active.deviceName}」？删除后将无法恢复该设备的点位与连接配置。`,
    "删除设备",
    {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    }
  );

  const nextDevices = (current.devices ?? []).filter((d) => d.deviceId !== active.deviceId);
  const nextActiveId = nextDevices[0]?.deviceId ?? "";
  const next: CommProjectDataV1 = {
    ...current,
    devices: nextDevices,
    uiState: {
      ...(current.uiState ?? {}),
      activeDeviceId: nextActiveId,
    },
  };

  try {
    await saveProject(next);
    ElMessage.success("设备已删除");
  } catch (e: unknown) {
    ElMessage.error(String((e as any)?.message ?? e ?? "删除设备失败"));
  }
}

async function confirmAddDevice() {
  const name = addDeviceName.value.trim();
  if (!name) {
    ElMessage.error("设备名称不能为空");
    return;
  }
  const current = project.value;
  if (!current) return;

  const deviceId = newPointKey();
  const profile = addUseActiveProfile.value && activeDevice.value
    ? cloneProfile(activeDevice.value.profile)
    : defaultProfile();

  const newDevice: CommDeviceV1 = {
    deviceId,
    deviceName: name,
    workbookName: sanitizeWorkbookName(name),
    profile,
    points: { schemaVersion: 1, points: [] },
  };

  const next: CommProjectDataV1 = {
    ...current,
    devices: [...(current.devices ?? []), newDevice],
    uiState: {
      ...(current.uiState ?? {}),
      activeDeviceId: deviceId,
    },
  };

  try {
    await saveProject(next);
    activeDeviceId.value = deviceId;
    addDialogOpen.value = false;
    ElMessage.success("已新增设备");
  } catch (e: unknown) {
    ElMessage.error(String((e as any)?.message ?? e ?? "新增设备失败"));
  }
}

async function confirmCopyDevice() {
  const name = copyDeviceName.value.trim();
  if (!name) {
    ElMessage.error("设备名称不能为空");
    return;
  }
  const current = project.value;
  if (!current) return;
  const source = (current.devices ?? []).find((d) => d.deviceId === copySourceDeviceId.value);
  if (!source) {
    ElMessage.error("源设备不存在");
    return;
  }

  const rules = normalizedCopyRules();
  const copiedPoints = source.points.points.map((p) => ({
    ...p,
    pointKey: newPointKey(),
    hmiName: applyCopyRules(p.hmiName, rules),
  }));

  const deviceId = newPointKey();
  const newDevice: CommDeviceV1 = {
    deviceId,
    deviceName: name,
    workbookName: sanitizeWorkbookName(name),
    profile: cloneProfile(source.profile),
    points: {
      schemaVersion: source.points.schemaVersion,
      points: copiedPoints,
    },
  };

  const next: CommProjectDataV1 = {
    ...current,
    devices: [...(current.devices ?? []), newDevice],
    uiState: {
      ...(current.uiState ?? {}),
      activeDeviceId: deviceId,
    },
  };

  try {
    await saveProject(next);
    activeDeviceId.value = deviceId;
    copyDialogOpen.value = false;
    ElMessage.success("已复制设备");
  } catch (e: unknown) {
    ElMessage.error(String((e as any)?.message ?? e ?? "复制设备失败"));
  }
}

async function saveDeviceMeta() {
  const current = project.value;
  const active = activeDevice.value;
  if (!current || !active) {
    ElMessage.error("未选择设备");
    return;
  }
  const name = deviceEdit.value.name.trim();
  if (!name) {
    ElMessage.error("设备名称不能为空");
    return;
  }
  const workbookName = deviceEdit.value.workbookName.trim() || sanitizeWorkbookName(name);
  const devices = current.devices ?? [];
  const idx = devices.findIndex((d) => d.deviceId === active.deviceId);
  if (idx < 0) return;
  const nextDevices = [...devices];
  nextDevices[idx] = {
    ...nextDevices[idx],
    deviceName: name,
    workbookName,
  };
  const next: CommProjectDataV1 = {
    ...current,
    devices: nextDevices,
  };
  await saveProject(next);
  ElMessage.success("设备信息已保存");
}

async function saveCopyTemplate() {
  const name = copyTemplateName.value.trim();
  if (!name) {
    ElMessage.error("模板名称不能为空");
    return;
  }
  const rules = normalizedCopyRules();
  if (rules.length === 0) {
    ElMessage.error("至少需要一条替换规则");
    return;
  }
  const current = project.value;
  if (!current) return;

  const nextTemplate: CommDeviceCopyTemplateV1 = {
    templateId: newPointKey(),
    name,
    rules,
  };
  const list = current.uiState?.deviceCopyTemplates ?? [];
  const next: CommProjectDataV1 = {
    ...current,
    uiState: {
      ...(current.uiState ?? {}),
      deviceCopyTemplates: [...list, nextTemplate],
    },
  };

  try {
    await saveProject(next);
    copyTemplateId.value = nextTemplate.templateId;
    ElMessage.success("模板已保存");
  } catch (e: unknown) {
    ElMessage.error(String((e as any)?.message ?? e ?? "保存模板失败"));
  }
}

async function deleteCopyTemplate() {
  const current = project.value;
  if (!current) return;
  const id = copyTemplateId.value;
  if (!id) return;

  const list = current.uiState?.deviceCopyTemplates ?? [];
  const nextList = list.filter((t) => t.templateId !== id);
  const next: CommProjectDataV1 = {
    ...current,
    uiState: {
      ...(current.uiState ?? {}),
      deviceCopyTemplates: nextList,
    },
  };

  try {
    await saveProject(next);
    copyTemplateId.value = "";
    ElMessage.success("模板已删除");
  } catch (e: unknown) {
    ElMessage.error(String((e as any)?.message ?? e ?? "删除模板失败"));
  }
}

watch(copyTemplateId, (id) => {
  if (!id) return;
  const template = copyTemplates.value.find((t) => t.templateId === id);
  if (!template) return;
  copyRules.value = template.rules.map((r) => ({ find: r.find, replace: r.replace }));
  copyTemplateName.value = template.name;
});

watch(project, (next) => {
  if (!next) {
    deviceEdit.value = { name: "", workbookName: "" };
    return;
  }
  const active = activeDevice.value;
  deviceEdit.value = {
    name: active?.deviceName ?? "",
    workbookName: active?.workbookName ?? "",
  };
});

watch(activeDevice, (next) => {
  deviceEdit.value = {
    name: next?.deviceName ?? "",
    workbookName: next?.workbookName ?? "",
  };
});
</script>

<template>
  <section class="comm-panel comm-animate" style="--delay: 60ms">
    <div class="comm-panel-header">
      <div class="comm-section-title">设备</div>
      <el-space wrap>
        <el-button size="small" type="primary" @click="openAddDialog">新增</el-button>
        <el-button size="small" :disabled="!activeDevice" @click="openCopyDialog">复制</el-button>
        <el-button size="small" type="danger" :disabled="!activeDevice" @click="confirmDeleteDevice">删除</el-button>
      </el-space>
    </div>

    <el-menu class="comm-device-menu" :default-active="activeDeviceId" @select="selectDevice">
      <el-menu-item v-for="d in devices" :key="d.deviceId" :index="d.deviceId">
        <div class="comm-device-item">
          <span class="comm-device-name">{{ d.deviceName }}</span>
          <span class="comm-device-meta">{{ d.points.points.length }} 点位</span>
        </div>
      </el-menu-item>
    </el-menu>

    <el-form label-width="72px" class="comm-form-compact" style="margin-top: 10px">
      <el-form-item label="名称">
        <el-input v-model="deviceEdit.name" :disabled="!activeDevice" />
      </el-form-item>
      <el-form-item label="Workbook">
        <el-input v-model="deviceEdit.workbookName" :disabled="!activeDevice" />
      </el-form-item>
    </el-form>

    <div class="comm-panel-actions">
      <el-button size="small" type="primary" :disabled="!deviceDirty" @click="saveDeviceMeta">保存设备</el-button>
    </div>

    <el-alert
      v-if="devices.length === 0"
      type="warning"
      show-icon
      :closable="false"
      title="当前工程没有设备，请先新增设备"
      style="margin-top: 12px"
    />
  </section>

  <el-dialog v-model="addDialogOpen" width="520px">
    <template #header>新增设备</template>
    <el-form label-width="140px">
      <el-form-item label="设备名称">
        <el-input v-model="addDeviceName" placeholder="例如 Pump-A" />
      </el-form-item>
      <el-form-item label="workbook 名称">
        <el-input :model-value="sanitizeWorkbookName(addDeviceName)" disabled />
      </el-form-item>
      <el-form-item label="连接参数">
        <el-switch v-model="addUseActiveProfile" active-text="复制当前设备连接" inactive-text="使用默认连接" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="addDialogOpen = false">取消</el-button>
      <el-button type="primary" @click="confirmAddDevice">确定</el-button>
    </template>
  </el-dialog>

  <el-dialog v-model="copyDialogOpen" width="780px">
    <template #header>复制设备（替换变量名称）</template>
    <el-form label-width="140px">
      <el-form-item label="源设备">
        <el-select v-model="copySourceDeviceId" style="width: 320px">
          <el-option v-for="d in devices" :key="d.deviceId" :label="d.deviceName" :value="d.deviceId" />
        </el-select>
      </el-form-item>
      <el-form-item label="新设备名称">
        <el-input v-model="copyDeviceName" />
      </el-form-item>
      <el-form-item label="workbook 名称">
        <el-input :model-value="sanitizeWorkbookName(copyDeviceName)" disabled />
      </el-form-item>

      <el-form-item label="模板">
        <el-space wrap>
          <el-select v-model="copyTemplateId" placeholder="选择模板" style="width: 240px">
            <el-option v-for="t in copyTemplates" :key="t.templateId" :label="t.name" :value="t.templateId" />
          </el-select>
          <el-button :disabled="!copyTemplateId" @click="deleteCopyTemplate">删除模板</el-button>
        </el-space>
      </el-form-item>

      <el-form-item label="替换规则">
        <el-table :data="copyRules" size="small" style="width: 100%">
          <el-table-column label="查找">
            <template #default="{ row }">
              <el-input v-model="row.find" />
            </template>
          </el-table-column>
          <el-table-column label="替换">
            <template #default="{ row }">
              <el-input v-model="row.replace" />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="120">
            <template #default="{ $index }">
              <el-button size="small" type="danger" @click="copyRules.splice($index, 1)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button style="margin-top: 8px" @click="copyRules.push({ find: '', replace: '' })">新增规则</el-button>
      </el-form-item>

      <el-form-item label="保存模板">
        <el-space wrap>
          <el-input v-model="copyTemplateName" placeholder="模板名称" style="width: 240px" />
          <el-button @click="saveCopyTemplate">保存为模板</el-button>
        </el-space>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="copyDialogOpen = false">取消</el-button>
      <el-button type="primary" @click="confirmCopyDevice">确定</el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.comm-device-menu {
  border-right: none;
  background: transparent;
  max-height: 36vh;
  overflow: auto;
  padding-right: 4px;
}

:deep(.comm-device-menu) {
  background-color: transparent;
}

:deep(.comm-device-menu .el-menu-item) {
  height: auto;
  line-height: 1.2;
  padding: 10px 12px;
  border-radius: 12px;
  margin-bottom: 6px;
}

:deep(.comm-device-menu .el-menu-item.is-active) {
  background: rgba(31, 94, 107, 0.16);
  color: var(--comm-text);
}

:deep(.comm-device-menu .el-menu-item:hover) {
  background: rgba(31, 94, 107, 0.1);
}

.comm-device-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.comm-device-name {
  font-weight: 600;
}

.comm-device-meta {
  font-size: 12px;
  color: var(--comm-muted);
}
</style>
