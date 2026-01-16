<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";

import type {
  CommDeviceCopyRuleV1,
  CommDeviceCopyTemplateV1,
  CommDeviceV1,
  CommProjectDataV1,
  CommProjectV1,
  ConnectionProfile,
  RegisterArea,
  SerialParity,
} from "../api";
import { commProjectCopy, commProjectCreate, commProjectDelete, commProjectsList } from "../api";
import { provideCommDeviceContext } from "../composables/useDeviceContext";
import { provideCommWorkspaceRuntime } from "../composables/useWorkspaceRuntime";

const route = useRoute();
const router = useRouter();

const projectId = computed(() => String(route.params.projectId ?? ""));
const {
  project,
  devices,
  activeDeviceId,
  activeDevice,
  reloadProject,
  saveProject,
} = provideCommDeviceContext(projectId);

const workspaceRuntime = provideCommWorkspaceRuntime();

const AREA_OPTIONS: RegisterArea[] = ["Holding", "Coil"];
const PARITY_OPTIONS: SerialParity[] = ["None", "Even", "Odd"];

const totalPoints = computed(() => (project.value?.devices ?? []).reduce((sum, d) => sum + d.points.points.length, 0));
const activePointCount = computed(() => activeDevice.value?.points.points.length ?? 0);

const runtimeStats = computed(() => {
  return (
    workspaceRuntime.stats.value ?? {
      total: 0,
      ok: 0,
      timeout: 0,
      commError: 0,
      decodeError: 0,
      configError: 0,
    }
  );
});
const runtimeUpdatedAt = computed(() => workspaceRuntime.updatedAtUtc.value || "--");

const tabs = computed(() => {
  const pid = projectId.value;
  return [
    { name: "points", label: "点位配置", path: `/projects/${pid}/comm/points` },
    { name: "run", label: "运行监控", path: `/projects/${pid}/comm/run` },
    { name: "export", label: "导出与证据包", path: `/projects/${pid}/comm/export` },
    { name: "advanced", label: "高级/集成", path: `/projects/${pid}/comm/advanced` },
  ];
});

const activeTab = computed<string>({
  get() {
    const p = route.path;
    if (p.includes("/comm/points")) return "points";
    if (p.includes("/comm/run")) return "run";
    if (p.includes("/comm/export")) return "export";
    if (p.includes("/comm/advanced")) return "advanced";
    return "points";
  },
  set(name) {
    const tab = tabs.value.find((t) => t.name === name);
    if (tab) router.push(tab.path);
  },
});

const projectList = ref<CommProjectV1[]>([]);
const projectListLoading = ref(false);
const projectCreateOpen = ref(false);
const projectCreateForm = ref({ name: "", device: "", notes: "" });

const selectedProjectId = computed<string>({
  get() {
    return projectId.value;
  },
  set(value) {
    if (!value || value === projectId.value) return;
    router.push(`/projects/${value}/comm/points`);
  },
});

const projectEdit = ref({ name: "", device: "", notes: "" });
const projectDirty = computed(() => {
  const current = project.value;
  if (!current) return false;
  return (
    projectEdit.value.name.trim() !== current.name ||
    projectEdit.value.device.trim() !== (current.device ?? "") ||
    projectEdit.value.notes.trim() !== (current.notes ?? "")
  );
});

const deviceEdit = ref({ name: "", workbookName: "" });
const deviceDirty = computed(() => {
  const current = activeDevice.value;
  if (!current) return false;
  return (
    deviceEdit.value.name.trim() !== current.deviceName ||
    deviceEdit.value.workbookName.trim() !== current.workbookName
  );
});

const profileDraft = ref<ConnectionProfile | null>(null);
const profileBaseline = ref("");
const profileDirty = computed(() => {
  if (!profileDraft.value) return false;
  return JSON.stringify(profileDraft.value) !== profileBaseline.value;
});

async function loadProjectList() {
  projectListLoading.value = true;
  try {
    const resp = await commProjectsList({ includeDeleted: false });
    projectList.value = resp.projects.filter((p) => !p.deletedAtUtc);
  } finally {
    projectListLoading.value = false;
  }
}

async function openCreateProject() {
  projectCreateForm.value = { name: "", device: "", notes: "" };
  projectCreateOpen.value = true;
}

async function confirmCreateProject() {
  const name = projectCreateForm.value.name.trim();
  if (!name) {
    ElMessage.error("工程名称不能为空");
    return;
  }
  const device = projectCreateForm.value.device.trim();
  const notes = projectCreateForm.value.notes.trim();
  const created = await commProjectCreate({
    name,
    device: device ? device : undefined,
    notes: notes ? notes : undefined,
  });
  projectCreateOpen.value = false;
  await loadProjectList();
  router.push(`/projects/${created.projectId}/comm/points`);
}

