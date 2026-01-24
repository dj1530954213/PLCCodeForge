<script setup lang="ts">
import { computed, ref } from "vue";
import Grid, { VGridVueEditor, type Editors } from "@revolist/vue3-datagrid";
import type { ColumnAutoSizeMode } from "@revolist/revogrid";

import TextEditor from "../components/revogrid/TextEditor.vue";
import SelectEditor from "../components/revogrid/SelectEditor.vue";
import NumberEditor from "../components/revogrid/NumberEditor.vue";
import BatchEditDialog from "../components/BatchEditDialog.vue";
import PointsHeader from "../components/points/PointsHeader.vue";
import RunPanel from "../components/points/RunPanel.vue";
import ValidationBar from "../components/points/ValidationBar.vue";
import ValidationDrawer from "../components/points/ValidationDrawer.vue";

import { COMM_BYTE_ORDERS_32, COMM_DATA_TYPES } from "../constants";
import { spanForArea } from "../services/address";
import { getSupportedDataTypes } from "../services/dataTypes";
import { usePointsRun } from "../composables/usePointsRun";
import { usePointsGrid } from "../composables/usePointsGrid";
import { usePointsFill } from "../composables/usePointsFill";
import { usePointsUndo } from "../composables/usePointsUndo";
import { usePointsBatchOps } from "../composables/usePointsBatchOps";
import { usePointsRows } from "../composables/usePointsRows";
import { usePointsRowOps } from "../composables/usePointsRowOps";
import { usePointsPersistence } from "../composables/usePointsPersistence";
import { usePointsColumns } from "../composables/usePointsColumns";
import { usePointsGridEvents } from "../composables/usePointsGridEvents";
import { usePointsLifecycle } from "../composables/usePointsLifecycle";
import {
  formatBackendReason,
  formatFieldLabel,
  usePointsValidation,
} from "../composables/usePointsValidation";

import type {
  ByteOrder32,
  CommPoint,
  ConnectionProfile,
  DataType,
  PointsV1,
  ProfilesV1,
  RegisterArea,
} from "../api";
import { useCommDeviceContext } from "../composables/useDeviceContext";
import { useCommWorkspaceRuntime } from "../composables/useWorkspaceRuntime";

const { projectId, project, activeDeviceId, activeDevice } = useCommDeviceContext();
const workspaceRuntime = useCommWorkspaceRuntime();

const BYTE_ORDERS: ByteOrder32[] = COMM_BYTE_ORDERS_32;

interface BackendFieldIssue {
  pointKey?: string;
  hmiName?: string;
  field: string;
  reason?: string;
}

type PointRow = CommPoint & {
  __selected: boolean;
  modbusAddress: string;
  quality: string;
  valueDisplay: string;
  errorMessage: string;
  timestamp: string;
  durationMs: number | "";
};

const gridRef = ref<any>(null);
const profiles = ref<ProfilesV1>({ schemaVersion: 1, profiles: [] });
const points = ref<PointsV1>({ schemaVersion: 1, points: [] });

const gridAutoSizeColumn = {
  mode: "autoSizeOnTextOverlap" as ColumnAutoSizeMode,
};

const activeChannelName = ref<string>("");
const gridRows = ref<PointRow[]>([]);

const showAllValidation = ref(false);
const touchedRowKeys = ref<Record<string, boolean>>({});
const pointsRevision = ref(0);
const validationPanelOpen = ref(false);
const focusedIssueCell = ref<{ pointKey: string; field: keyof PointRow } | null>(null);
const suppressChannelWatch = ref(false);

function markTouchedKeys(keys: string[]) {
  const next = { ...touchedRowKeys.value };
  for (const key of keys) next[String(key)] = true;
  touchedRowKeys.value = next;
}

let rowOps: ReturnType<typeof usePointsRowOps> | null = null;

async function appendRows(count: number, baseRow?: PointRow | null) {
  if (!rowOps) return;
  await rowOps.appendRows(count, baseRow);
}

const {
  selectedRangeRows,
  effectiveSelectedKeySet,
  effectiveSelectedRows,
  selectedCount,
  onGridSetRange,
  onGridSelectionChange,
  onGridClearRegion,
  gridApi,
  attachGridSelectionListeners,
  detachSelectionListeners: detachGridSelectionListeners,
  getSelectedRange,
  getRowClass,
} = usePointsGrid({
  gridRef,
  gridRows,
  appendRows,
  clearFocus: () => {
    focusedIssueCell.value = null;
  },
});

