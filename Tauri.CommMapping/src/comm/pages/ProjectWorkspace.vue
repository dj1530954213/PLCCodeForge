<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";

import type {
  CommDeviceCopyRuleV1,
  CommDeviceCopyTemplateV1,
  CommDeviceV1,
  CommProjectDataV1,
  ConnectionProfile,
} from "../api";
import { provideCommDeviceContext } from "../composables/useDeviceContext";

const route = useRoute();
const router = useRouter();

const projectId = computed(() => String(route.params.projectId ?? ""));
const {
  project,
  devices,
  activeDeviceId,
  activeDevice,
  saveProject,
} = provideCommDeviceContext(projectId);

const totalPoints = computed(() => (project.value?.devices ?? []).reduce((sum, d) => sum + d.points.points.length, 0));
const activePointCount = computed(() => activeDevice.value?.points.points.length ?? 0);
const activeProfile = computed(() => activeDevice.value?.profile ?? null);
const activeChannelName = computed(() => project.value?.uiState?.activeChannelName ?? activeProfile.value?.channelName ?? "");

const tabs = computed(() => {
  const pid = projectId.value;
  return [
    { name: "connection", label: "连接参数", path: `/projects/${pid}/comm/connection` },
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
    return "connection";
  },
  set(name) {
    const tab = tabs.value.find((t) => t.name === name);
    if (tab) router.push(tab.path);
  },
});

const activeTabLabel = computed(() => {
  const tab = tabs.value.find((t) => t.name === activeTab.value);
  return tab?.label ?? "连接参数";
});

const connectionSummary = computed(() => {
  const profile = activeProfile.value;
  if (!profile) return [];
  const base = [
    { label: "协议", value: profile.protocolType },
    { label: "通道", value: activeChannelName.value || "--" },
    { label: "区域", value: profile.readArea },
  ];
  if (profile.protocolType === "TCP") {
    return [
      ...base,
      { label: "IP", value: profile.ip },
      { label: "端口", value: String(profile.port) },
      { label: "超时", value: `${profile.timeoutMs} ms` },
      { label: "重试", value: String(profile.retryCount) },
      { label: "采样间隔", value: `${profile.pollIntervalMs} ms` },
    ];
  }
  return [
    ...base,
    { label: "串口", value: profile.serialPort },
    { label: "波特率", value: String(profile.baudRate) },
    { label: "校验", value: profile.parity },
    { label: "数据位", value: String(profile.dataBits) },
    { label: "停止位", value: String(profile.stopBits) },
    { label: "超时", value: `${profile.timeoutMs} ms` },
    { label: "重试", value: String(profile.retryCount) },
    { label: "采样间隔", value: `${profile.pollIntervalMs} ms` },
  ];
});

const workspaceTips = computed(() => {
  switch (activeTab.value) {
    case "connection":
      return [
        "先确认协议与寄存器区域，再录入点位地址",
        "地址不连续时可逐点配置，避免使用连续区间假设",
        "通道名称建议与现场设备一致，便于导出对齐",
      ];
    case "points":
      return [
        "建议按设备功能分组命名变量，减少后续查找成本",
        "批量新增前先确认数据类型步长，避免地址跳跃错误",
        "可用“批量替换”快速迁移变量前缀",
      ];
    case "run":
      return [
        "运行前先做配置校验，确保无阻断错误",
        "通讯异常时优先检查端口/串口与超时设置",
        "重点关注解析错误与配置错误的占比",
      ];
    case "export":
      return [
        "导出前确认变量名称与设备名称一致",
        "导出结果以冻结表头为准，请勿手动修改表头",
      ];
    case "advanced":
      return [
        "高级配置主要用于联调与集成场景",
        "修改后建议先在测试通道验证",
      ];
    default:
      return [];
  }
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
  if (next) {
    localStorage.setItem("comm.lastProjectId", next.projectId);
  }
});

</script>

