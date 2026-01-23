import { computed, ref, type Ref } from "vue";

type GridRowBase = {
  pointKey: string;
  __selected: boolean;
};

export type GridRangeSelection = {
  rowStart: number;
  rowEnd: number;
  colStart: number;
  colEnd: number;
};

interface UsePointsGridOptions<T extends GridRowBase> {
  gridRef: Ref<any>;
  gridRows: Ref<T[]>;
  appendRows?: (count: number, baseRow?: T | null) => Promise<void>;
  clearFocus?: () => void;
}

export function usePointsGrid<T extends GridRowBase>(options: UsePointsGridOptions<T>) {
  const selectedRangeRows = ref<{ rowStart: number; rowEnd: number } | null>(null);
  let lastRowSelectionIndex: number | null = null;

  const explicitSelectedKeys = computed<string[]>(() => options.gridRows.value.filter((r) => r.__selected).map((r) => r.pointKey));

  const rangeSelectedKeys = computed<string[]>(() => {
    const span = selectedRangeRows.value;
    if (!span) return [];
    const start = Math.max(0, Math.min(span.rowStart, span.rowEnd));
    const end = Math.min(options.gridRows.value.length - 1, Math.max(span.rowStart, span.rowEnd));
    if (end < 0 || start > end) return [];
    const out: string[] = [];
    for (let i = start; i <= end; i++) {
      const row = options.gridRows.value[i];
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
  const effectiveSelectedRows = computed(() =>
    options.gridRows.value.filter((r) => effectiveSelectedKeySet.value.has(r.pointKey))
  );

  const selectedCount = computed(() => effectiveSelectedKeys.value.length);

  function clearExplicitRowSelection() {
    if (!options.gridRows.value.some((r) => r.__selected)) return;
    options.gridRows.value.forEach((r) => {
      r.__selected = false;
    });
    options.gridRows.value = [...options.gridRows.value];
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

  function gridApi(): any | null {
    const v = options.gridRef.value as any;
    if (!v) return null;
    if (
      typeof v.getSource === "function" ||
      typeof v.scrollToRow === "function" ||
      typeof v.getSelectedRange === "function"
    ) {
      return v;
    }
    if (
      v.$el &&
      (typeof v.$el.getSource === "function" ||
        typeof v.$el.scrollToRow === "function" ||
        typeof v.$el.getSelectedRange === "function")
    ) {
      return v.$el;
    }
    return v.$el ?? v;
  }

  function gridElement(): HTMLElement | null {
    const v = options.gridRef.value as any;
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
      if (!Number.isFinite(rowIndex) || rowIndex < 0 || rowIndex >= options.gridRows.value.length) return;

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
          options.gridRows.value.forEach((r, idx) => {
            r.__selected = idx === rowIndex;
          });
        } else {
          const row = options.gridRows.value[rowIndex];
          if (row) row.__selected = !row.__selected;
        }
        options.gridRows.value = [...options.gridRows.value];
      }

      lastRowSelectionIndex = rowIndex;
      options.clearFocus?.();
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
      if (!options.appendRows) return;
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
      const baseRow = options.gridRows.value[options.gridRows.value.length - 1] ?? null;
      options
        .appendRows(AUTOFILL_APPEND_CHUNK, baseRow)
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

  function detachSelectionListeners() {
    detachGridSelectionListeners?.();
    detachGridSelectionListeners = null;
  }

  async function getSelectedRange(): Promise<GridRangeSelection | null> {
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

  function getRowClass(row: any): string {
    const pointRow = row?.model as T | undefined;
    return pointRow && effectiveSelectedKeySet.value.has(pointRow.pointKey) ? "row-selected" : "";
  }

  return {
    selectedRangeRows,
    effectiveSelectedKeySet,
    effectiveSelectedRows,
    selectedCount,
    onGridSetRange,
    onGridSelectionChange,
    onGridClearRegion,
    gridApi,
    attachGridSelectionListeners,
    detachSelectionListeners,
    getSelectedRange,
    getRowClass,
  };
}