async function copyProject() {
  const current = project.value;
  if (!current) {
    ElMessage.error("未选择工程");
    return;
  }
  const suggested = `${current.name} (copy)`;
  const name = await ElMessageBox.prompt("输入复制后的工程名称", "复制工程", {
    inputValue: suggested,
    confirmButtonText: "复制",
    cancelButtonText: "取消",
  })
    .then((r) => r.value)
    .catch(() => "");
  if (!name.trim()) return;
  const created = await commProjectCopy({ projectId: current.projectId, name: name.trim() });
  ElMessage.success("已复制工程");
  await loadProjectList();
  router.push(`/projects/${created.projectId}/comm/points`);
}

async function deleteProject() {
  const current = project.value;
  if (!current) {
    ElMessage.error("未选择工程");
    return;
  }
  await ElMessageBox.confirm(`确认删除工程「${current.name}」？（软删）`, "删除工程", {
    confirmButtonText: "删除",
    cancelButtonText: "取消",
    type: "warning",
  });
  await commProjectDelete(current.projectId);
  ElMessage.success("已删除（软删）");
  await loadProjectList();
  const next = projectList.value.find((p) => p.projectId !== current.projectId);
  if (next) {
    router.push(`/projects/${next.projectId}/comm/points`);
  } else {
    router.push("/");
  }
}