const activeProfile = computed<ConnectionProfile | null>(() => {
  const name = activeChannelName.value;
  if (!name) return null;
  return profiles.value.profiles.find((p) => p.channelName === name) ?? null;
});

const dataTypeOptions = computed<DataType[]>(() => {
  const profile = activeProfile.value;
  return profile ? getSupportedDataTypes(profile.readArea) : COMM_DATA_TYPES;
});

const {
  addressConflictByPointKey,
  hmiDuplicateByPointKey,
  validateHmiName,
  validateScale,
  validateModbusAddress,
  validateRowForRun,
  validationIssues,
  validationIssuesView,
  validationIssueByPointKey,
  hasValidationIssues,
} = usePointsValidation({
  gridRows,
  points,
  project,
  activeDeviceId,
  activeProfile,
});

function getValidationError(): string | null {
  return gridRows.value.map(validateRowForRun).find((v) => Boolean(v)) ?? null;
}

function resolveDataTypeForArea(area: RegisterArea, preferred?: DataType | null): DataType {
  const supported = getSupportedDataTypes(area);
  if (preferred && supported.includes(preferred)) return preferred;
  return supported[0] ?? preferred ?? "UInt16";
}

const backendFieldIssues = computed<BackendFieldIssue[]>(() => runError.value?.details?.missingFields ?? []);
const backendFieldIssuesView = computed(() =>
  backendFieldIssues.value.map((issue) => ({
    ...issue,
    fieldLabel: formatFieldLabel(issue.field),
    reasonLabel: formatBackendReason(issue.reason),
  }))
);
const hasBackendFieldIssues = computed(() => backendFieldIssues.value.length > 0);

const validationSummary = computed(() => {
  if (!hasValidationIssues.value && !hasBackendFieldIssues.value) {
    return "当前无阻断错误";
  }
  const parts: string[] = [];
  if (hasValidationIssues.value) {
    parts.push(`前端校验 ${validationIssues.value.length} 条`);
  }
  if (hasBackendFieldIssues.value) {
    parts.push(`后端校验 ${backendFieldIssues.value.length} 条`);
  }
  return `运行已阻止 · ${parts.join(" / ")}`;
});

const EDITOR_TEXT = "comm-text";
const EDITOR_SELECT = "comm-select";
const EDITOR_NUMBER = "comm-number";
const COL_ROW_SELECTED = "__selected";

const gridEditors: Editors = {
  [EDITOR_TEXT]: VGridVueEditor(TextEditor),
  [EDITOR_SELECT]: VGridVueEditor(SelectEditor),
  [EDITOR_NUMBER]: VGridVueEditor(NumberEditor),
};

const { columns, colIndexByProp } = usePointsColumns<PointRow>({
  dataTypeOptions,
  byteOrders: BYTE_ORDERS,
  showAllValidation,
  touchedRowKeys,
  focusedIssueCell,
  hmiDuplicateByPointKey,
  addressConflictByPointKey,
  validationIssueByPointKey,
  validateHmiName,
  validateScale,
  validateModbusAddress,
});

const { syncFromGridAndMapAddresses, applyLatestToGridRows, rebuildPlan } = usePointsRows({
  gridRows,
  points,
  activeChannelName,
  activeProfile,
  projectId,
  activeDeviceId,
  gridApi,
  colIndexByProp,
  onTouched: markTouchedKeys,
});

const {
  runUiState,
  runId,
  latest,
  runError,
  runErrorTitle,
  pollMs,
  autoRestartPending,
  resumeAfterFix,
  configChangedDuringRun,
  isStarting,
  isRunning,
  isStopping,
  pushLog,
  startRun,
  stopRun,
  markPointsChanged,
  dispose: disposeRun,
} = usePointsRun({
  projectId,
  activeDeviceId,
  activeProfile,
  points,
  pointsRevision,
  showAllValidation,
  getValidationError,
  syncFromGridAndMapAddresses,
  onLatestResults: applyLatestToGridRows,
  workspaceRuntime,
});

