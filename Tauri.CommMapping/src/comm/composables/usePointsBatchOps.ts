import { computed, ref, type ComputedRef, type Ref } from "vue";
import { ElMessage } from "element-plus";

import type {
  BatchInsertMode,
  ByteOrder32,
  CommPoint,
  ConnectionProfile,
  DataType,
  PointsV1,
  RegisterArea,
} from "../api";
import { buildBatchPointsTemplate, previewBatchPointsTemplate } from "../services/batchAdd";
import { applyBatchEdits, computeBatchEdits, type BatchEditRequest } from "../services/batchEdit";
import { inferNextAddress } from "../services/address";
import { patchProjectUiState } from "../services/projects";
import { createBatchAddUndoAction, createBatchEditUndoAction } from "../services/undoRedo";
import type { SelectionRange } from "../services/fill";
import type { UndoManager } from "../services/undoRedo";
import type { PointRowLike } from "./usePointsRows";

export type BatchAddTemplate = {
  count: number;
  startAddressHuman: string;
  dataType: DataType;
  byteOrder: ByteOrder32;
  hmiNameTemplate: string;
  scaleTemplate: string;
  insertMode: BatchInsertMode;
};

type ReplaceScope = "selected" | "all";
type RowSpan = { rowStart: number; rowEnd: number };
type LogLevel = "info" | "success" | "warning" | "error";

export interface UsePointsBatchOpsOptions<T extends PointRowLike> {
  gridRows: Ref<T[]>;
  points: Ref<PointsV1>;
  activeProfile: ComputedRef<ConnectionProfile | null>;
  selectedCount: ComputedRef<number>;
  effectiveSelectedKeySet: ComputedRef<Set<string>>;
  selectedRangeRows: Ref<RowSpan | null>;
  getSelectedRange: () => Promise<SelectionRange | null>;
  gridApi: () => any;
  undoManager: UndoManager;
  markPointsChanged: () => void;
  rebuildPlan: () => Promise<void>;
  resolveDataTypeForArea: (area: RegisterArea, preferred?: DataType | null) => DataType;
  projectId: Ref<string>;
  pushLog: (scope: string, level: LogLevel, message: string) => void;
  onTouched: (keys: string[]) => void;
}

