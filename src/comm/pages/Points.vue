<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";
import Grid, { VGridVueEditor, type ColumnRegular, type Editors } from "@revolist/vue3-datagrid";

import TextEditor from "../components/revogrid/TextEditor.vue";
import SelectEditor from "../components/revogrid/SelectEditor.vue";
import NumberEditor from "../components/revogrid/NumberEditor.vue";

import { formatHumanAddressFrom0Based, nextAddress, parseHumanAddress, spanForArea } from "../services/address";
import { buildBatchPointsTemplate, previewBatchPointsTemplate } from "../services/batchAdd";
import { computeFillAddressEdits, computeFillDownEdits } from "../services/fill";
import { compileScaleExpr } from "../services/scaleExpr";

import type {
  ByteOrder32,
  CommPoint,
  CommRunError,
  CommRunLatestResponse,
  ConnectionProfile,
  DataType,
  PointsV1,
  ProfilesV1,
  Quality,
  ReadPlan,
  SampleResult,
} from "../api";
import {
  commPlanBuild,
  commPointsSave,
  commProjectLoadV1,
  commProjectUiStatePatchV1,
  commRunLatestObs,
  commRunStartObs,
  commRunStopObs,
} from "../api";

const route = useRoute();
const projectId = computed(() => String(route.params.projectId ?? ""));

const DATA_TYPES: DataType[] = ["Bool", "Int16", "UInt16", "Int32", "UInt32", "Float32"];
const BYTE_ORDERS: ByteOrder32[] = ["ABCD", "BADC", "CDAB", "DCBA"];

type RunUiState = "idle" | "starting" | "running" | "stopping" | "error";
type LogLevel = "info" | "success" | "warning" | "error";