function updatePollMs(value: number) {
  pollMs.value = value;
}

const {
  fillMode,
  fillModeLabel,
  handleFillCommand,
  applyFill,
  applyFillDown,
  applyFillSeries,
} = usePointsFill({
  gridRows,
  columns,
  colIndexByProp,
  activeProfile,
  gridApi,
  getSelectedRange,
  syncFromGridAndMapAddresses,
  markPointsChanged,
  rowSelectedProp: COL_ROW_SELECTED,
});

const { undoManager, handleUndo, handleRedo } = usePointsUndo({
  onAfterChange: () => {
    void rebuildPlan();
  },
});

const {
  batchAddDrawerOpen,
  batchAddTemplate,
  batchAddPreview,
  openBatchAddDialog,
  confirmBatchAdd,
  batchEditDialogVisible,
  openBatchEditDialog,
  handleBatchEditConfirm,
  replaceDialogOpen,
  replaceForm,
  replacePreview,
  replacePreviewLimit,
  openReplaceDialog,
  confirmReplaceHmiNames,
} = usePointsBatchOps({
  gridRows,
  points,
  activeProfile,
  selectedCount,
  effectiveSelectedKeySet,
  selectedRangeRows,
  getSelectedRange,
  gridApi,
  undoManager,
  markPointsChanged,
  rebuildPlan,
  resolveDataTypeForArea,
  projectId,
  pushLog,
  onTouched: markTouchedKeys,
});

rowOps = usePointsRowOps({
  gridRows,
  points,
  activeProfile,
  activeChannelName,
  selectedCount,
  effectiveSelectedKeySet,
  effectiveSelectedRows,
  selectedRangeRows,
  getSelectedRange,
  gridApi,
  undoManager,
  markPointsChanged,
  rebuildPlan,
  resolveDataTypeForArea,
});

const { addSingleRow, removeSelectedRows } = rowOps;

const { loadAll, savePoints } = usePointsPersistence({
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
});

const { onAfterEdit, onBeforeGridKeyDown, onBeforeAutofill, handleJumpToIssue } = usePointsGridEvents<PointRow>({
  gridRows,
  selectedRangeRows,
  focusedIssueCell,
  validationPanelOpen,
  colIndexByProp,
  gridApi,
  fillMode,
  applyFillDown,
  applyFillSeries,
  appendRows,
  syncFromGridAndMapAddresses,
  markPointsChanged,
  handleUndo,
  handleRedo,
});

usePointsLifecycle({
  projectId,
  activeDeviceId,
  activeDevice,
  activeChannelName,
  profiles,
  selectedRangeRows,
  suppressChannelWatch,
  rebuildPlan,
  loadAll,
  pushLog,
  workspaceRuntime,
  attachGridSelectionListeners,
  detachGridSelectionListeners: () => {
    detachGridSelectionListeners?.();
  },
  disposeRun,
  shortcuts: {
    onBatchAdd: openBatchAddDialog,
    onBatchEdit: openBatchEditDialog,
    onDelete: removeSelectedRows,
    onUndo: handleUndo,
    onRedo: handleRedo,
    onSave: savePoints,
  },
});
</script>