export function usePointsBatchOps<T extends PointRowLike>(options: UsePointsBatchOpsOptions<T>) {
  const {
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
    onTouched,
  } = options;

  const batchAddDrawerOpen = ref(false);
  const batchAddTemplate = ref<BatchAddTemplate>({
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
      },
      10
    );
  });

  const batchEditDialogVisible = ref(false);

  const replaceDialogOpen = ref(false);
  const replaceForm = ref<{ find: string; replace: string; scope: ReplaceScope }>({
    find: "",
    replace: "",
    scope: "all",
  });
  const replacePreviewLimit = 8;

  function replaceAllLiteral(input: string, find: string, replace: string): { value: string; count: number } {
    if (!find) return { value: input, count: 0 };
    const parts = String(input).split(find);
    return { value: parts.join(replace), count: Math.max(0, parts.length - 1) };
  }

  const replacePreview = computed(() => {
    const find = replaceForm.value.find;
    const replaceValue = replaceForm.value.replace;
    const scope = replaceForm.value.scope;
    const targetSet = scope === "selected" ? effectiveSelectedKeySet.value : null;
    const preview: Array<{ rowIndex: number; before: string; after: string; count: number; pointKey: string }> = [];
    let matchedRows = 0;
    let replaceCount = 0;

    if (!find) {
      return { matchedRows, replaceCount, preview };
    }

    for (let i = 0; i < gridRows.value.length; i++) {
      const row = gridRows.value[i];
      if (targetSet && !targetSet.has(row.pointKey)) continue;
      const before = String(row.hmiName ?? "");
      if (!before) continue;
      const result = replaceAllLiteral(before, find, replaceValue);
      if (result.count <= 0 || result.value === before) continue;
      matchedRows += 1;
      replaceCount += result.count;
      if (preview.length < replacePreviewLimit) {
        preview.push({
          rowIndex: i + 1,
          before,
          after: result.value,
          count: result.count,
          pointKey: row.pointKey,
        });
      }
    }

    return { matchedRows, replaceCount, preview };
  });

  function openBatchAddDialog() {
    const profile = activeProfile.value;
    if (!profile) {
      ElMessage.error("请先选择连接");
      return;
    }

    const lastRow = gridRows.value[gridRows.value.length - 1];
    const preferredType = lastRow?.dataType ?? batchAddTemplate.value.dataType;
    const resolvedType = resolveDataTypeForArea(profile.readArea, preferredType);

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
    });
    if (!built.ok) {
      ElMessage.error(built.message);
      return;
    }

    const insertIndex = (() => {
      if (batchAddTemplate.value.insertMode === "afterSelection") {
        const selectedRowIndex = Math.max(
          ...gridRows.value.map((row, idx) => (row.__selected ? idx : -1))
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

    await gridScrollToPoint(built.points[0]?.pointKey);

    ElMessage.success(`已新增 ${built.points.length} 行（步长=${built.span}）`);

    const pid = projectId.value.trim();
    if (pid) {
      patchProjectUiState(pid, {
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

  async function openReplaceDialog() {
    const sel = await getSelectedRange();
    if (sel) {
      selectedRangeRows.value = { rowStart: sel.rowStart, rowEnd: sel.rowEnd };
    }

    replaceForm.value.scope = selectedCount.value > 0 ? "selected" : "all";
    replaceDialogOpen.value = true;
  }

  async function confirmReplaceHmiNames() {
    const find = replaceForm.value.find;
    if (!find) {
      ElMessage.error("查找内容不能为空");
      return;
    }

    if (replaceForm.value.scope === "selected" && selectedCount.value === 0) {
      ElMessage.warning("请先选中要替换的行");
      return;
    }

    const replaceValue = replaceForm.value.replace;
    const targetSet = replaceForm.value.scope === "selected" ? effectiveSelectedKeySet.value : null;
    const changes: Array<{ pointKey: string; before: string; after: string; count: number }> = [];
    let totalCount = 0;

    for (const row of gridRows.value) {
      if (targetSet && !targetSet.has(row.pointKey)) continue;
      const before = String(row.hmiName ?? "");
      if (!before) continue;
      const result = replaceAllLiteral(before, find, replaceValue);
      if (result.count <= 0 || result.value === before) continue;
      totalCount += result.count;
      changes.push({ pointKey: row.pointKey, before, after: result.value, count: result.count });
    }

    if (changes.length === 0) {
      ElMessage.info("未找到需要替换的变量名称");
      return;
    }

    const undoAction = createBatchEditUndoAction(
      () => points.value.points,
      (newPoints: CommPoint[]) => {
        points.value.points = newPoints;
      },
      `变量名称替换：${changes.length} 行 / ${totalCount} 处`
    );

    const changeMap = new Map(changes.map((change) => [change.pointKey, change.after]));
    for (const point of points.value.points) {
      const next = changeMap.get(point.pointKey);
      if (next !== undefined) point.hmiName = next;
    }

    for (const row of gridRows.value) {
      const next = changeMap.get(row.pointKey);
      if (next !== undefined) row.hmiName = next;
    }
    gridRows.value = [...gridRows.value];

    undoManager.push(undoAction);

    const touchedKeys = changes.map((change) => change.pointKey);
    if (touchedKeys.length > 0) {
      onTouched(touchedKeys);
    }

    replaceDialogOpen.value = false;
    markPointsChanged();
    await rebuildPlan();

    ElMessage.success(`已替换 ${changes.length} 行 / ${totalCount} 处`);
  }

  async function gridScrollToPoint(pointKey?: string) {
    const grid = gridApi();
    if (!grid || typeof grid.scrollToRow !== "function") return;
    const rowIndex = pointKey ? gridRows.value.findIndex((r) => r.pointKey === pointKey) : -1;
    await grid.scrollToRow(rowIndex >= 0 ? rowIndex : gridRows.value.length - 1);
  }

  return {
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
  };
}