interface LogEntry {
  ts: string;
  step: string;
  level: LogLevel;
  message: string;
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

const activeChannelName = ref<string>("");
const gridRows = ref<PointRow[]>([]);

const selectedCount = computed(() => gridRows.value.reduce((acc, r) => acc + (r.__selected ? 1 : 0), 0));

const showAllValidation = ref(false);
const touchedRowKeys = ref<Record<string, boolean>>({});
const pointsRevision = ref(0);

const plan = ref<ReadPlan | null>(null);
const start0ByPointKey = ref<Record<string, number>>({});

const runUiState = ref<RunUiState>("idle");
const runId = ref<string | null>(null);
const latest = ref<CommRunLatestResponse | null>(null);
const runError = ref<CommRunError | null>(null);
const logs = ref<LogEntry[]>([]);
const pollMs = ref<number>(1000);
const runtimeByPointKey = ref<Record<string, SampleResult>>({});
const advancedOpen = ref<string[]>([]);

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

function markPointsChanged() {
  pointsRevision.value += 1;
}

function makeUiConfigError(message: string): CommRunError {
  return {
    kind: "ConfigError",
    message,
    details: { projectId: projectId.value },
  };
}

function gridEl(): any | null {
  const v = gridRef.value as any;
  return v?.$el ?? v ?? null;
}

function profileLabel(p: ConnectionProfile): string {
  if (p.protocolType === "TCP") {
    return `${p.channelName} / TCP / ${p.ip}:${p.port} / area=${p.readArea} / start=${p.startAddress + 1} len=${p.length}`;
  }
  return `${p.channelName} / 485 / ${p.serialPort} / area=${p.readArea} / start=${p.startAddress + 1} len=${p.length}`;
}

function validateHmiName(row: PointRow): string | null {
  return String(row.hmiName ?? "").trim() ? null : "变量名称（HMI）不能为空";
}

function validateScale(row: PointRow): string | null {
  return Number.isFinite(Number(row.scale)) ? null : "缩放倍数必须为有效数字";
}

function validateModbusAddress(row: PointRow): string | null {
  const profile = activeProfile.value;
  if (!profile) return "请先选择连接";

  const addrRaw = row.modbusAddress.trim();
  if (!addrRaw) return null; // 兼容旧行为：空地址 => addressOffset=None，plan 会按顺排自动映射

  const parsed = parseHumanAddress(addrRaw);
  if (!parsed.ok) return parsed.message;
  if (parsed.area !== profile.readArea) {
    return `地址区域不匹配：profile.readArea=${profile.readArea}，输入=${parsed.area}`;
  }

  const len = spanForArea(profile.readArea, row.dataType);
  if (len === null) return `dataType=${row.dataType} 与 readArea=${profile.readArea} 不匹配`;

  const start0 = parsed.start0Based;
  if (start0 < profile.startAddress) return "地址小于连接起始地址";

  const end0 = start0 + len;
  const channelEnd0 = profile.startAddress + profile.length;
  if (end0 > channelEnd0) return `地址越界：end=${end0} > channelEnd=${channelEnd0}`;
  return null;
}

function validateRowForRun(row: PointRow): string | null {
  if (!activeProfile.value) return "请先选择连接";
  return validateHmiName(row) ?? validateScale(row) ?? validateModbusAddress(row);
}

function rowCellProps(field: keyof PointRow) {
  return ({ model }: any) => {
    const row = model as PointRow;
    const shouldValidate = showAllValidation.value || Boolean(touchedRowKeys.value[String(row.pointKey)]);
    if (!shouldValidate) return {};

    let err: string | null = null;
    if (field === "hmiName") err = validateHmiName(row);
    if (field === "scale") err = validateScale(row);
    if (field === "modbusAddress") err = validateModbusAddress(row);
    return err ? { class: { "comm-cell-error": true }, title: err } : {};
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
    prop: COL_ROW_SELECTED,
    name: "",
    size: 44,
    readonly: true,
    cellTemplate: (h: any, props: any) => {
      const row = props?.model as PointRow | undefined;
      if (!row) return "";
      return h("input", {
        type: "checkbox",
        class: "comm-row-checkbox",
        checked: Boolean(row.__selected),
        style: { display: "block", margin: "0 auto" },
        onClick: (e: MouseEvent) => {
          e.stopPropagation();
        },
        onChange: (e: Event) => {
          const el = e.target as HTMLInputElement | null;
          row.__selected = Boolean(el?.checked);
        },
      });
    },
  },
  { prop: "hmiName", name: "变量名称（HMI）*", size: 220, editor: EDITOR_TEXT, cellProperties: rowCellProps("hmiName") },
  { prop: "modbusAddress", name: "Modbus 地址", size: 140, editor: EDITOR_TEXT, cellProperties: rowCellProps("modbusAddress") },
  {
    prop: "dataType",
    name: "数据类型",
    size: 120,
    editor: EDITOR_SELECT,
    editorOptions: DATA_TYPES.map((v) => ({ label: v, value: v })),
  },
  {
    prop: "byteOrder",
    name: "字节序",
    size: 100,
    editor: EDITOR_SELECT,
    editorOptions: BYTE_ORDERS.map((v) => ({ label: v, value: v })),
  },
  { prop: "scale", name: "缩放倍数", size: 110, editor: EDITOR_NUMBER, cellProperties: rowCellProps("scale") },
  { prop: "quality", name: "quality", size: 110, readonly: true },
  { prop: "valueDisplay", name: "实时值", size: 160, readonly: true },
  { prop: "errorMessage", name: "error", size: 220, readonly: true },
  { prop: "timestamp", name: "timestamp", size: 200, readonly: true },
  { prop: "durationMs", name: "ms", size: 90, readonly: true },
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
  return {
    ...p,
    __selected: false,
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
    plan.value = null;
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
    const built = await commPlanBuild({ profiles: filteredProfiles, points: filteredPoints }, projectId.value);
    plan.value = built;
    const map: Record<string, number> = {};
    for (const job of built.jobs) {
      for (const point of job.points) {
        map[point.pointKey] = job.startAddress + point.offset;
      }
    }
    start0ByPointKey.value = map;
  } catch {
    plan.value = null;
    start0ByPointKey.value = {};
  } finally {
    rebuildGridRows();
  }
}

let suppressChannelWatch = false;

async function loadAll() {
  try {
    const pid = projectId.value.trim();
    if (!pid) {
      profiles.value = { schemaVersion: 1, profiles: [] };
      points.value = { schemaVersion: 1, points: [] };
      activeChannelName.value = "";
      return;
    }

    const project = await commProjectLoadV1(pid);
    profiles.value = project.connections ?? { schemaVersion: 1, profiles: [] };
    points.value = project.points ?? { schemaVersion: 1, points: [] };
    showAllValidation.value = false;
    touchedRowKeys.value = {};
    markPointsChanged();

    suppressChannelWatch = true;
    const ui = project.uiState;
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
        insertMode: (t.insertMode as any) === "afterSelection" ? "afterSelection" : "append",
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
  await commPointsSave(points.value, projectId.value);
  ElMessage.success("已保存 points");
  showAllValidation.value = false;
  touchedRowKeys.value = {};
}

type BatchInsertMode = "append" | "afterSelection";

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

function openBatchAddDialog() {
  const profile = activeProfile.value;
  if (!profile) {
    ElMessage.error("请先选择连接");
    return;
  }

  let suggestedStart = formatHumanAddressFrom0Based(profile.readArea, profile.startAddress);
  const lastRow = gridRows.value[gridRows.value.length - 1];
  if (lastRow?.modbusAddress?.trim()) {
    const next = nextAddress(lastRow.modbusAddress, lastRow.dataType);
    if (next.ok) suggestedStart = next.nextHumanAddr;
  }

  batchAddTemplate.value = {
    count: Math.max(1, Math.min(500, Math.floor(batchAddTemplate.value.count || 10))),
    startAddressHuman: suggestedStart,
    dataType: lastRow?.dataType ?? batchAddTemplate.value.dataType ?? "UInt16",
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

  points.value.points.splice(insertIndex, 0, ...built.points);
  markPointsChanged();
  batchAddDrawerOpen.value = false;
  await rebuildPlan();

  await nextTick(async () => {
    const grid = gridEl();
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
  const grid = gridEl();
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
  const selected = gridRows.value.filter((r) => r.__selected);
  if (selected.length === 0) {
    ElMessage.warning("请先勾选要删除的行");
    return;
  }
  const count = selected.length;

  await ElMessageBox.confirm(`确认删除选中的 ${count} 行点位？`, "删除点位", {
    confirmButtonText: "删除",
    cancelButtonText: "取消",
    type: "warning",
  });

  const selectedKeys = new Set(selected.map((r) => r.pointKey));
  points.value.points = points.value.points.filter((p) => !selectedKeys.has(p.pointKey));
  markPointsChanged();
  await rebuildPlan();
  ElMessage.success(`已删除 ${count} 行`);
}

async function fillDownFromSelection() {
  const sel = await getSelectedRange();
  if (!sel) {
    ElMessage.warning("请先框选一个单元格区域");
    return;
  }
  const { rowStart, rowEnd, colStart, colEnd } = sel;
  if (rowEnd <= rowStart) {
    ElMessage.warning("请框选至少两行");
    return;
  }

  const grid = gridEl();
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

async function fillAddressFromSelection() {
  const profile = activeProfile.value;
  if (!profile) {
    ElMessage.error("请先选择连接");
    return;
  }

  const sel = await getSelectedRange();
  if (!sel) {
    ElMessage.warning("请先框选一个行区域（至少两行）");
    return;
  }
  const { rowStart, rowEnd } = sel;
  if (rowEnd <= rowStart) {
    ElMessage.warning("请框选至少两行");
    return;
  }

  await ElMessageBox.confirm(
    `确认对选区行自动递增填充 Modbus 地址？（将覆盖 ${rowEnd - rowStart + 1} 行的地址列）`,
    "Fill Address",
    {
      confirmButtonText: "填充",
      cancelButtonText: "取消",
      type: "warning",
    }
  );

  const grid = gridEl();
  if (!grid) return;

  const computed = computeFillAddressEdits({ rows: gridRows.value, range: { rowStart, rowEnd } });
  if (!computed.ok) {
    ElMessage.error(computed.message);
    return;
  }

  // Validate against active profile bounds.
  const channelEnd0 = profile.startAddress + profile.length;
  for (const e of computed.edits) {
    const parsed = parseHumanAddress(e.value);
    if (!parsed.ok) {
      ElMessage.error(parsed.message);
      return;
    }
    if (parsed.area !== profile.readArea) {
      ElMessage.error(`地址区域不匹配：profile.readArea=${profile.readArea}，输入=${parsed.area}`);
      return;
    }
    const row = gridRows.value[e.rowIndex];
    const len = row ? spanForArea(profile.readArea, row.dataType) : null;
    if (len === null) {
      ElMessage.error(`dataType=${row?.dataType ?? "?"} 与 readArea=${profile.readArea} 不匹配（row=${e.rowIndex + 1}）`);
      return;
    }
    if (parsed.start0Based < profile.startAddress) {
      ElMessage.error("地址小于连接起始地址");
      return;
    }
    if (parsed.start0Based + len > channelEnd0) {
      ElMessage.error(`地址越界：end=${parsed.start0Based + len} > channelEnd=${channelEnd0}`);
      return;
    }
  }

  const addrCol = colIndexByProp.value["modbusAddress"];
  for (const e of computed.edits) {
    await grid.setDataAt({ row: e.rowIndex, col: addrCol, rowType: "rgRow", colType: "rgCol", val: e.value });
  }

  const touched = gridRows.value.slice(rowStart, rowEnd + 1).map((r) => r.pointKey);
  await syncFromGridAndMapAddresses(touched);
  markPointsChanged();
  ElMessage.success(`已填充地址：${rowEnd - rowStart + 1} 行`);
}

async function applyBatch() {
  const selected = gridRows.value
    .map((row, rowIndex) => ({ row, rowIndex }))
    .filter((v) => v.row.__selected);
  if (selected.length === 0) {
    ElMessage.warning("请先勾选要批量设置的行");
    return;
  }

  const grid = gridEl();
  if (!grid) return;
  const dataTypeCol = colIndexByProp.value["dataType"];
  const byteOrderCol = colIndexByProp.value["byteOrder"];
  const scaleCol = colIndexByProp.value["scale"];

  const compiled = batchScaleExpr.value.trim() ? compileScaleExpr(batchScaleExpr.value) : null;
  if (compiled && !compiled.ok) {
    ElMessage.error(`缩放表达式错误：${compiled.message}`);
    return;
  }

  let changed = 0;
  for (const { row, rowIndex } of selected) {
    if (batchDataType.value) {
      await grid.setDataAt({ row: rowIndex, col: dataTypeCol, rowType: "rgRow", colType: "rgCol", val: batchDataType.value });
      changed += 1;
    }
    if (batchByteOrder.value) {
      await grid.setDataAt({ row: rowIndex, col: byteOrderCol, rowType: "rgRow", colType: "rgCol", val: batchByteOrder.value });
      changed += 1;
    }
    if (compiled?.ok) {
      const oldScale = Number(row.scale);
      try {
        const next = compiled.apply(Number.isFinite(oldScale) ? oldScale : 0);
        await grid.setDataAt({ row: rowIndex, col: scaleCol, rowType: "rgRow", colType: "rgCol", val: next });
        changed += 1;
      } catch (e: unknown) {
        ElMessage.error(String((e as any)?.message ?? e ?? "缩放表达式计算失败"));
        return;
      }
    }
  }

  const touched = selected.map((v) => v.row.pointKey);
  await syncFromGridAndMapAddresses(touched);
  markPointsChanged();

  ElMessage.success(`已批量修改 ${selected.length} 行（${changed} 单元格）`);
}

const batchDataType = ref<DataType | "">("");
const batchByteOrder = ref<ByteOrder32 | "">("");
const batchScaleExpr = ref<string>("");

async function syncFromGridAndMapAddresses(touchedKeys?: string[]) {
  const grid = gridEl();
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

    const parsed = parseHumanAddress(addrRaw);
    if (!parsed.ok) continue;
    if (parsed.area !== profile.readArea) continue;

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

async function buildPlanForDisplay() {
  await syncFromGridAndMapAddresses();
  await rebuildPlan();
  ElMessage.success(`已生成 plan：jobs=${plan.value?.jobs.length ?? 0}`);
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

  const grid = gridEl();
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
      projectId.value
    );
    plan.value = planToUse;
    pushLog("run_start", "info", `plan ok: jobs=${planToUse.jobs.length}`);

    pushLog("run_start", "info", "调用 comm_run_start_obs（后端 spawn，不阻塞 UI）");
    const resp = await commRunStartObs(
      {
        profiles: { schemaVersion: 1, profiles: [profile] },
        points: { schemaVersion: 1, points: channelPoints },
        plan: planToUse,
      },
      projectId.value
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
    return;
  }

  latest.value = resp.value;
  applyLatestToGridRows(resp.value.results);
  pushLog("run_latest", "success", `ok: total=${resp.value.stats.total} ok=${resp.value.stats.ok}`);
}

async function stopRun() {
  if (!runId.value || runUiState.value !== "running") return;
  runUiState.value = "stopping";
  const id = runId.value;
  pushLog("run_stop", "info", "stop clicked");

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
  pushLog("run_restart", "info", "restart clicked");
  await stopRun();
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
  void syncFromGridAndMapAddresses(touched);
  markPointsChanged();
}

watch(activeChannelName, async (v) => {
  if (suppressChannelWatch) return;
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

watch(projectId, loadAll, { immediate: true });

onBeforeUnmount(() => {
  clearTimer();
});
</script>

<template>
  <el-card>
    <template #header>
      <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px">
        <div style="font-weight: 600">点位配置 + 实时采集</div>
        <el-space wrap>
          <el-button @click="loadAll">加载</el-button>
          <el-button @click="savePoints">保存</el-button>
        </el-space>
      </div>
    </template>

    <el-alert
      type="info"
      show-icon
      :closable="false"
      title="提示：在表格中直接编辑；支持键盘导航与复制粘贴（Excel TSV）；框选区域后可 Fill Down / 批量设置。"
      style="margin-bottom: 12px"
    />

    <el-space wrap style="margin-bottom: 12px">
      <el-select
        v-model="activeChannelName"
        placeholder="选择连接"
        style="width: 520px"
        :disabled="isRunning || isStarting || isStopping"
      >
        <el-option v-for="p in profiles.profiles" :key="p.channelName" :label="profileLabel(p)" :value="p.channelName" />
      </el-select>
      <el-button type="primary" :loading="isStarting" :disabled="isRunning || isStopping" @click="startRun">开始运行</el-button>
      <el-button type="danger" :loading="isStopping" :disabled="!isRunning" @click="stopRun">停止</el-button>
      <el-button v-if="isRunning && configChangedDuringRun" type="warning" :disabled="isStarting || isStopping" @click="restartRun"
        >配置已变更：重启使其生效</el-button
      >
      <el-select v-model="pollMs" style="width: 160px">
        <el-option :value="500" label="轮询 500ms" />
        <el-option :value="1000" label="轮询 1s" />
        <el-option :value="2000" label="轮询 2s" />
      </el-select>
      <el-tag v-if="runId" type="info">runId={{ runId }}</el-tag>
      <el-tag v-if="runUiState === 'running'" type="success">running</el-tag>
      <el-tag v-else-if="runUiState === 'starting'" type="warning">starting</el-tag>
      <el-tag v-else-if="runUiState === 'stopping'" type="warning">stopping</el-tag>
      <el-tag v-else-if="runUiState === 'error'" type="danger">error</el-tag>
      <el-tag v-else type="info">idle</el-tag>
      <el-tag v-if="latest" type="info">updatedAt={{ latest.updatedAtUtc }}</el-tag>
    </el-space>

    <el-space wrap style="margin-bottom: 12px">
      <el-button type="primary" @click="openBatchAddDialog">批量新增</el-button>
      <el-button type="danger" :disabled="selectedCount === 0" @click="removeSelectedRows">删除选中行（{{ selectedCount }}）</el-button>
      <el-select v-model="batchDataType" placeholder="数据类型（批量）" style="width: 180px">
        <el-option label="数据类型（不修改）" value="" />
        <el-option v-for="opt in DATA_TYPES" :key="opt" :label="opt" :value="opt" />
      </el-select>
      <el-select v-model="batchByteOrder" placeholder="字节序（批量）" style="width: 160px">
        <el-option label="字节序（不修改）" value="" />
        <el-option v-for="opt in BYTE_ORDERS" :key="opt" :label="opt" :value="opt" />
      </el-select>
      <el-input v-model="batchScaleExpr" placeholder="缩放表达式：2 或 {{x}}*10 或 ({{x}}+1)*0.5" style="width: 360px" />
      <el-button type="primary" :disabled="selectedCount === 0" @click="applyBatch">Apply（对选中行）</el-button>
    </el-space>

    <el-divider />

    <el-row :gutter="12" style="margin-bottom: 12px">
      <el-col :span="4"><el-statistic title="Total" :value="latest?.stats.total ?? 0" /></el-col>
      <el-col :span="4"><el-statistic title="OK" :value="latest?.stats.ok ?? 0" /></el-col>
      <el-col :span="4"><el-statistic title="Timeout" :value="latest?.stats.timeout ?? 0" /></el-col>
      <el-col :span="4"><el-statistic title="CommErr" :value="latest?.stats.commError ?? 0" /></el-col>
      <el-col :span="4"><el-statistic title="DecodeErr" :value="latest?.stats.decodeError ?? 0" /></el-col>
      <el-col :span="4"><el-statistic title="CfgErr" :value="latest?.stats.configError ?? 0" /></el-col>
    </el-row>

    <el-alert
      v-if="runError"
      style="margin-bottom: 12px"
      type="error"
      show-icon
      :closable="false"
      :title="`${runError.kind}: ${runError.message}`"
    />

    <el-card v-if="runError?.details?.missingFields?.length" shadow="never" style="margin-bottom: 12px">
      <template #header>配置缺失/非法字段（后端校验返回）</template>
      <el-table :data="runError.details!.missingFields" size="small" height="220">
        <el-table-column prop="hmiName" label="HMI" min-width="120" />
        <el-table-column prop="pointKey" label="pointKey" min-width="160" />
        <el-table-column prop="field" label="field" min-width="120" />
        <el-table-column prop="reason" label="reason" min-width="160" />
      </el-table>
    </el-card>

    <Grid
      ref="gridRef"
      :source="gridRows"
      :columns="columns"
      :editors="gridEditors"
      :range="true"
      :useClipboard="true"
      :canFocus="true"
      :resize="true"
      :rowHeaders="true"
      style="height: 560px"
      @afteredit="onAfterEdit"
    />

    <el-collapse v-model="advancedOpen" style="margin-top: 12px">
      <el-collapse-item name="tools" title="高级/工具（Fill/Plan/诊断）">
        <el-space wrap>
          <el-button @click="buildPlanForDisplay">生成 Plan（用于地址预览）</el-button>
          <el-tag v-if="activeProfile" type="info">readArea={{ activeProfile.readArea }}</el-tag>
          <el-tag v-if="activeProfile" type="info">start={{ activeProfile.startAddress + 1 }}</el-tag>
          <el-tag v-if="activeProfile" type="info">len={{ activeProfile.length }}</el-tag>
          <el-tag v-if="plan" type="info">jobs={{ plan.jobs.length }}</el-tag>

          <el-divider direction="vertical" />

          <el-button @click="fillDownFromSelection">Fill Down（同值填充）</el-button>
          <el-button @click="fillAddressFromSelection">Fill Address（递增）</el-button>
        </el-space>
      </el-collapse-item>

      <el-collapse-item name="logs" title="最近调用日志（最多 20 条）">
        <el-table :data="logs" size="small" height="220" style="width: 100%">
          <el-table-column prop="ts" label="ts" width="190" />
          <el-table-column prop="step" label="step" width="120" />
          <el-table-column prop="level" label="level" width="100" />
          <el-table-column prop="message" label="message" />
        </el-table>
      </el-collapse-item>
    </el-collapse>

    <el-dialog v-model="batchAddDrawerOpen" title="批量新增（模板）" width="980px">
      <el-row :gutter="12">
        <el-col :span="12">
          <el-form label-width="140px">
            <el-form-item label="行数（N）">
              <el-input-number v-model="batchAddTemplate.count" :min="1" :max="500" />
            </el-form-item>
            <el-form-item label="起始 Modbus 地址">
              <el-input v-model="batchAddTemplate.startAddressHuman" placeholder="例如 40001" />
            </el-form-item>
            <el-form-item label="数据类型（步长）">
              <el-select v-model="batchAddTemplate.dataType" style="width: 220px">
                <el-option v-for="opt in DATA_TYPES" :key="opt" :label="opt" :value="opt" />
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
                支持占位符：<code v-pre>{{i}}</code>（从 1 开始） / <code v-pre>{{addr}}</code>（如 40001）
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
  </el-card>
</template>

<style scoped>
:deep(.comm-cell-error) {
  background: transparent;
  box-shadow: inset 0 0 0 1px rgba(245, 108, 108, 0.85);
}

:deep(.comm-rg-editor) {
  width: 100%;
  box-sizing: border-box;
  height: 28px;
  padding: 0 6px;
  border: 1px solid var(--el-border-color);
  border-radius: 2px;
  font-size: 12px;
  outline: none;
}

:deep(.comm-rg-editor:focus) {
  border-color: var(--el-color-primary);
}
</style>
