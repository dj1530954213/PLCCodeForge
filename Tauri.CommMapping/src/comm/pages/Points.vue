<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import Grid, { VGridVueEditor, type ColumnRegular, type Editors } from "@revolist/vue3-datagrid";
import type { ColumnAutoSizeMode } from "@revolist/revogrid";

import TextEditor from "../components/revogrid/TextEditor.vue";
import SelectEditor from "../components/revogrid/SelectEditor.vue";
import NumberEditor from "../components/revogrid/NumberEditor.vue";
import BatchEditDialog from "../components/BatchEditDialog.vue";

import { COMM_BYTE_ORDERS_32, COMM_DATA_TYPES } from "../constants";
import { formatHumanAddressFrom0Based, parseHumanAddress, spanForArea, inferNextAddress } from "../services/address";
import { buildBatchPointsTemplate, previewBatchPointsTemplate } from "../services/batchAdd";
import { computeFillAddressEdits, computeFillDownEdits, type SelectionRange } from "../services/fill";
import { UndoManager, createBatchAddUndoAction, createBatchEditUndoAction, createDeleteRowsUndoAction } from "../services/undoRedo";
import { useKeyboardShortcuts, createStandardShortcuts } from "../composables/useKeyboardShortcuts";
import { computeBatchEdits, applyBatchEdits, type BatchEditRequest } from "../services/batchEdit";
import { getSupportedDataTypes } from "../services/dataTypes";

import type {
  BatchInsertMode,
  ByteOrder32,
  CommPoint,
  CommRunError,
  CommRunLatestResponse,
  ConnectionProfile,
  DataType,
  PointsV1,
  ProfilesV1,
  Quality,
  RegisterArea,
  SampleResult,
} from "../api";
import {
  commPlanBuild,
  commPointsLoad,
  commPointsSave,
  commProjectUiStatePatchV1,
  commProfilesLoad,
  commRunLatestObs,
  commRunStartObs,
  commRunStopObs,
} from "../api";
import { useCommDeviceContext } from "../composables/useDeviceContext";

const { projectId, project, activeDeviceId, activeDevice } = useCommDeviceContext();

const BYTE_ORDERS: ByteOrder32[] = COMM_BYTE_ORDERS_32;

type RunUiState = "idle" | "starting" | "running" | "stopping" | "error";
type LogLevel = "info" | "success" | "warning" | "error";

interface LogEntry {
  ts: string;
  step: string;
  level: LogLevel;
  message: string;
}

interface ValidationIssue {
  pointKey: string;
  hmiName: string;
  modbusAddress: string;
  message: string;
  field?: keyof PointRow;
}

