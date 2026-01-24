import { nextTick, type ComputedRef, type Ref } from "vue";

import type { SelectionRange } from "../services/fill";
import type { FocusedIssueCell } from "./usePointsColumns";
import type { PointRowLike } from "./usePointsRows";

type RowSpan = { rowStart: number; rowEnd: number };

export type ValidationIssueJump = {
  pointKey: string;
  field?: string;
};

export interface UsePointsGridEventsOptions<T extends PointRowLike> {
  gridRows: Ref<T[]>;
  selectedRangeRows: Ref<RowSpan | null>;
  focusedIssueCell: Ref<FocusedIssueCell<T> | null>;
  validationPanelOpen: Ref<boolean>;
  colIndexByProp: ComputedRef<Record<string, number>>;
  gridApi: () => any;
  fillMode: Ref<"copy" | "series">;
  applyFillDown: (range: SelectionRange) => Promise<void>;
  applyFillSeries: (range: SelectionRange) => Promise<void>;
  appendRows: (count: number, baseRow?: T | null) => Promise<void>;
  syncFromGridAndMapAddresses: (touchedKeys?: string[]) => Promise<void>;
  markPointsChanged: () => void;
  handleUndo: () => void;
  handleRedo: () => void;
}

export function usePointsGridEvents<T extends PointRowLike>(options: UsePointsGridEventsOptions<T>) {
  function collectTouchedPointKeysFromAfterEdit(e: any): string[] {
    const keys = new Set<string>();
    const detail = e?.detail ?? e;

    if (detail?.model?.pointKey) keys.add(String(detail.model.pointKey));
    if (detail?.models && typeof detail.models === "object") {
      for (const value of Object.values(detail.models)) {
        if (value && typeof value === "object" && "pointKey" in value) {
          keys.add(String((value as any).pointKey));
        }
      }
    }
    return [...keys];
  }

  function onAfterEdit(e: any) {
    const touched = collectTouchedPointKeysFromAfterEdit(e);
    options.focusedIssueCell.value = null;
    void options.syncFromGridAndMapAddresses(touched);
    options.markPointsChanged();
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
      options.handleUndo();
      e.preventDefault?.();
      original.preventDefault();
      original.stopPropagation();
      return;
    }

    if (key === "y" || (key === "z" && original.shiftKey)) {
      options.handleRedo();
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

    const missing = rowEnd - (options.gridRows.value.length - 1);
    if (missing > 0) {
      const baseRow = options.gridRows.value[rowStart] ?? options.gridRows.value[options.gridRows.value.length - 1] ?? null;
      await options.appendRows(missing, baseRow);
      await nextTick();
    }

    const selRange: SelectionRange = { rowStart, rowEnd, colStart, colEnd };
    if (options.fillMode.value === "copy") {
      await options.applyFillDown(selRange);
    } else {
      await options.applyFillSeries(selRange);
    }
  }

  async function jumpToIssue(issue: ValidationIssueJump) {
    const rowIndex = options.gridRows.value.findIndex((row) => row.pointKey === issue.pointKey);
    if (rowIndex < 0) return;

    options.selectedRangeRows.value = null;
    options.gridRows.value.forEach((row) => {
      row.__selected = row.pointKey === issue.pointKey;
    });
    options.gridRows.value = [...options.gridRows.value];

    await nextTick();
    const grid = options.gridApi();
    if (!grid) return;
    if (typeof grid.scrollToRow === "function") {
      await grid.scrollToRow(rowIndex);
    }

    if (issue.field) {
      const colKey = String(issue.field);
      const colIndex = options.colIndexByProp.value[colKey];
      if (typeof grid.scrollToColumnIndex === "function" && typeof colIndex === "number") {
        await grid.scrollToColumnIndex(colIndex);
      } else if (typeof grid.scrollToColumnProp === "function") {
        await grid.scrollToColumnProp(colKey);
      } else if (typeof grid.scrollToCoordinate === "function" && typeof colIndex === "number") {
        await grid.scrollToCoordinate({ x: colIndex, y: rowIndex });
      }
    }
  }

  async function handleJumpToIssue(issue: ValidationIssueJump) {
    options.validationPanelOpen.value = false;
    if (issue.field && Object.prototype.hasOwnProperty.call(options.colIndexByProp.value, issue.field)) {
      options.focusedIssueCell.value = { pointKey: issue.pointKey, field: issue.field as keyof T };
    } else {
      options.focusedIssueCell.value = null;
    }
    await nextTick();
    await jumpToIssue(issue);
  }

  return {
    onAfterEdit,
    onBeforeGridKeyDown,
    onBeforeAutofill,
    handleJumpToIssue,
  };
}