<template>
  <div class="comm-subpage comm-subpage--points">
    <PointsHeader :active-device-name="activeDevice?.deviceName" @load="loadAll" @save="savePoints" />

      <el-alert
        class="comm-hint-bar comm-animate"
        type="info"
        show-icon
        :closable="false"
        title="提示：表格支持直接编辑、TSV 粘贴；框选区域后使用“填充”进行同值或序列递增。"
        style="--delay: 60ms"
      />

      <div class="comm-points-stack">
        <RunPanel
          :active-profile="activeProfile"
          :is-starting="isStarting"
          :is-running="isRunning"
          :is-stopping="isStopping"
          :poll-ms="pollMs"
          :run-ui-state="runUiState"
          :config-changed-during-run="configChangedDuringRun"
          :auto-restart-pending="autoRestartPending"
          :run-id="runId"
          :resume-after-fix="resumeAfterFix"
          :latest-updated-at="latest?.updatedAtUtc"
          @start="startRun"
          @stop="stopRun('manual')"
          @update:poll-ms="updatePollMs"
        />

        <el-alert
          v-if="runError"
          class="comm-run-error comm-animate"
          style="--delay: 140ms"
          type="error"
          show-icon
          :closable="false"
          :title="runErrorTitle"
        />

        <ValidationBar
          :summary="validationSummary"
          :has-issues="hasValidationIssues || hasBackendFieldIssues"
          @open="validationPanelOpen = true"
        />

        <section class="comm-panel comm-panel--table comm-animate comm-points-main" style="--delay: 180ms">
          <div class="comm-toolbar">
            <div class="comm-toolbar-left">
            <el-button type="primary" @click="addSingleRow">新增单行</el-button>
            <el-button @click="openBatchAddDialog">批量新增 (Ctrl+B)</el-button>
            <el-button :disabled="gridRows.length === 0" @click="openBatchEditDialog">批量编辑 (Ctrl+E)</el-button>
            <el-button :disabled="gridRows.length === 0" @click="openReplaceDialog">变量名替换</el-button>
            <el-dropdown
              split-button
              type="default"
              :disabled="gridRows.length === 0"
                @click="applyFill"
                @command="handleFillCommand"
              >
                {{ fillModeLabel }}
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="copy">同值填充</el-dropdown-item>
                    <el-dropdown-item command="series">序列递增</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
            <div class="comm-toolbar-right">
              <el-button type="danger" :disabled="gridRows.length === 0" @click="removeSelectedRows">删除选中行（{{ selectedCount }}）(Del)</el-button>
              <el-button :disabled="!undoManager.canUndo()" @click="handleUndo">撤销 (Ctrl+Z)</el-button>
              <el-button :disabled="!undoManager.canRedo()" @click="handleRedo">重做 (Ctrl+Y)</el-button>
            </div>
          </div>
          <div class="comm-toolbar-tip">
            <span>选中 {{ selectedCount }} 行</span>
            <span>快捷键：Ctrl+B / Ctrl+E / Del / Ctrl+Z / Ctrl+Y</span>
          </div>

          <div class="comm-grid-layer">
            <Grid
              ref="gridRef"
              :source="gridRows"
              :columns="columns"
              :editors="gridEditors"
              :range="true"
              :useClipboard="true"
              :canFocus="true"
              :autoSizeColumn="gridAutoSizeColumn"
              :stretch="true"
              :resize="true"
              :rowHeaders="true"
              :rowClass="getRowClass"
              class="comm-grid"
              style="height: clamp(460px, 68vh, 820px); width: 100%"
              @beforeautofill="onBeforeAutofill"
              @afteredit="onAfterEdit"
              @beforekeydown="onBeforeGridKeyDown"
              @setrange="onGridSetRange"
              @selectionchangeinit="onGridSelectionChange"
              @clearregion="onGridClearRegion"
            />
          </div>
        </section>
      </div>

    <ValidationDrawer
      v-model="validationPanelOpen"
      :validation-issues="validationIssuesView"
      :has-validation-issues="hasValidationIssues"
      :backend-issues="backendFieldIssuesView"
      :has-backend-issues="hasBackendFieldIssues"
      @jump="handleJumpToIssue"
    />

    <el-dialog v-model="batchAddDrawerOpen" title="批量新增（模板）" width="980px">
      <el-row :gutter="12">
        <el-col :span="12">
          <el-form label-width="140px">
            <el-form-item label="行数（N）">
              <el-input-number v-model="batchAddTemplate.count" :min="1" :max="500" />
            </el-form-item>
            <el-form-item label="起始点位地址（从 1 开始）">
              <el-input v-model="batchAddTemplate.startAddressHuman" placeholder="例如 1" />
            </el-form-item>
            <el-form-item label="数据类型（步长）">
              <el-select v-model="batchAddTemplate.dataType" style="width: 220px">
                <el-option v-for="opt in dataTypeOptions" :key="opt" :label="opt" :value="opt" />
              </el-select>
              <el-tag v-if="activeProfile" type="info" style="margin-left: 10px"
                >步长={{ spanForArea(activeProfile.readArea, batchAddTemplate.dataType) ?? "?" }}</el-tag
              >
            </el-form-item>
            <el-form-item label="字节序">
              <el-select v-model="batchAddTemplate.byteOrder" style="width: 220px">
                <el-option v-for="opt in BYTE_ORDERS" :key="opt" :label="opt" :value="opt" />
              </el-select>
            </el-form-item>
            <el-form-item label="变量名称（HMI）模板">
              <el-input v-model="batchAddTemplate.hmiNameTemplate" placeholder="例如 AI_{{i}} 或 AI_{{addr}}" />
              <div style="margin-top: 6px; color: var(--el-text-color-secondary); font-size: 12px">
                支持占位符：<code v-pre>{{i}}</code>（从 1 开始） / <code v-pre>{{addr}}</code>（如 1）
              </div>
            </el-form-item>
            <el-form-item label="缩放倍数模板">
              <el-input v-model="batchAddTemplate.scaleTemplate" placeholder="例如 1 或 {{i}}" />
              <div style="margin-top: 6px; color: var(--el-text-color-secondary); font-size: 12px">
                当前仅支持数字或 <code v-pre>{{i}}</code>
              </div>
            </el-form-item>
            <el-form-item label="插入位置">
              <el-radio-group v-model="batchAddTemplate.insertMode">
                <el-radio-button label="append">追加到末尾</el-radio-button>
                <el-radio-button label="afterSelection">插入到选中行之后</el-radio-button>
              </el-radio-group>
            </el-form-item>
          </el-form>
        </el-col>

        <el-col :span="12">
          <el-card shadow="never">
            <template #header>预览（前 10 行）</template>

            <el-alert v-if="!batchAddPreview.ok" type="error" :closable="false" show-icon :title="batchAddPreview.message" />

            <el-table v-else :data="batchAddPreview.preview" size="small" height="360" style="width: 100%">
              <el-table-column prop="i" label="#" width="60" />
              <el-table-column prop="hmiName" label="变量名称（HMI）" min-width="160" />
              <el-table-column prop="modbusAddress" label="点位地址" width="110" />
              <el-table-column prop="dataType" label="数据类型" width="100" />
              <el-table-column prop="byteOrder" label="字节序" width="100" />
              <el-table-column prop="scale" label="缩放倍数" width="100" />
            </el-table>
          </el-card>
        </el-col>
      </el-row>

      <template #footer>
        <el-button @click="batchAddDrawerOpen = false">取消</el-button>
        <el-button type="primary" @click="confirmBatchAdd">生成并插入</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="replaceDialogOpen" title="变量名称批量替换" width="720px">
      <el-form label-width="120px">
        <el-form-item label="查找内容">
          <el-input v-model="replaceForm.find" placeholder="例如 AI_" />
        </el-form-item>
        <el-form-item label="替换为">
          <el-input v-model="replaceForm.replace" placeholder="例如 DI_" />
        </el-form-item>
        <el-form-item label="替换范围">
          <el-radio-group v-model="replaceForm.scope">
            <el-radio-button label="selected" :disabled="selectedCount === 0">仅选中行</el-radio-button>
            <el-radio-button label="all">当前通道全部</el-radio-button>
          </el-radio-group>
          <div class="comm-replace-hint">
            {{ replaceForm.scope === "selected" ? `已选中 ${selectedCount} 行` : `当前通道 ${gridRows.length} 行` }}
          </div>
        </el-form-item>
      </el-form>

      <el-card shadow="never">
        <template #header>预览</template>
        <div class="comm-replace-summary">
          匹配 {{ replacePreview.matchedRows }} 行 / 替换 {{ replacePreview.replaceCount }} 处（仅展示前 {{ replacePreviewLimit }} 行）
        </div>
        <el-table v-if="replacePreview.preview.length > 0" :data="replacePreview.preview" size="small" style="width: 100%" height="260">
          <el-table-column prop="rowIndex" label="#" width="60" />
          <el-table-column prop="before" label="原变量名" min-width="180" />
          <el-table-column prop="after" label="替换后" min-width="180" />
          <el-table-column prop="count" label="替换次数" width="100" />
        </el-table>
        <el-empty v-else description="暂无匹配预览" />
      </el-card>

      <template #footer>
        <el-button @click="replaceDialogOpen = false">取消</el-button>
        <el-button type="primary" @click="confirmReplaceHmiNames">应用替换</el-button>
      </template>
    </el-dialog>

    <BatchEditDialog
      v-model="batchEditDialogVisible"
      :selected-count="selectedCount"
      :selected-rows="effectiveSelectedRows"
      :data-type-options="dataTypeOptions"
      @confirm="handleBatchEditConfirm"
    />
  </div>