<template>
  <div class="comm-page comm-page--workspace">
    <div class="comm-shell comm-shell--wide">
      <header class="comm-hero comm-animate" style="--delay: 0ms">
        <div class="comm-hero-title">
          <div class="comm-title">工程：{{ project?.name ?? "未找到" }}</div>
          <div class="comm-subtitle">设备：{{ activeDevice?.deviceName ?? "未选择" }}</div>
          <div class="comm-inline-meta">
            <span>projectId: {{ projectId }}</span>
            <span>设备数: {{ devices.length }}</span>
          </div>
        </div>
        <div class="comm-hero-actions">
          <el-button @click="router.push('/projects')">返回工程列表</el-button>
        </div>
      </header>

      <div class="comm-workspace-grid">
        <aside class="comm-workspace-side">
          <section class="comm-panel comm-animate" style="--delay: 40ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">项目</div>
            </div>
            <div class="comm-info-list">
              <div class="comm-info-row">
                <span class="comm-info-label">名称</span>
                <span class="comm-info-value">{{ project?.name ?? "未找到" }}</span>
              </div>
              <div class="comm-info-row">
                <span class="comm-info-label">项目ID</span>
                <span class="comm-info-value comm-mono">{{ projectId }}</span>
              </div>
              <div class="comm-info-row">
                <span class="comm-info-label">设备数</span>
                <span class="comm-info-value">{{ devices.length }}</span>
              </div>
            </div>
          </section>

          <section class="comm-panel comm-animate" style="--delay: 60ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">设备列表</div>
              <el-space wrap>
                <el-button type="primary" @click="openAddDialog">新增设备</el-button>
                <el-button :disabled="!activeDevice" @click="openCopyDialog">复制设备</el-button>
                <el-button type="danger" :disabled="!activeDevice" @click="confirmDeleteDevice">删除设备</el-button>
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
            </div>
            <div v-if="connectionSummary.length > 0" class="comm-info-list">
              <div v-for="item in connectionSummary" :key="item.label" class="comm-info-row">
                <span class="comm-info-label">{{ item.label }}</span>
                <span class="comm-info-value">{{ item.value }}</span>
              </div>
            </div>
            <el-empty v-else description="未选择设备或无连接配置" />
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
              <div class="comm-inline-meta">连接 → 点位 → 运行 → 导出</div>
            </div>
            <el-tabs v-model="activeTab" type="card" class="comm-workspace-tabs">
              <el-tab-pane v-for="t in tabs" :key="t.name" :name="t.name" :label="t.label" />
            </el-tabs>
          </section>

          <router-view />
        </main>

        <aside class="comm-workspace-context">
          <section class="comm-panel comm-panel--flat comm-animate" style="--delay: 120ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">工作区提示</div>
              <div class="comm-inline-meta">当前：{{ activeTabLabel }}</div>
            </div>
            <div v-if="workspaceTips.length > 0" class="comm-tip-list">
              <div v-for="(tip, idx) in workspaceTips" :key="idx" class="comm-tip-item">
                <span class="comm-tip-index">{{ idx + 1 }}</span>
                <span class="comm-tip-text">{{ tip }}</span>
              </div>
            </div>
            <el-empty v-else description="暂无提示" />
          </section>

          <section class="comm-panel comm-animate" style="--delay: 140ms">
            <div class="comm-panel-header">
              <div class="comm-section-title">详细信息</div>
            </div>
            <div class="comm-info-list">
              <div class="comm-info-row">
                <span class="comm-info-label">设备</span>
                <span class="comm-info-value">{{ activeDevice?.deviceName ?? "未选择" }}</span>
              </div>
              <div class="comm-info-row">
                <span class="comm-info-label">设备ID</span>
                <span class="comm-info-value comm-mono">{{ activeDevice?.deviceId ?? "--" }}</span>
              </div>
              <div class="comm-info-row">
                <span class="comm-info-label">通道</span>
                <span class="comm-info-value">{{ activeChannelName || "--" }}</span>
              </div>
              <div class="comm-info-row">
                <span class="comm-info-label">区域</span>
                <span class="comm-info-value">{{ activeProfile?.readArea ?? "--" }}</span>
              </div>
            </div>
          </section>

          <section class="comm-panel comm-animate" style="--delay: 160ms">
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

.comm-kpi-desc {
  font-size: 12px;
  color: var(--comm-muted);
  margin-top: 4px;
}

.comm-info-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.comm-info-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid var(--comm-border);
  background: #ffffff;
}

.comm-info-label {
  font-size: 12px;
  color: var(--comm-muted);
}

.comm-info-value {
  font-size: 13px;
  color: var(--comm-text);
}

.comm-tip-list {
  display: grid;
  gap: 8px;
}

.comm-tip-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid var(--comm-border);
  background: #ffffff;
}

.comm-tip-index {
  width: 22px;
  height: 22px;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  background: rgba(31, 94, 107, 0.12);
  color: var(--comm-primary-ink);
  flex-shrink: 0;
}

.comm-tip-text {
  font-size: 13px;
  color: var(--comm-text);
  line-height: 1.5;
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