type PointRow = CommPoint & {
  __selected: boolean;
  modbusAddress: string;
  quality: Quality | "";
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

const selectedRangeRows = ref<{ rowStart: number; rowEnd: number } | null>(null);
let lastRowSelectionIndex: number | null = null;

const explicitSelectedKeys = computed<string[]>(() => gridRows.value.filter((r) => r.__selected).map((r) => r.pointKey));

const rangeSelectedKeys = computed<string[]>(() => {
  const span = selectedRangeRows.value;
  if (!span) return [];
  const start = Math.max(0, Math.min(span.rowStart, span.rowEnd));
  const end = Math.min(gridRows.value.length - 1, Math.max(span.rowStart, span.rowEnd));
  if (end < 0 || start > end) return [];
  const out: string[] = [];
  for (let i = start; i <= end; i++) {
    const row = gridRows.value[i];
    if (row) out.push(row.pointKey);
  }
  return out;
});

const effectiveSelectedKeys = computed<string[]>(() => {
  if (rangeSelectedKeys.value.length > 0) return rangeSelectedKeys.value;
  if (explicitSelectedKeys.value.length > 0) return explicitSelectedKeys.value;
  return [];
});

const effectiveSelectedKeySet = computed(() => new Set(effectiveSelectedKeys.value));
const effectiveSelectedRows = computed(() => gridRows.value.filter((r) => effectiveSelectedKeySet.value.has(r.pointKey)));

const selectedCount = computed(() => effectiveSelectedKeys.value.length);

function clearExplicitRowSelection() {
  if (!gridRows.value.some((r) => r.__selected)) return;
  gridRows.value.forEach((r) => {
    r.__selected = false;
  });
  gridRows.value = [...gridRows.value];
}

function applyRangeSelection(range: { y: number; y1: number } | null) {
  if (!range || typeof range.y !== "number" || typeof range.y1 !== "number") return;
  const rowStart = Math.min(range.y, range.y1);
  const rowEnd = Math.max(range.y, range.y1);
  if (!Number.isFinite(rowStart) || !Number.isFinite(rowEnd) || rowEnd < 0) return;
  selectedRangeRows.value = { rowStart: Math.max(0, rowStart), rowEnd: Math.max(0, rowEnd) };
  clearExplicitRowSelection();
  lastRowSelectionIndex = rowEnd;
}

function onGridSetRange(e: any) {
  applyRangeSelection(e?.detail ?? null);
}

function onGridSelectionChange(e: any) {
  applyRangeSelection(e?.detail?.newRange ?? null);
}

function onGridClearRegion() {
  selectedRangeRows.value = null;
}

const showAllValidation = ref(false);
const touchedRowKeys = ref<Record<string, boolean>>({});
const pointsRevision = ref(0);
const validationPanelOpen = ref(false);
const fillMode = ref<"copy" | "series">("copy");
const focusedIssueCell = ref<{ pointKey: string; field: keyof PointRow } | null>(null);

const fillModeLabel = computed(() => (fillMode.value === "copy" ? "同值填充" : "序列递增"));

const start0ByPointKey = ref<Record<string, number>>({});

const runUiState = ref<RunUiState>("idle");
const runId = ref<string | null>(null);
const latest = ref<CommRunLatestResponse | null>(null);
const runError = ref<CommRunError | null>(null);
const logs = ref<LogEntry[]>([]);
const pollMs = ref<number>(1000);
const runtimeByPointKey = ref<Record<string, SampleResult>>({});
const autoRestartPending = ref(false);
const AUTO_RESTART_DELAY_MS = 600;
const resumeAfterFix = ref(false);

const runPointsRevision = ref<number | null>(null);
const configChangedDuringRun = computed(() => {
  return isRunning.value && runPointsRevision.value !== null && pointsRevision.value !== runPointsRevision.value;
});

const isStarting = computed(() => runUiState.value === "starting");
const isRunning = computed(() => runUiState.value === "running");
const isStopping = computed(() => runUiState.value === "stopping");

const activeProfile = computed<ConnectionProfile | null>(() => {
  const name = activeChannelName.value;
  if (!name) return null;
  return profiles.value.profiles.find((p) => p.channelName === name) ?? null;
});

const dataTypeOptions = computed<DataType[]>(() => {
  const profile = activeProfile.value;
  return profile ? getSupportedDataTypes(profile.readArea) : COMM_DATA_TYPES;
});

function resolveDataTypeForArea(area: RegisterArea, preferred?: DataType | null): DataType {
  const supported = getSupportedDataTypes(area);
  if (preferred && supported.includes(preferred)) return preferred;
  return supported[0] ?? preferred ?? "UInt16";
}

function normalizeHmiName(name: string): string {
  return String(name ?? "").trim();
}

const hmiDuplicateByPointKey = computed<Record<string, string>>(() => {
  const out: Record<string, string> = {};
  const devices = project.value?.devices ?? [];
  if (devices.length === 0) return out;

  const byName = new Map<
    string,
    Array<{ deviceId: string; deviceName: string; pointKey: string }>
  >();

  for (const device of devices) {
    const devicePoints =
      device.deviceId === activeDeviceId.value ? points.value.points : device.points.points;
    for (const point of devicePoints) {
      const name = normalizeHmiName(point.hmiName);
      if (!name) continue;
      const list = byName.get(name) ?? [];
      list.push({
        deviceId: device.deviceId,
        deviceName: device.deviceName,
        pointKey: point.pointKey,
      });
      byName.set(name, list);
    }
  }

  for (const list of byName.values()) {
    if (list.length < 2) continue;
    const deviceLabel = Array.from(new Set(list.map((v) => v.deviceName))).join(" / ");
    const message = `HMI 重名：${deviceLabel}`;
    for (const item of list) {
      if (item.deviceId === activeDeviceId.value) {
        out[item.pointKey] = message;
      }
    }
  }

  return out;
});

const addressConflictByPointKey = computed<Record<string, string>>(() => {
  const out: Record<string, string> = {};
  const profile = activeProfile.value;
  if (!profile) return out;

  type Segment = { pointKey: string; start: number; end: number };
  const segments: Segment[] = [];

  for (const row of gridRows.value) {
    const addrRaw = row.modbusAddress.trim();
    if (!addrRaw) continue;
    const parsed = parseHumanAddress(addrRaw, profile.readArea);
    if (!parsed.ok) continue;
    const span = spanForArea(profile.readArea, row.dataType);
    if (span === null) continue;
    const start = parsed.start0Based;
    const end = start + span;
    if (start < profile.startAddress) continue;
    if (end > profile.startAddress + profile.length) continue;
    segments.push({ pointKey: row.pointKey, start, end });
  }

  for (let i = 0; i < segments.length; i++) {
    for (let j = i + 1; j < segments.length; j++) {
      if (segments[i].start < segments[j].end && segments[j].start < segments[i].end) {
        out[segments[i].pointKey] = "地址冲突";
        out[segments[j].pointKey] = "地址冲突";
      }
    }
  }

  return out;
});

const validationIssues = computed<ValidationIssue[]>(() => {
  const out: ValidationIssue[] = [];
  for (const row of gridRows.value) {
    const result = validateRowForRunDetailed(row);
    if (!result) continue;
    out.push({
      pointKey: row.pointKey,
      hmiName: row.hmiName,
      modbusAddress: row.modbusAddress,
      message: result.message,
      field: result.field,
    });
  }
  return out;
});

const validationIssueByPointKey = computed<Record<string, string>>(() => {
  const out: Record<string, string> = {};
  for (const issue of validationIssues.value) {
    out[issue.pointKey] = issue.message;
  }
  return out;
});

const backendFieldIssues = computed(() => runError.value?.details?.missingFields ?? []);
const hasBackendFieldIssues = computed(() => backendFieldIssues.value.length > 0);

const hasValidationIssues = computed(() => validationIssues.value.length > 0);

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

let timer: number | null = null;
function clearTimer() {
  if (timer !== null) {
    window.clearInterval(timer);
    timer = null;
  }
}

function pushLog(step: string, level: LogLevel, message: string) {
  logs.value.unshift({ ts: new Date().toISOString(), step, level, message });
  if (logs.value.length > 20) logs.value.length = 20;
}

let autoRestartTimer: number | null = null;
function clearAutoRestartTimer() {
  if (autoRestartTimer !== null) {
    window.clearTimeout(autoRestartTimer);
    autoRestartTimer = null;
  }
  autoRestartPending.value = false;
}

type AutoRestartMode = "restart" | "start";

function scheduleAutoRestart(reason: string, mode: AutoRestartMode) {
  if (isStarting.value || isStopping.value) return;
  if (mode === "restart" && !isRunning.value) return;
  if (mode === "start" && isRunning.value) return;
  clearAutoRestartTimer();
  autoRestartPending.value = true;
  autoRestartTimer = window.setTimeout(() => {
    autoRestartTimer = null;
    autoRestartPending.value = false;
    pushLog("run_restart", "info", `auto restart: ${reason}`);
    if (mode === "restart") {
      if (!isRunning.value) return;
      void restartRun();
    } else {
      if (isRunning.value) return;
      void startRun();
    }
  }, AUTO_RESTART_DELAY_MS);
}

function markPointsChanged() {
  pointsRevision.value += 1;

  if (hasValidationIssues.value) {
    if (isRunning.value) {
      const first = validationIssues.value[0];
      resumeAfterFix.value = true;
      runError.value = makeUiConfigError(first.message);
      void stopRun("validation");
    }
    return;
  }

  if (resumeAfterFix.value && !isRunning.value) {
    resumeAfterFix.value = false;
    runError.value = null;
    scheduleAutoRestart("config fixed", "start");
    return;
  }

  scheduleAutoRestart("config changed", "restart");
}

function makeUiConfigError(message: string): CommRunError {
  return {
    kind: "ConfigError",
    message,
    details: {
      projectId: projectId.value,
      deviceId: activeDeviceId.value || undefined,
    },
  };
}

function gridApi(): any | null {
  const v = gridRef.value as any;
  if (!v) return null;
  if (
    typeof v.getSource === "function" ||
    typeof v.scrollToRow === "function" ||
    typeof v.getSelectedRange === "function"
  )
    return v;
  if (
    v.$el &&
    (typeof v.$el.getSource === "function" ||
      typeof v.$el.scrollToRow === "function" ||
      typeof v.$el.getSelectedRange === "function")
  )
    return v.$el;
  return v.$el ?? v;
}

function gridElement(): HTMLElement | null {
  const v = gridRef.value as any;
  const el = v?.$el ?? v;
  if (el && typeof el.addEventListener === "function") return el as HTMLElement;
  return null;
}

let detachGridSelectionListeners: (() => void) | null = null;
function attachGridSelectionListeners() {
  if (detachGridSelectionListeners) return;
  const el = gridElement();
  if (!el?.addEventListener) return;

  const onSetRange = (ev: any) => {
    applyRangeSelection(ev?.detail ?? null);
  };

  const onClearRegion = () => {
    selectedRangeRows.value = null;
  };

  const onRowHeaderMouseDown = (ev: MouseEvent) => {
    if (ev.button !== 0) return;
    const target = ev.target as HTMLElement | null;
    if (!target) return;
    const headerRoot = target.closest(".rowHeaders");
    if (!headerRoot) return;
    const cell = target.closest<HTMLElement>("[data-rgRow],[data-rgrow],[data-rg-row]");
    if (!cell) return;
    const rawIndex =
      cell.getAttribute("data-rgRow") ??
      cell.getAttribute("data-rgrow") ??
      cell.getAttribute("data-rg-row");
    const rowIndex = Number(rawIndex);
    if (!Number.isFinite(rowIndex) || rowIndex < 0 || rowIndex >= gridRows.value.length) return;

    const isMultiSelect = ev.ctrlKey || ev.metaKey;
    const isRangeSelect = ev.shiftKey;

    if (isRangeSelect) {
      const anchor = lastRowSelectionIndex ?? rowIndex;
      selectedRangeRows.value = {
        rowStart: Math.min(anchor, rowIndex),
        rowEnd: Math.max(anchor, rowIndex),
      };
      clearExplicitRowSelection();
    } else {
      selectedRangeRows.value = null;
      if (!isMultiSelect) {
        gridRows.value.forEach((r, idx) => {
          r.__selected = idx === rowIndex;
        });
      } else {
        const row = gridRows.value[rowIndex];
        if (row) row.__selected = !row.__selected;
      }
      gridRows.value = [...gridRows.value];
    }

    lastRowSelectionIndex = rowIndex;
    focusedIssueCell.value = null;
    ev.preventDefault();
  };

  let autofillDragging = false;
  let autofillAppendPending = false;
  let lastAutoAppendAt = 0;
  const AUTOFILL_APPEND_CHUNK = 10;
  const AUTOFILL_APPEND_THROTTLE_MS = 200;

  const onAutofillMouseDown = (ev: MouseEvent) => {
    const target = ev.target as HTMLElement | null;
    if (!target) return;
    if (!target.closest(".autofill-handle")) return;
    autofillDragging = true;
    lastAutoAppendAt = 0;
  };

  const onAutofillMouseMove = (ev: MouseEvent) => {
    if (!autofillDragging || ev.buttons !== 1) return;
    const host = gridElement();
    if (!host) return;
    const rect = host.getBoundingClientRect();
    const threshold = 6;
    if (ev.clientY < rect.bottom - threshold) return;
    const now = Date.now();
    if (now - lastAutoAppendAt < AUTOFILL_APPEND_THROTTLE_MS) return;
    if (autofillAppendPending) return;

    lastAutoAppendAt = now;
    autofillAppendPending = true;
    const baseRow = gridRows.value[gridRows.value.length - 1] ?? null;
    appendRows(AUTOFILL_APPEND_CHUNK, baseRow)
      .catch(() => {})
      .finally(() => {
        autofillAppendPending = false;
      });
  };

  const onAutofillMouseUp = () => {
    autofillDragging = false;
  };

  el.addEventListener("setrange", onSetRange as any);
  el.addEventListener("clearregion", onClearRegion as any);
  el.addEventListener("mousedown", onRowHeaderMouseDown as any);
  el.addEventListener("mousedown", onAutofillMouseDown as any);
  document.addEventListener("mousemove", onAutofillMouseMove as any);
  document.addEventListener("mouseup", onAutofillMouseUp as any);

  detachGridSelectionListeners = () => {
    el.removeEventListener("setrange", onSetRange as any);
    el.removeEventListener("clearregion", onClearRegion as any);
    el.removeEventListener("mousedown", onRowHeaderMouseDown as any);
    el.removeEventListener("mousedown", onAutofillMouseDown as any);
    document.removeEventListener("mousemove", onAutofillMouseMove as any);
    document.removeEventListener("mouseup", onAutofillMouseUp as any);
  };
}

function profileLabel(p: ConnectionProfile): string {
  if (p.protocolType === "TCP") {
    return `${p.channelName} / TCP / ${p.ip}:${p.port} / area=${p.readArea} / start=${formatHumanAddressFrom0Based(p.readArea, p.startAddress)} len=${p.length}`;
  }
  return `${p.channelName} / 485 / ${p.serialPort} / area=${p.readArea} / start=${formatHumanAddressFrom0Based(p.readArea, p.startAddress)} len=${p.length}`;
}

function validateHmiName(row: PointRow): string | null {
  return normalizeHmiName(row.hmiName) ? null : "变量名称（HMI）不能为空";
}

function validateScale(row: PointRow): string | null {
  const raw = String(row.scale ?? "").trim();
  if (!raw) return "缩放倍数不能为空";
  return Number.isFinite(Number(raw)) ? null : "缩放倍数必须为有效数字";
}

function validateModbusAddress(row: PointRow): string | null {
  const profile = activeProfile.value;
  if (!profile) return "请先选择连接";

  const len = spanForArea(profile.readArea, row.dataType);
  if (len === null) return `dataType=${row.dataType} 与 readArea=${profile.readArea} 不匹配`;

  const addrRaw = row.modbusAddress.trim();
  if (!addrRaw) return null; // 兼容旧行为：空地址 => addressOffset=None，plan 会按顺排自动映射

  const parsed = parseHumanAddress(addrRaw, profile.readArea);
  if (!parsed.ok) return parsed.message;

  const start0 = parsed.start0Based;
  if (start0 < profile.startAddress) {
    return `地址小于连接起始地址 ${formatHumanAddressFrom0Based(profile.readArea, profile.startAddress)}`;
  }

  const end0 = start0 + len;
  const channelEnd0 = profile.startAddress + profile.length;
  if (end0 > channelEnd0) {
    const endHuman = formatHumanAddressFrom0Based(profile.readArea, end0 - 1);
    const limitHuman = formatHumanAddressFrom0Based(profile.readArea, channelEnd0 - 1);
    return `地址越界：结束地址 ${endHuman} 超出连接范围 ${limitHuman}`;
  }
  return null;
}

function validateRowForRunDetailed(row: PointRow): { message: string; field?: keyof PointRow } | null {
  if (!activeProfile.value) return { message: "请先选择连接" };

  const hmiErr = validateHmiName(row) ?? hmiDuplicateByPointKey.value[row.pointKey];
  if (hmiErr) return { message: hmiErr, field: "hmiName" };

  const scaleErr = validateScale(row);
  if (scaleErr) return { message: scaleErr, field: "scale" };

  const addrErr = validateModbusAddress(row) ?? addressConflictByPointKey.value[row.pointKey];
  if (addrErr) return { message: addrErr, field: "modbusAddress" };

  return null;
}

function validateRowForRun(row: PointRow): string | null {
  return validateRowForRunDetailed(row)?.message ?? null;
}

function rowCellProps(field: keyof PointRow) {
  return ({ model }: any) => {
    const row = model as PointRow;
    const focus = focusedIssueCell.value;
    const isFocused = Boolean(focus && focus.pointKey === row.pointKey && focus.field === field);
    const shouldValidate =
      isFocused ||
      showAllValidation.value ||
      Boolean(touchedRowKeys.value[String(row.pointKey)]) ||
      (field === "hmiName" && Boolean(hmiDuplicateByPointKey.value[row.pointKey])) ||
      (field === "modbusAddress" && Boolean(addressConflictByPointKey.value[row.pointKey])) ||
      Boolean(validationIssueByPointKey.value[row.pointKey]);
    if (!shouldValidate) return {};

    let err: string | null = null;
    if (field === "hmiName") err = validateHmiName(row) ?? hmiDuplicateByPointKey.value[row.pointKey];
    if (field === "scale") err = validateScale(row);
    if (field === "modbusAddress") {
      err = validateModbusAddress(row) ?? addressConflictByPointKey.value[row.pointKey];
    }

    const classes: Record<string, boolean> = {};
    if (err) classes["comm-cell-error"] = true;
    if (isFocused) classes["comm-cell-focus"] = true;
    return Object.keys(classes).length > 0 ? { class: classes, title: err ?? validationIssueByPointKey.value[row.pointKey] } : {};
  };
}

const EDITOR_TEXT = "comm-text";
const EDITOR_SELECT = "comm-select";
const EDITOR_NUMBER = "comm-number";
const COL_ROW_SELECTED = "__selected";

const gridEditors: Editors = {
  [EDITOR_TEXT]: VGridVueEditor(TextEditor),
  [EDITOR_SELECT]: VGridVueEditor(SelectEditor),
  [EDITOR_NUMBER]: VGridVueEditor(NumberEditor),
};

const columns = computed<ColumnRegular[]>(() => [
  {
    prop: "hmiName",
    name: "变量名称（HMI）*",
    size: 220,
    minSize: 160,
    autoSize: true,
    editor: EDITOR_TEXT,
    cellProperties: rowCellProps("hmiName"),
  },
  {
    prop: "modbusAddress",
    name: "起始地址(1-based)",
    size: 120,
    minSize: 110,
    editor: EDITOR_TEXT,
    cellProperties: rowCellProps("modbusAddress"),
  },
  {
    prop: "dataType",
    name: "数据类型",
    size: 110,
    minSize: 100,
    editor: EDITOR_SELECT,
    editorOptions: dataTypeOptions.value.map((v) => ({ label: v, value: v })),
  },
  {
    prop: "byteOrder",
    name: "字节序",
    size: 90,
    minSize: 90,
    editor: EDITOR_SELECT,
    editorOptions: BYTE_ORDERS.map((v) => ({ label: v, value: v })),
  },
  { prop: "scale", name: "缩放倍数", size: 90, minSize: 90, editor: EDITOR_NUMBER, cellProperties: rowCellProps("scale") },
  { prop: "quality", name: "quality", size: 90, minSize: 90, readonly: true },
  { prop: "valueDisplay", name: "实时值", size: 160, minSize: 140, autoSize: true, readonly: true },
  { prop: "timestamp", name: "timestamp", size: 180, minSize: 160, readonly: true },
  { prop: "durationMs", name: "ms", size: 70, minSize: 70, readonly: true },
  { prop: "errorMessage", name: "error", size: 220, minSize: 180, readonly: true },
]);

const colIndexByProp = computed<Record<string, number>>(() => {
  const out: Record<string, number> = {};
  for (let i = 0; i < columns.value.length; i++) {
    out[String(columns.value[i].prop)] = i;
  }
  return out;
});

function makeRowFromPoint(p: CommPoint): PointRow {
  const profile = activeProfile.value;
  let addr = "";
  if (profile && profile.channelName === p.channelName) {
    if (typeof p.addressOffset === "number") {
      addr = formatHumanAddressFrom0Based(profile.readArea, profile.startAddress + p.addressOffset);
    } else if (start0ByPointKey.value[p.pointKey] !== undefined) {
      addr = formatHumanAddressFrom0Based(profile.readArea, start0ByPointKey.value[p.pointKey]);
    }
  }
  const runtime = runtimeByPointKey.value[p.pointKey];
  
  // 保留现有的选中状态
  const existingRow = gridRows.value.find(r => r.pointKey === p.pointKey);
  const isSelected = existingRow?.__selected ?? false;
  
  return {
    ...p,
    __selected: isSelected,
    modbusAddress: addr,
    quality: runtime?.quality ?? "",
    valueDisplay: runtime?.valueDisplay ?? "",
    errorMessage: runtime?.errorMessage ?? "",
    timestamp: runtime?.timestamp ?? "",
    durationMs: runtime?.durationMs ?? "",
  };
}

function rebuildGridRows() {
  const channel = activeChannelName.value;
  gridRows.value = points.value.points.filter((p) => p.channelName === channel).map(makeRowFromPoint);
}

async function rebuildPlan() {
  const profile = activeProfile.value;
  if (!profile) {
    start0ByPointKey.value = {};
    rebuildGridRows();
    return;
  }

  try {
    const filteredProfiles: ProfilesV1 = { schemaVersion: 1, profiles: [profile] };
    const filteredPoints: PointsV1 = {
      schemaVersion: 1,
      points: points.value.points.filter((p) => p.channelName === profile.channelName),
    };
    const built = await commPlanBuild(
      { profiles: filteredProfiles, points: filteredPoints },
      projectId.value,
      activeDeviceId.value
    );
    const map: Record<string, number> = {};
    for (const job of built.jobs) {
      for (const point of job.points) {
        map[point.pointKey] = job.startAddress + point.offset;
      }
    }
    start0ByPointKey.value = map;
  } catch {
    start0ByPointKey.value = {};
  } finally {
    rebuildGridRows();
  }
}

let suppressChannelWatch = false;

async function loadAll() {
  try {
    const pid = projectId.value.trim();
    const did = activeDeviceId.value.trim();
    if (!pid || !did) {
      profiles.value = { schemaVersion: 1, profiles: [] };
      points.value = { schemaVersion: 1, points: [] };
      activeChannelName.value = "";
      return;
    }

    profiles.value = await commProfilesLoad(pid, did);
    points.value = await commPointsLoad(pid, did);
    showAllValidation.value = false;
    touchedRowKeys.value = {};
    selectedRangeRows.value = null;
    markPointsChanged();

    suppressChannelWatch = true;
    const ui = project.value?.uiState;
    const stored = ui?.activeChannelName ?? "";
    if (stored && profiles.value.profiles.some((p) => p.channelName === stored)) {
      activeChannelName.value = stored;
    } else if (profiles.value.profiles.length > 0) {
      activeChannelName.value = profiles.value.profiles[0].channelName;
    } else if (points.value.points.length > 0) {
      activeChannelName.value = points.value.points[0].channelName;
    } else {
      activeChannelName.value = "";
    }

    const t = ui?.pointsBatchTemplate;
    if (t && t.schemaVersion === 1) {
      batchAddTemplate.value = {
        count: Math.max(1, Math.min(500, Math.floor(t.count || 10))),
        startAddressHuman: String(t.startAddressHuman ?? "").trim(),
        dataType: t.dataType ?? batchAddTemplate.value.dataType,
        byteOrder: t.byteOrder ?? batchAddTemplate.value.byteOrder,
        hmiNameTemplate: String(t.hmiNameTemplate ?? batchAddTemplate.value.hmiNameTemplate),
        scaleTemplate: String(t.scaleTemplate ?? batchAddTemplate.value.scaleTemplate),
        insertMode: t.insertMode === "afterSelection" ? "afterSelection" : "append",
      };
    }

    suppressChannelWatch = false;
    await rebuildPlan();
    ElMessage.success("已加载 points/profiles");
  } catch (e: unknown) {
    ElMessage.error(String((e as any)?.message ?? e ?? "load failed"));
  }
}

async function savePoints() {
  showAllValidation.value = true;
  await syncFromGridAndMapAddresses();
  const invalid = gridRows.value.map(validateRowForRun).find((v) => Boolean(v));
  if (invalid) {
    ElMessage.error(invalid);
    return;
  }
  if (!activeDeviceId.value.trim()) {
    ElMessage.error("未选择设备");
    return;
  }
  await commPointsSave(points.value, projectId.value, activeDeviceId.value);
  if (project.value) {
    const devices = project.value.devices ?? [];
    const idx = devices.findIndex((d) => d.deviceId === activeDeviceId.value);
    if (idx >= 0) {
      const nextDevices = [...devices];
      nextDevices[idx] = {
        ...nextDevices[idx],
        points: { ...points.value, points: [...points.value.points] },
      };
      project.value = { ...project.value, devices: nextDevices };
    }
  }
  ElMessage.success("已保存 points");
  showAllValidation.value = false;
  touchedRowKeys.value = {};
}

const batchAddDrawerOpen = ref(false);
const batchAddTemplate = ref<{
  count: number;
  startAddressHuman: string;
  dataType: DataType;
  byteOrder: ByteOrder32;
  hmiNameTemplate: string;
  scaleTemplate: string;
  insertMode: BatchInsertMode;
}>({
  count: 10,
  startAddressHuman: "",
  dataType: "UInt16",
  byteOrder: "ABCD",
  hmiNameTemplate: "AI_{{i}}",
  scaleTemplate: "1",
  insertMode: "append",
});

const batchAddPreview = computed(() => {
  const profile = activeProfile.value;
  if (!profile) return { ok: false as const, message: "请先选择连接" };
  return previewBatchPointsTemplate(
    {
      channelName: profile.channelName,
      count: batchAddTemplate.value.count,
      startAddressHuman: batchAddTemplate.value.startAddressHuman,
      dataType: batchAddTemplate.value.dataType,
      byteOrder: batchAddTemplate.value.byteOrder,
      mode: "increment",
      hmiNameTemplate: batchAddTemplate.value.hmiNameTemplate,
      scaleTemplate: batchAddTemplate.value.scaleTemplate,
      profileReadArea: profile.readArea,
      profileStartAddress: profile.startAddress,
      profileLength: profile.length,
    },
    10
  );
});

function newPointKey(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `pt-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function findInsertAnchor(): { row: PointRow; rowIndex: number } | null {
  if (selectedCount.value === 0) return null;
  const selectedSet = effectiveSelectedKeySet.value;
  let anchorRow: PointRow | null = null;
  let anchorIndex = -1;
  for (let i = 0; i < gridRows.value.length; i++) {
    const row = gridRows.value[i];
    if (selectedSet.has(row.pointKey)) {
      anchorRow = row;
      anchorIndex = i;
    }
  }
  if (!anchorRow || anchorIndex < 0) return null;
  return { row: anchorRow, rowIndex: anchorIndex };
}

function buildSinglePoint(profile: ConnectionProfile, baseRow?: PointRow | null): CommPoint {
  const dataType = resolveDataTypeForArea(profile.readArea, baseRow?.dataType ?? "UInt16");
  const byteOrder = baseRow?.byteOrder ?? "ABCD";
  const scale = Number.isFinite(Number(baseRow?.scale)) ? Number(baseRow!.scale) : 1;
  const suggestedStart = inferNextAddress(
    baseRow?.modbusAddress,
    dataType,
    profile.readArea,
    profile.startAddress
  );

  let addressOffset: number | undefined;
  const parsed = parseHumanAddress(suggestedStart, profile.readArea);
  if (parsed.ok && parsed.area === profile.readArea) {
    const offset = parsed.start0Based - profile.startAddress;
    if (offset >= 0) {
      addressOffset = offset;
    }
  }

  return {
    pointKey: newPointKey(),
    hmiName: "",
    dataType,
    byteOrder,
    channelName: profile.channelName,
    addressOffset,
    scale,
  };
}

function buildSinglePointWithoutProfile(channelName: string, baseRow?: PointRow | null): CommPoint {
  const dataType = baseRow?.dataType ?? "UInt16";
  const byteOrder = baseRow?.byteOrder ?? "ABCD";
  const scale = Number.isFinite(Number(baseRow?.scale)) ? Number(baseRow!.scale) : 1;
  return {
    pointKey: newPointKey(),
    hmiName: "",
    dataType,
    byteOrder,
    channelName,
    addressOffset: undefined,
    scale,
  };
}

function buildTempRowFromPoint(point: CommPoint, profile: ConnectionProfile): PointRow {
  const addr =
    typeof point.addressOffset === "number"
      ? formatHumanAddressFrom0Based(profile.readArea, profile.startAddress + point.addressOffset)
      : "";
  return {
    ...point,
    __selected: false,
    modbusAddress: addr,
    quality: "",
    valueDisplay: "",
    errorMessage: "",
    timestamp: "",
    durationMs: "",
  };
}

async function appendRows(count: number, baseRow?: PointRow | null) {
  if (count <= 0) return;
  const profile = activeProfile.value ?? null;
  const channelName = profile?.channelName ?? baseRow?.channelName ?? activeChannelName.value;
  if (!channelName) return;

  let cursor = baseRow ?? gridRows.value[gridRows.value.length - 1] ?? null;
  const newPoints: CommPoint[] = [];
  for (let i = 0; i < count; i++) {
    const point = profile
      ? buildSinglePoint(profile, cursor)
      : buildSinglePointWithoutProfile(channelName, cursor);
    newPoints.push(point);
    if (profile) {
      cursor = buildTempRowFromPoint(point, profile);
    }
  }

  let insertIndex = -1;
  for (let i = 0; i < points.value.points.length; i++) {
    if (points.value.points[i].channelName === channelName) insertIndex = i;
  }
  insertIndex = insertIndex + 1;

  const undoAction = createBatchAddUndoAction(
    () => points.value.points,
    (newPoints: CommPoint[]) => {
      points.value.points = newPoints;
    },
    newPoints.map((p) => p.pointKey),
    `新增 ${newPoints.length} 行`
  );

  points.value.points.splice(insertIndex, 0, ...newPoints);
  undoManager.push(undoAction);
  markPointsChanged();
  await rebuildPlan();
}

async function addSingleRow() {
  const profile = activeProfile.value;
  if (!profile) {
    ElMessage.error("请先选择连接");
    return;
  }

  const anchor = findInsertAnchor();
  const baseRow = anchor?.row ?? gridRows.value[gridRows.value.length - 1];
  const newPoint = buildSinglePoint(profile, baseRow);

  const insertIndex = (() => {
    if (anchor) {
      const anchorIndex = points.value.points.findIndex((p) => p.pointKey === anchor.row.pointKey);
      if (anchorIndex >= 0) return anchorIndex + 1;
    }
    let idx = -1;
    for (let i = 0; i < points.value.points.length; i++) {
      if (points.value.points[i].channelName === profile.channelName) idx = i;
    }
    return idx + 1;
  })();

  const undoAction = createBatchAddUndoAction(
    () => points.value.points,
    (newPoints: CommPoint[]) => {
      points.value.points = newPoints;
    },
    [newPoint.pointKey],
    "新增 1 行"
  );

  points.value.points.splice(insertIndex, 0, newPoint);
  undoManager.push(undoAction);
  selectedRangeRows.value = null;
  markPointsChanged();
  await rebuildPlan();

  await nextTick(async () => {
    const rowIndex = gridRows.value.findIndex((r) => r.pointKey === newPoint.pointKey);
    if (rowIndex < 0) return;
    gridRows.value.forEach((r, idx) => {
      r.__selected = idx === rowIndex;
    });
    gridRows.value = [...gridRows.value];

    const grid = gridApi();
    if (grid && typeof grid.scrollToRow === "function") {
      await grid.scrollToRow(rowIndex);
    }
  });

  ElMessage.success("已新增 1 行");
}

function openBatchAddDialog() {
  const profile = activeProfile.value;
  if (!profile) {
    ElMessage.error("请先选择连接");
    return;
  }

  const lastRow = gridRows.value[gridRows.value.length - 1];
  const preferredType = lastRow?.dataType ?? batchAddTemplate.value.dataType;
  const resolvedType = resolveDataTypeForArea(profile.readArea, preferredType);
  
  // 使用智能地址推断
  const suggestedStart = inferNextAddress(
    lastRow?.modbusAddress,
    resolvedType,
    profile.readArea,
    profile.startAddress
  );

  batchAddTemplate.value = {
    count: Math.max(1, Math.min(500, Math.floor(batchAddTemplate.value.count || 10))),
    startAddressHuman: suggestedStart,
    dataType: resolvedType,
    byteOrder: lastRow?.byteOrder ?? batchAddTemplate.value.byteOrder ?? "ABCD",
    hmiNameTemplate: batchAddTemplate.value.hmiNameTemplate?.trim() ? batchAddTemplate.value.hmiNameTemplate : "AI_{{i}}",
    scaleTemplate: String(
      Number.isFinite(Number(lastRow?.scale)) ? Number(lastRow!.scale) : batchAddTemplate.value.scaleTemplate ?? "1"
    ),
    insertMode: selectedCount.value > 0 ? "afterSelection" : "append",
  };
  batchAddDrawerOpen.value = true;
}

async function confirmBatchAdd() {
  const profile = activeProfile.value;
  if (!profile) {
    ElMessage.error("请先选择连接");
    return;
  }
  const built = buildBatchPointsTemplate({
    channelName: profile.channelName,
    count: batchAddTemplate.value.count,
    startAddressHuman: batchAddTemplate.value.startAddressHuman,
    dataType: batchAddTemplate.value.dataType,
    byteOrder: batchAddTemplate.value.byteOrder,
    mode: "increment",
    hmiNameTemplate: batchAddTemplate.value.hmiNameTemplate,
    scaleTemplate: batchAddTemplate.value.scaleTemplate,
    profileReadArea: profile.readArea,
    profileStartAddress: profile.startAddress,
    profileLength: profile.length,
  });
  if (!built.ok) {
    ElMessage.error(built.message);
    return;
  }

  const insertIndex = (() => {
    if (batchAddTemplate.value.insertMode === "afterSelection") {
      const selectedRowIndex = Math.max(
        ...gridRows.value.map((r, idx) => (r.__selected ? idx : -1))
      );
      if (selectedRowIndex >= 0) {
        const anchor = gridRows.value[selectedRowIndex];
        const anchorIndex = points.value.points.findIndex((p) => p.pointKey === anchor.pointKey);
        if (anchorIndex >= 0) return anchorIndex + 1;
      }
    }

    let idx = -1;
    for (let i = 0; i < points.value.points.length; i++) {
      if (points.value.points[i].channelName === profile.channelName) idx = i;
    }
    return idx + 1;
  })();

  // 创建撤销操作：先建 action（捕获 before），再修改数据，最后 push（捕获 after）。
  const undoAction = createBatchAddUndoAction(
    () => points.value.points,
    (newPoints: CommPoint[]) => {
      points.value.points = newPoints;
    },
    built.points.map((p) => p.pointKey),
    `添加了 ${built.points.length} 个点位`
  );

  points.value.points.splice(insertIndex, 0, ...built.points);
  undoManager.push(undoAction);
  
  markPointsChanged();
  batchAddDrawerOpen.value = false;
  await rebuildPlan();

  await nextTick(async () => {
    const grid = gridApi();
    if (grid && typeof grid.scrollToRow === "function") {
      const firstKey = built.points[0]?.pointKey;
      const rowIndex = firstKey ? gridRows.value.findIndex((r) => r.pointKey === firstKey) : -1;
      await grid.scrollToRow(rowIndex >= 0 ? rowIndex : gridRows.value.length - 1);
    }
  });

  ElMessage.success(`已新增 ${built.points.length} 行（step=${built.span}）`);

  const pid = projectId.value.trim();
  if (pid) {
    commProjectUiStatePatchV1(pid, {
      pointsBatchTemplate: {
        schemaVersion: 1,
        count: batchAddTemplate.value.count,
        startAddressHuman: batchAddTemplate.value.startAddressHuman,
        dataType: batchAddTemplate.value.dataType,
        byteOrder: batchAddTemplate.value.byteOrder,
        hmiNameTemplate: batchAddTemplate.value.hmiNameTemplate,
        scaleTemplate: batchAddTemplate.value.scaleTemplate,
        insertMode: batchAddTemplate.value.insertMode,
      },
    }).catch((e: unknown) => {
      pushLog("ui_state", "warning", `批量模板保存失败：${String((e as any)?.message ?? e ?? "")}`);
    });
  }
}

async function getSelectedRange(): Promise<{ rowStart: number; rowEnd: number; colStart: number; colEnd: number } | null> {
  const grid = gridApi();
  if (!grid) return null;
  const range = await grid.getSelectedRange();
  if (!range) return null;
  const rowStart = Math.min(range.y, range.y1);
  const rowEnd = Math.max(range.y, range.y1);
  const colStart = Math.min(range.x, range.x1);
  const colEnd = Math.max(range.x, range.x1);
  return { rowStart, rowEnd, colStart, colEnd };
}

async function removeSelectedRows() {
  let selected = effectiveSelectedRows.value;
  if (selected.length === 0) {
    const sel = await getSelectedRange();
    if (sel) {
      selectedRangeRows.value = { rowStart: sel.rowStart, rowEnd: sel.rowEnd };
      selected = effectiveSelectedRows.value;
    }
  }
  if (selected.length === 0) {
    ElMessage.warning("请先选中行（点击行号）或框选一段行区域");
    return;
  }
  const count = selected.length;

  await ElMessageBox.confirm(`确认删除选中的 ${count} 行点位？`, "删除点位", {
    confirmButtonText: "删除",
    cancelButtonText: "取消",
    type: "warning",
  });

  const selectedKeys = new Set(selected.map((r) => r.pointKey));
  
  // 创建撤销操作：先建 action（捕获 before），再修改数据，最后 push（捕获 after）。
  const undoAction = createDeleteRowsUndoAction(
    () => points.value.points,
    (newPoints: CommPoint[]) => {
      points.value.points = newPoints;
    },
    Array.from(selectedKeys),
    `删除了 ${count} 行`
  );
  
  points.value.points = points.value.points.filter((p) => !selectedKeys.has(p.pointKey));
  undoManager.push(undoAction);
  markPointsChanged();
  await rebuildPlan();
  ElMessage.success(`已删除 ${count} 行`);
}

function handleFillCommand(command: string) {
  if (command === "copy" || command === "series") {
    fillMode.value = command as "copy" | "series";
  }
}

async function applyFill() {
  if (fillMode.value === "copy") {
    await fillDownFromSelection();
    return;
  }
  await fillSeriesFromSelection();
}

async function applyFillDown(range: SelectionRange) {
  const { rowStart, rowEnd, colStart, colEnd } = range;
  if (rowEnd <= rowStart) {
    ElMessage.warning("请框选至少两行");
    return;
  }

  const grid = gridApi();
  if (!grid) return;
  const propsByColIndex = columns.value.map((c) => String(c.prop ?? ""));
  const skipProps = new Set([
    COL_ROW_SELECTED,
    "pointKey",
    "quality",
    "valueDisplay",
    "errorMessage",
    "timestamp",
    "durationMs",
  ]);
  const { edits, changed } = computeFillDownEdits({
    rows: gridRows.value,
    propsByColIndex,
    range: { rowStart, rowEnd, colStart, colEnd },
    skipProps,
  });
  for (const e of edits) {
    await grid.setDataAt({ row: e.rowIndex, col: e.colIndex, rowType: "rgRow", colType: "rgCol", val: e.value });
  }
  const touched = gridRows.value.slice(rowStart, rowEnd + 1).map((r) => r.pointKey);
  await syncFromGridAndMapAddresses(touched);
  markPointsChanged();
  ElMessage.success(`已向下填充：${rowEnd - rowStart} 行 × ${colEnd - colStart + 1} 列（${changed} 单元格）`);
}

async function fillDownFromSelection() {
  const sel = await getSelectedRange();
  if (!sel) {
    ElMessage.warning("请先框选一个单元格区域");
    return;
  }
  await applyFillDown(sel);
}

function incrementStringSuffix(input: string, step: number): string | null {
  const match = String(input).match(/^(.*?)(\d+)$/);
  if (!match) return null;
  const prefix = match[1];
  const rawNum = match[2];
  const num = Number(rawNum);
  if (!Number.isFinite(num)) return null;
  const next = num + step;
  const padded = String(next).padStart(rawNum.length, "0");
  return `${prefix}${padded}`;
}

async function applyFillSeries(range: SelectionRange) {
  const { rowStart, rowEnd, colStart, colEnd } = range;
  if (rowEnd <= rowStart && colEnd <= colStart) {
    ElMessage.warning("请框选至少两个单元格");
    return;
  }

  const grid = gridApi();
  if (!grid) return;

  const propsByColIndex = columns.value.map((c) => String(c.prop ?? ""));
  const skipProps = new Set([
    COL_ROW_SELECTED,
    "pointKey",
    "quality",
    "valueDisplay",
    "errorMessage",
    "timestamp",
    "durationMs",
  ]);

  const baseRow = gridRows.value[rowStart];
  if (!baseRow) return;

  const addrCol = colIndexByProp.value["modbusAddress"];
  const includeAddress = addrCol >= colStart && addrCol <= colEnd;
  const profile = activeProfile.value;

  if (includeAddress && !profile) {
    ElMessage.error("请先选择连接");
    return;
  }

  if (includeAddress) {
    const active = profile;
    if (!active) {
      ElMessage.error("请先选择连接");
      return;
    }

    const computed = computeFillAddressEdits({
      rows: gridRows.value,
      range: { rowStart, rowEnd },
      readArea: active.readArea,
    });
    if (!computed.ok) {
      ElMessage.error(computed.message);
      return;
    }

    const channelEnd0 = active.startAddress + active.length;
    for (const e of computed.edits) {
      const parsed = parseHumanAddress(e.value, active.readArea);
      if (!parsed.ok) {
        ElMessage.error(parsed.message);
        return;
      }
      const row = gridRows.value[e.rowIndex];
      const len = row ? spanForArea(active.readArea, row.dataType) : null;
      if (len === null) {
        ElMessage.error(`dataType=${row?.dataType ?? "?"} 与 readArea=${active.readArea} 不匹配（row=${e.rowIndex + 1}）`);
        return;
      }
      if (parsed.start0Based < active.startAddress) {
        ElMessage.error(`地址小于连接起始地址 ${formatHumanAddressFrom0Based(active.readArea, active.startAddress)}`);
        return;
      }
      if (parsed.start0Based + len > channelEnd0) {
        const endHuman = formatHumanAddressFrom0Based(active.readArea, parsed.start0Based + len - 1);
        const limitHuman = formatHumanAddressFrom0Based(active.readArea, channelEnd0 - 1);
        ElMessage.error(`地址越界：结束地址 ${endHuman} 超出连接范围 ${limitHuman}`);
        return;
      }
    }

    for (const e of computed.edits) {
      await grid.setDataAt({ row: e.rowIndex, col: addrCol, rowType: "rgRow", colType: "rgCol", val: e.value });
    }
  }

  for (let r = rowStart; r <= rowEnd; r++) {
    const step = r - rowStart;
    for (let c = colStart; c <= colEnd; c++) {
      if (includeAddress && c === addrCol) continue;
      const prop = String(propsByColIndex[c] ?? "");
      if (!prop || skipProps.has(prop)) continue;
      const baseVal = (baseRow as any)[prop];
      let nextVal = baseVal;
      if (typeof baseVal === "number" && Number.isFinite(baseVal)) {
        nextVal = baseVal + step;
      } else if (typeof baseVal === "string") {
        nextVal = incrementStringSuffix(baseVal, step) ?? baseVal;
      }
      await grid.setDataAt({ row: r, col: c, rowType: "rgRow", colType: "rgCol", val: nextVal });
    }
  }

  const touched = gridRows.value.slice(rowStart, rowEnd + 1).map((r) => r.pointKey);
  await syncFromGridAndMapAddresses(touched);
  markPointsChanged();
  ElMessage.success(`已序列填充：${rowEnd - rowStart + 1} 行`);
}

async function fillSeriesFromSelection() {
  const sel = await getSelectedRange();
  if (!sel) {
    ElMessage.warning("请先框选一个单元格区域");
    return;
  }
  await applyFillSeries(sel);
}

// 撤销管理器
const undoManager = new UndoManager(20);

// 批量编辑对话框
const batchEditDialogVisible = ref(false);

async function openBatchEditDialog() {
  const sel = await getSelectedRange();
  if (sel) {
    selectedRangeRows.value = { rowStart: sel.rowStart, rowEnd: sel.rowEnd };
  }
  if (selectedCount.value === 0) {
    ElMessage.warning("请先选中行（点击行号）或框选一段行区域");
    return;
  }
  batchEditDialogVisible.value = true;
}

async function handleBatchEditConfirm(request: BatchEditRequest) {
  const result = computeBatchEdits(points.value.points, request);

  if (result.totalChanges === 0) {
    ElMessage.info("没有需要修改的字段");
    return;
  }

  // 创建撤销操作：先建 action（捕获 before），再修改数据，最后 push（捕获 after）。
  const undoAction = createBatchEditUndoAction(
    () => points.value.points,
    (newPoints: CommPoint[]) => {
      points.value.points = newPoints;
    },
    `批量编辑：${result.affectedPoints} 行 / ${result.totalChanges} 个字段`
  );

  applyBatchEdits(points.value.points, result);
  undoManager.push(undoAction);

  markPointsChanged();
  await rebuildPlan();

  ElMessage.success(`已批量编辑 ${result.affectedPoints} 行 / ${result.totalChanges} 个字段`);
}

function handleUndo() {
  if (!undoManager.canUndo()) {
    ElMessage.warning('没有可撤销的操作');
    return;
  }
  undoManager.undo();
  void rebuildPlan();
  ElMessage.success('已撤销');
}

function handleRedo() {
  if (!undoManager.canRedo()) {
    ElMessage.warning('没有可重做的操作');
    return;
  }
  undoManager.redo();
  void rebuildPlan();
  ElMessage.success('已重做');
}

async function syncFromGridAndMapAddresses(touchedKeys?: string[]) {
  const grid = gridApi();
  if (!grid) return;
  const source = (await grid.getSource()) as PointRow[];
  gridRows.value = source;

  const profile = activeProfile.value;
  if (!profile) return;

  for (const row of gridRows.value) {
    const point = points.value.points.find((p) => p.pointKey === row.pointKey);
    if (!point) continue;

    point.hmiName = row.hmiName;
    point.dataType = row.dataType;
    point.byteOrder = row.byteOrder;
    point.scale = Number(row.scale);

    const addrRaw = row.modbusAddress.trim();
    if (!addrRaw) {
      point.addressOffset = undefined;
      continue;
    }

    const parsed = parseHumanAddress(addrRaw, profile.readArea);
    if (!parsed.ok) continue;

    const offset = parsed.start0Based - profile.startAddress;
    if (offset < 0) continue;
    point.addressOffset = offset;
  }

  if (touchedKeys && touchedKeys.length > 0) {
    const next = { ...touchedRowKeys.value };
    for (const key of touchedKeys) next[String(key)] = true;
    touchedRowKeys.value = next;
  }
}

async function startPolling() {
  clearTimer();
  timer = window.setInterval(pollLatest, pollMs.value);
  await pollLatest();
  pushLog("poll", "info", `polling every ${pollMs.value}ms`);
}

function applyLatestToGridRows(results: SampleResult[]) {
  const byKey: Record<string, SampleResult> = {};
  for (const r of results) byKey[r.pointKey] = r;
  runtimeByPointKey.value = byKey;

  const grid = gridApi();
  if (!grid) return;
  const idx = colIndexByProp.value;

  for (let rowIndex = 0; rowIndex < gridRows.value.length; rowIndex++) {
    const row = gridRows.value[rowIndex];
    const res = byKey[row.pointKey];
    if (!res) continue;

    const nextQuality = res.quality;
    const nextValue = res.valueDisplay;
    const nextErr = res.errorMessage ?? "";
    const nextTs = res.timestamp;
    const nextMs = res.durationMs;

    const changed =
      row.quality !== nextQuality ||
      row.valueDisplay !== nextValue ||
      row.errorMessage !== nextErr ||
      row.timestamp !== nextTs ||
      row.durationMs !== nextMs;
    if (!changed) continue;

    row.quality = nextQuality;
    row.valueDisplay = nextValue;
    row.errorMessage = nextErr;
    row.timestamp = nextTs;
    row.durationMs = nextMs;

    void grid.setDataAt({ row: rowIndex, col: idx["quality"], rowType: "rgRow", colType: "rgCol", val: nextQuality });
    void grid.setDataAt({ row: rowIndex, col: idx["valueDisplay"], rowType: "rgRow", colType: "rgCol", val: nextValue });
    void grid.setDataAt({ row: rowIndex, col: idx["errorMessage"], rowType: "rgRow", colType: "rgCol", val: nextErr });
    void grid.setDataAt({ row: rowIndex, col: idx["timestamp"], rowType: "rgRow", colType: "rgCol", val: nextTs });
    void grid.setDataAt({ row: rowIndex, col: idx["durationMs"], rowType: "rgRow", colType: "rgCol", val: nextMs });
  }
}

async function startRun() {
  if (runUiState.value === "starting" || runUiState.value === "running") return;
  clearAutoRestartTimer();
  runUiState.value = "starting";
  runError.value = null;
  latest.value = null;
  runPointsRevision.value = null;
  pushLog("run_start", "info", "start clicked");

  showAllValidation.value = true;
  await syncFromGridAndMapAddresses();
  const invalid = gridRows.value.map(validateRowForRun).find((v) => Boolean(v));
  if (invalid) {
    runError.value = makeUiConfigError(invalid);
    pushLog("run_start", "error", `${runError.value.kind}: ${runError.value.message}`);
    ElMessage.error(invalid);
    runUiState.value = "error";
    return;
  }

  try {
    const profile = activeProfile.value;
    if (!profile) {
      const err = makeUiConfigError("未选择连接");
      runError.value = err;
      pushLog("run_start", "error", `${err.kind}: ${err.message}`);
      ElMessage.error(err.message);
      runUiState.value = "error";
      return;
    }

    const channelPoints = points.value.points.filter((p) => p.channelName === profile.channelName);
    if (channelPoints.length === 0) {
      const err = makeUiConfigError("points 为空：请先新增点位并保存");
      runError.value = err;
      pushLog("run_start", "error", `${err.kind}: ${err.message}`);
      ElMessage.error(err.message);
      runUiState.value = "error";
      return;
    }

    const planToUse = await commPlanBuild(
      { profiles: { schemaVersion: 1, profiles: [profile] }, points: { schemaVersion: 1, points: channelPoints } },
      projectId.value,
      activeDeviceId.value
    );
    pushLog("run_start", "info", `plan ok: jobs=${planToUse.jobs.length}`);

    pushLog("run_start", "info", "调用 comm_run_start_obs（后端 spawn，不阻塞 UI）");
    const resp = await commRunStartObs(
      {
        profiles: { schemaVersion: 1, profiles: [profile] },
        points: { schemaVersion: 1, points: channelPoints },
        plan: planToUse,
      },
      projectId.value,
      activeDeviceId.value
    );

    if (!resp.ok || !resp.runId) {
      const err =
        resp.error ??
        ({
          kind: "InternalError",
          message: "comm_run_start_obs failed (ok=false) but error is missing",
        } as CommRunError);
      runError.value = err;
      pushLog("run_start", "error", `${err.kind}: ${err.message}`);
      ElMessage.error(`${err.kind}: ${err.message}`);
      runUiState.value = "error";
      return;
    }

    runId.value = resp.runId;
    runUiState.value = "running";
    runPointsRevision.value = pointsRevision.value;
    pushLog("run_start", "success", `run started: runId=${resp.runId}`);
    ElMessage.success(`采集已启动：runId=${resp.runId}`);

    await startPolling();
  } catch (e: unknown) {
    const err = makeUiConfigError(String((e as any)?.message ?? e ?? "unknown error"));
    runError.value = err;
    pushLog("run_start", "error", `${err.kind}: ${err.message}`);
    ElMessage.error(`${err.kind}: ${err.message}`);
    runUiState.value = "error";
  }
}

async function pollLatest() {
  const id = runId.value;
  if (!id) return;
  const resp = await commRunLatestObs(id);
  if (!resp.ok || !resp.value) {
    const err =
      resp.error ??
      ({
        kind: "InternalError",
        message: "comm_run_latest_obs failed (ok=false) but error is missing",
      } as CommRunError);
    runError.value = err;
    pushLog("run_latest", "error", `${err.kind}: ${err.message}`);
    runUiState.value = "error";
    clearTimer();
    if (runId.value) {
      void commRunStopObs(id, projectId.value).catch(() => {
        // ignore stop errors after latest failure
      });
    }
    return;
  }

  latest.value = resp.value;
  applyLatestToGridRows(resp.value.results);
  pushLog("run_latest", "success", `ok: total=${resp.value.stats.total} ok=${resp.value.stats.ok}`);
}

async function stopRun(reason: "manual" | "restart" | "validation" = "manual") {
  if (!runId.value || runUiState.value !== "running") return;
  clearAutoRestartTimer();
  if (reason !== "validation") {
    resumeAfterFix.value = false;
  }
  runUiState.value = "stopping";
  const id = runId.value;
  const reasonLabel =
    reason === "validation" ? "auto stop: invalid config" : reason === "restart" ? "stop for restart" : "stop clicked";
  pushLog("run_stop", "info", reasonLabel);

  try {
    const resp = await commRunStopObs(id, projectId.value);
    if (!resp.ok) {
      const err =
        resp.error ??
        ({
          kind: "InternalError",
          message: "comm_run_stop_obs failed (ok=false) but error is missing",
        } as CommRunError);
      runError.value = err;
      pushLog("run_stop", "error", `${err.kind}: ${err.message}`);
      ElMessage.error(`${err.kind}: ${err.message}`);
      runUiState.value = "error";
      return;
    }
    pushLog("run_stop", "success", "stopped");
    ElMessage.success("采集已停止");
    runUiState.value = "idle";
    clearTimer();
  } catch (e: unknown) {
    const err = makeUiConfigError(String((e as any)?.message ?? e ?? "unknown error"));
    runError.value = err;
    pushLog("run_stop", "error", `${err.kind}: ${err.message}`);
    ElMessage.error(`${err.kind}: ${err.message}`);
    runUiState.value = "error";
  }
}

async function restartRun() {
  if (!isRunning.value || !runId.value) return;
  clearAutoRestartTimer();
  pushLog("run_restart", "info", "restart clicked");
  await stopRun("restart");
  await startRun();
}

function collectTouchedPointKeysFromAfterEdit(e: any): string[] {
  const keys = new Set<string>();
  const detail = e?.detail ?? e;

  if (detail?.model?.pointKey) keys.add(String(detail.model.pointKey));
  if (detail?.models && typeof detail.models === "object") {
    for (const v of Object.values(detail.models)) {
      if (v && typeof v === "object" && "pointKey" in v) keys.add(String((v as any).pointKey));
    }
  }
  return [...keys];
}

function onAfterEdit(e: any) {
  const touched = collectTouchedPointKeysFromAfterEdit(e);
  focusedIssueCell.value = null;
  void syncFromGridAndMapAddresses(touched);
  markPointsChanged();
}

function onBeforeGridKeyDown(e: any) {
  const original = e?.detail?.original as KeyboardEvent | undefined;
  if (!original) return;
  const gridHasFocus = Boolean(e?.detail?.focus || e?.detail?.range || e?.detail?.edit);
  if (!gridHasFocus) return;

  const key = original.key?.toLowerCase();
  const isCtrl = original.ctrlKey || original.metaKey;
  if (!isCtrl || !key) return;

  if (key === "z" && !original.shiftKey) {
    handleUndo();
    e.preventDefault?.();
    original.preventDefault();
    original.stopPropagation();
    return;
  }

  if (key === "y" || (key === "z" && original.shiftKey)) {
    handleRedo();
    e.preventDefault?.();
    original.preventDefault();
    original.stopPropagation();
  }
}

async function onBeforeAutofill(e: any) {
  const detail = e?.detail;
  const range = detail?.newRange ?? detail?.range ?? detail?.oldRange;
  if (!range) return;

  if (typeof e?.preventDefault === "function") {
    e.preventDefault();
  }

  const rowStart = Math.min(range.y, range.y1);
  const rowEnd = Math.max(range.y, range.y1);
  const colStart = Math.min(range.x, range.x1);
  const colEnd = Math.max(range.x, range.x1);

  const missing = rowEnd - (gridRows.value.length - 1);
  if (missing > 0) {
    const baseRow = gridRows.value[rowStart] ?? gridRows.value[gridRows.value.length - 1] ?? null;
    await appendRows(missing, baseRow);
    await nextTick();
  }

  const selRange: SelectionRange = { rowStart, rowEnd, colStart, colEnd };
  if (fillMode.value === "copy") {
    await applyFillDown(selRange);
  } else {
    await applyFillSeries(selRange);
  }
}

function getRowClass(row: any): string {
  const pointRow = row?.model as PointRow | undefined;
  return pointRow && effectiveSelectedKeySet.value.has(pointRow.pointKey) ? 'row-selected' : '';
}

async function jumpToIssue(issue: ValidationIssue) {
  const rowIndex = gridRows.value.findIndex((r) => r.pointKey === issue.pointKey);
  if (rowIndex < 0) return;

  selectedRangeRows.value = null;
  gridRows.value.forEach((r) => {
    r.__selected = r.pointKey === issue.pointKey;
  });
  gridRows.value = [...gridRows.value];

  await nextTick();
  const grid = gridApi();
  if (!grid) return;
  if (typeof grid.scrollToRow === "function") {
    await grid.scrollToRow(rowIndex);
  }

  if (issue.field) {
    const colIndex = colIndexByProp.value[String(issue.field)];
    if (typeof grid.scrollToColumnIndex === "function" && typeof colIndex === "number") {
      await grid.scrollToColumnIndex(colIndex);
    } else if (typeof grid.scrollToColumnProp === "function") {
      await grid.scrollToColumnProp(issue.field);
    } else if (typeof grid.scrollToCoordinate === "function" && typeof colIndex === "number") {
      await grid.scrollToCoordinate({ x: colIndex, y: rowIndex });
    }
  }
}

async function handleJumpToIssue(issue: ValidationIssue) {
  validationPanelOpen.value = false;
  if (issue.field) {
    focusedIssueCell.value = { pointKey: issue.pointKey, field: issue.field };
  } else {
    focusedIssueCell.value = null;
  }
  await nextTick();
  await jumpToIssue(issue);
}

onMounted(() => {
  nextTick(() => {
    attachGridSelectionListeners();
  });
});

watch(activeChannelName, async (v) => {
  if (suppressChannelWatch) return;
  selectedRangeRows.value = null;
  const pid = projectId.value.trim();
  if (pid) {
    commProjectUiStatePatchV1(pid, { activeChannelName: v }).catch((e: unknown) => {
      pushLog("ui_state", "warning", `activeChannelName 保存失败：${String((e as any)?.message ?? e ?? "")}`);
    });
  }
  await rebuildPlan();
});

watch(pollMs, (v) => {
  if (!isRunning.value) return;
  clearTimer();
  timer = window.setInterval(pollLatest, v);
  pushLog("poll", "info", `polling interval changed: ${v}ms`);
});

watch([projectId, activeDeviceId], loadAll, { immediate: true });

// 注册键盘快捷键
useKeyboardShortcuts(createStandardShortcuts({
  onBatchAdd: openBatchAddDialog,
  onBatchEdit: openBatchEditDialog,
  onDelete: removeSelectedRows,
  onUndo: handleUndo,
  onRedo: handleRedo,
  onSave: savePoints,
}));

onBeforeUnmount(() => {
  clearTimer();
  clearAutoRestartTimer();
  detachGridSelectionListeners?.();
});
</script>

<template>
  <div class="comm-subpage comm-subpage--points">
      <header class="comm-hero comm-animate" style="--delay: 0ms">
        <div class="comm-hero-title">
          <div class="comm-title">点位配置</div>
          <div class="comm-subtitle">
            实时采集 <span v-if="activeDevice">· {{ activeDevice.deviceName }}</span>
          </div>
        </div>
        <div class="comm-hero-actions">
          <el-button @click="loadAll">加载</el-button>
          <el-button type="primary" @click="savePoints">保存</el-button>
        </div>
      </header>

      <el-alert
        class="comm-hint-bar comm-animate"
        type="info"
        show-icon
        :closable="false"
        title="提示：表格支持直接编辑、TSV 粘贴；框选区域后使用“填充”进行同值或序列递增。"
        style="--delay: 60ms"
      />

      <section class="comm-panel comm-panel--run comm-animate" style="--delay: 120ms">
        <div class="comm-run-grid">
          <div class="comm-profile-block">
            <div class="comm-label">连接配置</div>
            <el-select
              v-model="activeChannelName"
              placeholder="选择连接"
              :disabled="isRunning || isStarting || isStopping"
            >
              <el-option v-for="p in profiles.profiles" :key="p.channelName" :label="profileLabel(p)" :value="p.channelName" />
            </el-select>
            <div v-if="activeProfile" class="comm-profile-meta">
              <span class="comm-chip">{{ activeProfile.protocolType }}</span>
              <span class="comm-chip">{{ activeProfile.channelName }}</span>
              <span class="comm-chip">Area {{ activeProfile.readArea }}</span>
              <span class="comm-chip">Start {{ formatHumanAddressFrom0Based(activeProfile.readArea, activeProfile.startAddress) }}</span>
              <span class="comm-chip">Len {{ activeProfile.length }}</span>
            </div>
          </div>

          <div class="comm-run-actions">
            <el-button type="primary" :loading="isStarting" :disabled="isRunning || isStopping" @click="startRun">开始运行</el-button>
            <el-button type="danger" :loading="isStopping" :disabled="!isRunning" @click="stopRun('manual')">停止</el-button>
            <el-select v-model="pollMs" style="width: 160px">
              <el-option :value="500" label="轮询 500ms" />
              <el-option :value="1000" label="轮询 1s" />
              <el-option :value="2000" label="轮询 2s" />
            </el-select>
          </div>
        </div>

        <div class="comm-run-meta">
          <div class="comm-run-tags">
            <el-tag v-if="isRunning && configChangedDuringRun" type="warning" effect="light">
              {{ autoRestartPending ? "配置已变更：即将自动重启" : "配置已变更：重启中" }}
            </el-tag>
            <el-tag v-if="runUiState === 'running'" type="success">running</el-tag>
            <el-tag v-else-if="runUiState === 'starting'" type="warning">starting</el-tag>
            <el-tag v-else-if="runUiState === 'stopping'" type="warning">stopping</el-tag>
            <el-tag v-else-if="runUiState === 'error'" type="danger">error</el-tag>
            <el-tag v-else type="info">idle</el-tag>
          </div>
          <div class="comm-run-tags">
            <el-tag v-if="runId" type="info">runId={{ runId }}</el-tag>
            <el-tag v-if="resumeAfterFix && !isRunning" type="warning">配置无效：修复后自动恢复</el-tag>
            <el-tag v-if="latest" type="info">updatedAt={{ latest.updatedAtUtc }}</el-tag>
          </div>
        </div>
      </section>

      <section class="comm-panel comm-panel--stats comm-animate" style="--delay: 180ms">
        <div class="comm-stat-grid">
          <div class="comm-stat"><el-statistic title="Total" :value="latest?.stats.total ?? 0" /></div>
          <div class="comm-stat"><el-statistic title="OK" :value="latest?.stats.ok ?? 0" /></div>
          <div class="comm-stat"><el-statistic title="Timeout" :value="latest?.stats.timeout ?? 0" /></div>
          <div class="comm-stat"><el-statistic title="CommErr" :value="latest?.stats.commError ?? 0" /></div>
          <div class="comm-stat"><el-statistic title="DecodeErr" :value="latest?.stats.decodeError ?? 0" /></div>
          <div class="comm-stat"><el-statistic title="CfgErr" :value="latest?.stats.configError ?? 0" /></div>
        </div>
      </section>

      <el-alert
        v-if="runError"
        class="comm-run-error comm-animate"
        style="--delay: 200ms"
        type="error"
        show-icon
        :closable="false"
        :title="`${runError.kind}: ${runError.message}`"
      />

      <div class="comm-status-bar comm-animate" style="--delay: 220ms" :class="{ 'is-error': hasValidationIssues || hasBackendFieldIssues }">
        <div class="comm-status-left">
          <div class="comm-status-title">配置校验</div>
          <div class="comm-status-desc">{{ validationSummary }}</div>
        </div>
        <div class="comm-status-actions">
          <el-button size="small" :disabled="!hasValidationIssues && !hasBackendFieldIssues" @click="validationPanelOpen = true">
            查看详情
          </el-button>
        </div>
      </div>

      <section class="comm-panel comm-panel--table comm-animate" style="--delay: 260ms">
        <div class="comm-toolbar">
          <div class="comm-toolbar-left">
            <el-button type="primary" @click="addSingleRow">新增单行</el-button>
            <el-button @click="openBatchAddDialog">批量新增 (Ctrl+B)</el-button>
            <el-button :disabled="gridRows.length === 0" @click="openBatchEditDialog">批量编辑 (Ctrl+E)</el-button>
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
            style="height: clamp(420px, 62vh, 740px); width: 100%"
            @beforeautofill="onBeforeAutofill"
            @afteredit="onAfterEdit"
            @beforekeydown="onBeforeGridKeyDown"
            @setrange="onGridSetRange"
            @selectionchangeinit="onGridSelectionChange"
            @clearregion="onGridClearRegion"
          />
        </div>
      </section>

    <el-drawer v-model="validationPanelOpen" title="配置校验" size="520px">
      <div class="comm-validation-drawer">
        <el-empty v-if="!hasValidationIssues && !hasBackendFieldIssues" description="暂无校验错误" />

        <template v-else>
          <el-alert
            v-if="hasValidationIssues"
            type="error"
            show-icon
            :closable="false"
            :title="`前端校验阻断错误 ${validationIssues.length} 条`"
            style="margin-bottom: 12px"
          />
          <el-table v-if="hasValidationIssues" :data="validationIssues" size="small" height="240">
            <el-table-column prop="hmiName" label="HMI" min-width="140" />
            <el-table-column prop="modbusAddress" label="Addr" width="110" />
            <el-table-column prop="message" label="问题" min-width="200" />
            <el-table-column label="定位" width="96" align="center" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleJumpToIssue(row)">定位</el-button>
              </template>
            </el-table-column>
          </el-table>

          <el-divider v-if="hasBackendFieldIssues" style="margin: 16px 0" />

          <el-alert
            v-if="hasBackendFieldIssues"
            type="warning"
            show-icon
            :closable="false"
            :title="`后端校验字段问题 ${backendFieldIssues.length} 条`"
            style="margin-bottom: 12px"
          />
          <el-table v-if="hasBackendFieldIssues" :data="backendFieldIssues" size="small" height="220">
            <el-table-column prop="hmiName" label="HMI" min-width="120" />
            <el-table-column prop="pointKey" label="pointKey" min-width="160" />
            <el-table-column prop="field" label="field" min-width="120" />
            <el-table-column prop="reason" label="reason" min-width="160" />
          </el-table>
        </template>
      </div>
    </el-drawer>

    <el-dialog v-model="batchAddDrawerOpen" title="批量新增（模板）" width="980px">
      <el-row :gutter="12">
        <el-col :span="12">
          <el-form label-width="140px">
            <el-form-item label="行数（N）">
              <el-input-number v-model="batchAddTemplate.count" :min="1" :max="500" />
            </el-form-item>
            <el-form-item label="起始地址（1-based）">
              <el-input v-model="batchAddTemplate.startAddressHuman" placeholder="例如 1" />
            </el-form-item>
            <el-form-item label="数据类型（步长）">
              <el-select v-model="batchAddTemplate.dataType" style="width: 220px">
                <el-option v-for="opt in dataTypeOptions" :key="opt" :label="opt" :value="opt" />
              </el-select>
              <el-tag v-if="activeProfile" type="info" style="margin-left: 10px"
                >step={{ spanForArea(activeProfile.readArea, batchAddTemplate.dataType) ?? "?" }}</el-tag
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
            <el-form-item label="scale 模板">
              <el-input v-model="batchAddTemplate.scaleTemplate" placeholder="例如 1 或 {{i}}" />
              <div style="margin-top: 6px; color: var(--el-text-color-secondary); font-size: 12px">
                MVP：仅支持数字或 <code v-pre>{{i}}</code>
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
              <el-table-column prop="hmiName" label="HMI" min-width="160" />
              <el-table-column prop="modbusAddress" label="Addr" width="110" />
              <el-table-column prop="dataType" label="dtype" width="100" />
              <el-table-column prop="byteOrder" label="endian" width="100" />
              <el-table-column prop="scale" label="scale" width="100" />
            </el-table>
          </el-card>
        </el-col>
      </el-row>

      <template #footer>
        <el-button @click="batchAddDrawerOpen = false">取消</el-button>
        <el-button type="primary" @click="confirmBatchAdd">生成并插入</el-button>
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
.comm-validation-drawer {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

:deep(.comm-grid) {
  width: 100%;
}

:deep(.comm-grid .rgHeaderCell) {
  background: #f1f5f9;
  color: var(--comm-text);
  font-weight: 600;
  border-bottom: 1px solid var(--comm-border);
}

:deep(.comm-grid .rgCell) {
  font-size: 12px;
  padding: 0 8px;
  border-color: rgba(15, 23, 42, 0.06);
  font-variant-numeric: tabular-nums;
}

:deep(.comm-grid .rgRow:nth-child(even) .rgCell) {
  background: rgba(15, 23, 42, 0.02);
}

:deep(.comm-grid .rgRow:hover .rgCell) {
  background: rgba(37, 99, 235, 0.06);
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
  border: 1px solid rgba(148, 163, 184, 0.5);
  border-radius: 8px;
  font-size: 12px;
  outline: none;
  background: #ffffff;
}

:deep(.comm-rg-editor:focus) {
  border-color: var(--comm-primary);
  box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.15);
}

:deep(.rgHeaderCell[data-type="rowHeaders"]) {
  cursor: pointer;
  user-select: none;
  transition: background-color 0.2s;
}

:deep(.rgHeaderCell[data-type="rowHeaders"]:hover) {
  background-color: rgba(37, 99, 235, 0.12);
}

:deep(.rowHeaders .rgCell) {
  cursor: pointer;
  user-select: none;
  transition: background-color 0.2s;
}

:deep(.rowHeaders .rgCell:hover) {
  background-color: rgba(37, 99, 235, 0.1);
}

:deep(.row-selected) {
  background-color: rgba(37, 99, 235, 0.08) !important;
}

:deep(.row-selected .rgHeaderCell[data-type="rowHeaders"]) {
  background-color: rgba(37, 99, 235, 0.22) !important;
  font-weight: 600;
  color: var(--comm-primary-ink);
}

:deep(.row-selected .rgCell) {
  background-color: rgba(37, 99, 235, 0.06) !important;
}
</style>