</template>

<style scoped>
.comm-replace-hint {
  margin-top: 6px;
  font-size: 12px;
  color: var(--comm-muted);
}

.comm-replace-summary {
  margin-bottom: 8px;
  font-size: 12px;
  color: var(--comm-muted);
}

.comm-points-stack {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

@media (max-width: 1100px) {
  :deep(.comm-run-grid) {
    grid-template-columns: 1fr;
  }

  :deep(.comm-run-actions) {
    justify-content: flex-start;
  }
}

:deep(.comm-grid) {
  width: 100%;
}

:deep(.comm-grid .rgHeaderCell) {
  background: #e6eef2;
  color: var(--comm-text);
  font-weight: 600;
  font-size: 14px;
  border-bottom: 1px solid var(--comm-border);
}

:deep(.comm-grid .rgCell) {
  font-size: 14px;
  padding: 0 8px;
  border-color: rgba(201, 213, 220, 0.8);
  background: #ffffff;
  color: var(--comm-text);
  font-variant-numeric: tabular-nums;
}

:deep(.comm-grid .rgRow:nth-child(even) .rgCell) {
  background: #f4f8fa;
}

:deep(.comm-grid .rgRow:hover .rgCell) {
  background: rgba(31, 94, 107, 0.1);
}

:deep(.comm-cell-error) {
  background: rgba(245, 108, 108, 0.14);
  box-shadow: inset 0 0 0 1px rgba(245, 108, 108, 0.85);
  animation: comm-blink 1.2s ease-in-out infinite;
}

:deep(.comm-cell-focus) {
  box-shadow: inset 0 0 0 2px rgba(245, 108, 108, 0.95);
  background: rgba(245, 108, 108, 0.2);
  animation: comm-focus-pulse 1.1s ease-in-out infinite;
}

@keyframes comm-blink {
  0%,
  100% {
    background-color: rgba(245, 108, 108, 0.12);
  }
  50% {
    background-color: rgba(245, 108, 108, 0.32);
  }
}

@keyframes comm-focus-pulse {
  0%,
  100% {
    box-shadow: inset 0 0 0 2px rgba(245, 108, 108, 0.95);
  }
  50% {
    box-shadow: inset 0 0 0 3px rgba(245, 108, 108, 0.8);
  }
}

@media (prefers-reduced-motion: reduce) {
  :deep(.comm-cell-error),
  :deep(.comm-cell-focus) {
    animation: none;
  }
}

:deep(.comm-rg-editor) {
  width: 100%;
  box-sizing: border-box;
  height: 30px;
  padding: 0 8px;
  border: 1px solid rgba(201, 213, 220, 0.9);
  border-radius: 8px;
  font-size: 14px;
  outline: none;
  background: #ffffff;
  color: var(--comm-text);
}

:deep(.comm-rg-editor:focus) {
  border-color: var(--comm-primary);
  box-shadow: 0 0 0 2px rgba(31, 94, 107, 0.2);
}

:deep(.rgHeaderCell[data-type="rowHeaders"]) {
  cursor: pointer;
  user-select: none;
  transition: background-color 0.2s;
}

:deep(.rgHeaderCell[data-type="rowHeaders"]:hover) {
  background-color: rgba(31, 94, 107, 0.12);
}

:deep(.rowHeaders .rgCell) {
  cursor: pointer;
  user-select: none;
  transition: background-color 0.2s;
}

:deep(.rowHeaders .rgCell:hover) {
  background-color: rgba(31, 94, 107, 0.1);
}

:deep(.row-selected) {
  background-color: rgba(111, 183, 177, 0.22) !important;
}

:deep(.row-selected .rgHeaderCell[data-type="rowHeaders"]) {
  background-color: rgba(111, 183, 177, 0.3) !important;
  font-weight: 600;
  color: var(--comm-primary-ink);
}

:deep(.row-selected .rgCell) {
  background-color: rgba(111, 183, 177, 0.18) !important;
}

</style>
