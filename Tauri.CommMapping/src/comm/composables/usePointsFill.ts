import { computed, ref, type ComputedRef, type Ref } from "vue";
import { ElMessage } from "element-plus";
import type { ColumnRegular } from "@revolist/vue3-datagrid";

import type { ConnectionProfile, DataType } from "../api";
import { parseHumanAddress, spanForArea } from "../services/address";
import { computeFillAddressEdits, computeFillDownEdits, type SelectionRange } from "../services/fill";
import type { GridRangeSelection } from "./usePointsGrid";

type FillRow = {
  pointKey: string;
  modbusAddress: string;
  dataType: DataType;
};

interface UsePointsFillOptions<T extends FillRow> {
  gridRows: Ref<T[]>;
  columns: ComputedRef<ColumnRegular[]>;
  colIndexByProp: ComputedRef<Record<string, number>>;
  activeProfile: ComputedRef<ConnectionProfile | null>;
  gridApi: () => any | null;
  getSelectedRange: () => Promise<GridRangeSelection | null>;
  syncFromGridAndMapAddresses: (touchedKeys?: string[]) => Promise<void>;
  markPointsChanged: () => void;
  rowSelectedProp?: string;
}

export function usePointsFill<T extends FillRow>(options: UsePointsFillOptions<T>) {
  const fillMode = ref<"copy" | "series">("copy");
  const fillModeLabel = computed(() => (fillMode.value === "copy" ? "同值填充" : "序列递增"));

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

    const grid = options.gridApi();
    if (!grid) return;
    const propsByColIndex = options.columns.value.map((c) => String(c.prop ?? ""));
    const skipProps = new Set([
      options.rowSelectedProp ?? "__selected",
      "pointKey",
      "quality",
      "valueDisplay",
      "errorMessage",
      "timestamp",
      "durationMs",
    ]);
    const { edits, changed } = computeFillDownEdits({
      rows: options.gridRows.value,
      propsByColIndex,
      range: { rowStart, rowEnd, colStart, colEnd },
      skipProps,
    });
    for (const e of edits) {
      await grid.setDataAt({ row: e.rowIndex, col: e.colIndex, rowType: "rgRow", colType: "rgCol", val: e.value });
    }
    const touched = options.gridRows.value.slice(rowStart, rowEnd + 1).map((r) => r.pointKey);
    await options.syncFromGridAndMapAddresses(touched);
    options.markPointsChanged();
    ElMessage.success(`已向下填充：${rowEnd - rowStart} 行 × ${colEnd - colStart + 1} 列（${changed} 单元格）`);
  }

  async function fillDownFromSelection() {
    const sel = await options.getSelectedRange();
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

    const grid = options.gridApi();
    if (!grid) return;

    const propsByColIndex = options.columns.value.map((c) => String(c.prop ?? ""));
    const skipProps = new Set([
      options.rowSelectedProp ?? "__selected",
      "pointKey",
      "quality",
      "valueDisplay",
      "errorMessage",
      "timestamp",
      "durationMs",
    ]);

    const baseRow = options.gridRows.value[rowStart];
    if (!baseRow) return;

    const addrCol = options.colIndexByProp.value["modbusAddress"];
    const includeAddress = addrCol >= colStart && addrCol <= colEnd;
    const profile = options.activeProfile.value;

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
        rows: options.gridRows.value,
        range: { rowStart, rowEnd },
        readArea: active.readArea,
      });
      if (!computed.ok) {
        ElMessage.error(computed.message);
        return;
      }

      for (const e of computed.edits) {
        const parsed = parseHumanAddress(e.value, active.readArea);
        if (!parsed.ok) {
          ElMessage.error(parsed.message);
          return;
        }
        const row = options.gridRows.value[e.rowIndex];
        const len = row ? spanForArea(active.readArea, row.dataType) : null;
        if (len === null) {
          ElMessage.error(`数据类型 ${row?.dataType ?? "?"} 与读取区域 ${active.readArea} 不匹配（行 ${e.rowIndex + 1}）`);
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

    const touched = options.gridRows.value.slice(rowStart, rowEnd + 1).map((r) => r.pointKey);
    await options.syncFromGridAndMapAddresses(touched);
    options.markPointsChanged();
    ElMessage.success(`已序列填充：${rowEnd - rowStart + 1} 行`);
  }

  async function fillSeriesFromSelection() {
    const sel = await options.getSelectedRange();
    if (!sel) {
      ElMessage.warning("请先框选一个单元格区域");
      return;
    }
    await applyFillSeries(sel);
  }

  return {
    fillMode,
    fillModeLabel,
    handleFillCommand,
    applyFill,
    applyFillDown,
    applyFillSeries,
    fillDownFromSelection,
    fillSeriesFromSelection,
  };
}