async function saveProjectMeta() {
  const current = project.value;
  if (!current) {
    ElMessage.error("未选择工程");
    return;
  }
  const name = projectEdit.value.name.trim();
  if (!name) {
    ElMessage.error("工程名称不能为空");
    return;
  }
  const next: CommProjectDataV1 = {
    ...current,
    name,
    device: projectEdit.value.device.trim() || undefined,
    notes: projectEdit.value.notes.trim() || undefined,
  };
  await saveProject(next);
  await loadProjectList();
  ElMessage.success("工程信息已保存");
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
  const next: CommProjectDataV1 = {
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

watch(projectId, () => {
  void loadProjectList();
}, { immediate: true });

watch(project, (next) => {
  if (!next) {
    projectEdit.value = { name: "", device: "", notes: "" };
    deviceEdit.value = { name: "", workbookName: "" };
    resetProfileDraft();
    return;
  }
  projectEdit.value = {
    name: next.name ?? "",
    device: next.device ?? "",
    notes: next.notes ?? "",
  };
  const active = activeDevice.value;
  deviceEdit.value = {
    name: active?.deviceName ?? "",
    workbookName: active?.workbookName ?? "",
  };
  resetProfileDraft();
});

watch(activeDevice, (next) => {
  deviceEdit.value = {
    name: next?.deviceName ?? "",
    workbookName: next?.workbookName ?? "",
  };
  resetProfileDraft();
});

watch(project, (next) => {
  if (next) {
    localStorage.setItem("comm.lastProjectId", next.projectId);
  }
});

</script>

<template>
  <div class="comm-page comm-page--workspace">
    <div class="comm-shell comm-shell--wide">
      <div class="comm-workspace-grid">
        <aside class="comm-workspace-side">
          <section class="comm-panel comm-animate" style="--delay: 40ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">项目</div>
              <el-space wrap>
                <el-button size="small" type="primary" @click="openCreateProject">新建</el-button>
                <el-button size="small" :disabled="!project" @click="copyProject">复制</el-button>
                <el-button size="small" type="danger" :disabled="!project" @click="deleteProject">删除</el-button>
              </el-space>
            </div>

            <div class="comm-project-overview">
              <div class="comm-project-title">{{ project?.name ?? "未选择工程" }}</div>
              <div class="comm-project-sub">
                设备：{{ activeDevice?.deviceName ?? "未选择" }}
              </div>
              <div class="comm-inline-meta">
                <span>projectId: {{ projectId || "--" }}</span>
                <span>设备数: {{ devices.length }}</span>
              </div>
            </div>

            <el-form label-width="72px" class="comm-form-compact">
              <el-form-item label="选择">
                <el-select v-model="selectedProjectId" filterable placeholder="选择工程" style="width: 100%">
                  <el-option v-for="p in projectList" :key="p.projectId" :label="p.name" :value="p.projectId" />
                </el-select>
              </el-form-item>
              <el-form-item label="名称">
                <el-input v-model="projectEdit.name" />
              </el-form-item>
              <el-form-item label="设备">
                <el-input v-model="projectEdit.device" />
              </el-form-item>
              <el-form-item label="备注">
                <el-input v-model="projectEdit.notes" type="textarea" :rows="2" />
              </el-form-item>
            </el-form>

            <div class="comm-panel-actions">
              <el-button size="small" :loading="projectListLoading" @click="loadProjectList">刷新</el-button>
              <el-button size="small" @click="reloadProject">同步当前</el-button>
              <el-button size="small" type="primary" :disabled="!projectDirty" @click="saveProjectMeta">保存项目</el-button>
            </div>
          </section>

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
                  <el-select v-model="profileDraft.readArea" style="width: 100%">
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
                    <el-select v-model="profileDraft.parity" style="width: 100%">
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

          <section class="comm-panel comm-animate" style="--delay: 100ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">点位统计</div>
            </div>
            <div class="comm-kpi-grid">
              <div class="comm-kpi-item">
                <div class="comm-kpi-label">当前设备点位</div>
                <div class="comm-kpi-value">{{ activePointCount }}</div>
              </div>
              <div class="comm-kpi-item">
                <div class="comm-kpi-label">项目点位总数</div>
                <div class="comm-kpi-value">{{ totalPoints }}</div>
              </div>
            </div>
          </section>
        </aside>

        <main class="comm-workspace-main">
          <section class="comm-panel comm-panel--flat comm-animate" style="--delay: 100ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">功能导航</div>
              <div class="comm-inline-meta">点位 → 运行 → 导出</div>
            </div>
            <el-tabs v-model="activeTab" type="card" class="comm-workspace-tabs">
              <el-tab-pane v-for="t in tabs" :key="t.name" :name="t.name" :label="t.label" />
            </el-tabs>
          </section>

          <router-view />
        </main>

        <aside class="comm-workspace-context">
          <section class="comm-panel comm-panel--stats comm-animate" style="--delay: 120ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">运行统计</div>
              <div class="comm-inline-meta">更新时间：{{ runtimeUpdatedAt }}</div>
            </div>
            <div class="comm-stat-grid">
              <div class="comm-stat"><el-statistic title="总数" :value="runtimeStats.total" /></div>
              <div class="comm-stat"><el-statistic title="正常" :value="runtimeStats.ok" /></div>
              <div class="comm-stat"><el-statistic title="超时" :value="runtimeStats.timeout" /></div>
              <div class="comm-stat"><el-statistic title="通讯错误" :value="runtimeStats.commError" /></div>
              <div class="comm-stat"><el-statistic title="解析错误" :value="runtimeStats.decodeError" /></div>
              <div class="comm-stat"><el-statistic title="配置错误" :value="runtimeStats.configError" /></div>
            </div>
          </section>

          <section class="comm-panel comm-animate" style="--delay: 140ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">报文与诊断</div>
            </div>
            <el-empty description="报文查看功能规划中" />
            <div class="comm-inline-meta" style="margin-top: 8px">
              预留用于展示最近一次请求/响应与通讯诊断信息
            </div>
          </section>
        </aside>
      </div>
    </div>

    <el-dialog v-model="projectCreateOpen" title="新建工程" width="560px">
      <el-form label-width="110px">
        <el-form-item label="工程名称">
          <el-input v-model="projectCreateForm.name" />
        </el-form-item>
        <el-form-item label="设备（可选）">
          <el-input v-model="projectCreateForm.device" />
        </el-form-item>
        <el-form-item label="备注（可选）">
          <el-input v-model="projectCreateForm.notes" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="projectCreateOpen = false">取消</el-button>
        <el-button type="primary" @click="confirmCreateProject">创建并进入</el-button>
      </template>
    </el-dialog>

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
  </div>
</template>

<style scoped>
.comm-workspace-grid {
  display: grid;
  grid-template-columns: minmax(260px, 340px) minmax(0, 1fr) minmax(260px, 340px);
  gap: 16px;
  align-items: start;
}

.comm-workspace-side,
.comm-workspace-main,
.comm-workspace-context {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

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

.comm-project-overview {
  padding: 8px 10px 4px;
  border-radius: 12px;
  border: 1px solid var(--comm-border);
  background: #ffffff;
  margin-bottom: 12px;
}

.comm-project-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--comm-text);
}

.comm-project-sub {
  font-size: 12px;
  color: var(--comm-muted);
  margin-top: 4px;
}

.comm-kpi-grid {
  display: grid;
  gap: 12px;
}

.comm-kpi-item {
  padding: 12px;
  border-radius: 12px;
  border: 1px solid var(--comm-border);
  background: #ffffff;
}

.comm-kpi-label {
  font-size: 12px;
  color: var(--comm-muted);
  letter-spacing: 0.04em;
}

.comm-kpi-value {
  font-size: 18px;
  font-weight: 600;
  margin-top: 4px;
}

.comm-form-compact :deep(.el-form-item) {
  margin-bottom: 10px;
}

.comm-panel-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 6px;
}

.comm-connection-form :deep(.el-input-number),
.comm-connection-form :deep(.el-select) {
  width: 100%;
}

.comm-connection-form :deep(.el-radio-group) {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

@media (max-width: 1400px) {
  .comm-workspace-grid {
    grid-template-columns: minmax(260px, 340px) minmax(0, 1fr);
  }

  .comm-workspace-context {
    display: none;
  }
}

@media (max-width: 1200px) {
  .comm-workspace-grid {
    grid-template-columns: 1fr;
  }
}
</style>
