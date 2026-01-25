import { nextTick, type ComputedRef, type Ref } from "vue";
import type { CommPoint, ConnectionProfile, DataType, PointsV1, RegisterArea } from "../api";
import { formatHumanAddressFrom0Based, inferNextAddress, parseHumanAddress } from "../services/address";
import { newPointKey } from "../services/ids";
import { confirmAction, notifyError, notifySuccess, notifyWarning } from "../services/notify";
import { createBatchAddUndoAction, createDeleteRowsUndoAction } from "../services/undoRedo";
import type { SelectionRange } from "../services/fill";
import type { UndoManager } from "../services/undoRedo";
import type { PointRowLike } from "./usePointsRows";

type RowSpan = { rowStart: number; rowEnd: number };

export interface UsePointsRowOpsOptions<T extends PointRowLike> {
  gridRows: Ref<T[]>;
  points: Ref<PointsV1>;
  activeProfile: ComputedRef<ConnectionProfile | null>;
  activeChannelName: Ref<string>;
  selectedCount: ComputedRef<number>;
  effectiveSelectedKeySet: ComputedRef<Set<string>>;
  effectiveSelectedRows: ComputedRef<T[]>;
  selectedRangeRows: Ref<RowSpan | null>;
  getSelectedRange: () => Promise<SelectionRange | null>;
  gridApi: () => any;
  undoManager: UndoManager;
  markPointsChanged: () => void;
  rebuildPlan: () => Promise<void>;
  resolveDataTypeForArea: (area: RegisterArea, preferred?: DataType | null) => DataType;
}

export function usePointsRowOps<T extends PointRowLike>(options: UsePointsRowOpsOptions<T>) {
  const {
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
  } = options;

  function findInsertAnchor(): { row: T; rowIndex: number } | null {
    if (selectedCount.value === 0) return null;
    const selectedSet = effectiveSelectedKeySet.value;
    let anchorRow: T | null = null;
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

  function buildSinglePoint(profile: ConnectionProfile, baseRow?: T | null): CommPoint {
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

  function buildSinglePointWithoutProfile(channelName: string, baseRow?: T | null): CommPoint {
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

  function buildTempRowFromPoint(point: CommPoint, profile: ConnectionProfile): T {
    const addr =
      typeof point.addressOffset === "number"
        ? formatHumanAddressFrom0Based(profile.readArea, profile.startAddress + point.addressOffset)
        : "";
    return {
      ...(point as T),
      __selected: false,
      modbusAddress: addr,
      quality: "",
      valueDisplay: "",
      errorMessage: "",
      timestamp: "",
      durationMs: "",
    };
  }

  async function appendRows(count: number, baseRow?: T | null) {
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
      notifyError("请先选择连接");
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
      gridRows.value.forEach((row, idx) => {
        row.__selected = idx === rowIndex;
      });
      gridRows.value = [...gridRows.value];

      const grid = gridApi();
      if (grid && typeof grid.scrollToRow === "function") {
        await grid.scrollToRow(rowIndex);
      }
    });

    notifySuccess("已新增 1 行");
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
      notifyWarning("请先选中行（点击行号）或框选一段行区域");
      return;
    }
    const count = selected.length;

    const ok = await confirmAction(`确认删除选中的 ${count} 行点位？`, "删除点位", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    if (!ok) return;

    const selectedKeys = new Set(selected.map((row) => row.pointKey));

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
    notifySuccess(`已删除 ${count} 行`);
  }

  return {
    appendRows,
    addSingleRow,
    removeSelectedRows,
  };
}
