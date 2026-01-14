import type { DataType, RegisterArea } from "../api";
import { formatHumanAddressFrom0Based, parseHumanAddress, spanForArea } from "./address";

export type SelectionRange = {
  rowStart: number;
  rowEnd: number;
  colStart: number;
  colEnd: number;
};

export type CellEdit = {
  rowIndex: number;
  colIndex: number;
  prop: string;
  value: unknown;
};

export function computeFillDownEdits<T extends Record<string, unknown>>(params: {
  rows: readonly T[];
  propsByColIndex: readonly string[];
  range: SelectionRange;
  skipProps?: ReadonlySet<string>;
}): { edits: CellEdit[]; changed: number } {
  const { rows, propsByColIndex, range, skipProps } = params;
  const base = rows[range.rowStart] as Record<string, unknown> | undefined;
  if (!base) return { edits: [], changed: 0 };

  const edits: CellEdit[] = [];
  let changed = 0;
  for (let r = range.rowStart + 1; r <= range.rowEnd; r++) {
    for (let c = range.colStart; c <= range.colEnd; c++) {
      const prop = String(propsByColIndex[c] ?? "");
      if (!prop) continue;
      if (skipProps?.has(prop)) continue;
      const value = base[prop];
      edits.push({ rowIndex: r, colIndex: c, prop, value });
      changed += 1;
    }
  }
  return { edits, changed };
}

export function computeFillAddressEdits(params: {
  rows: ReadonlyArray<{ modbusAddress: string; dataType: DataType }>;
  range: Pick<SelectionRange, "rowStart" | "rowEnd">;
  readArea: RegisterArea;
  addressProp?: string;
}):
  | { ok: true; edits: Array<{ rowIndex: number; value: string }>; changed: number }
  | { ok: false; message: string } {
  const { rows, range } = params;
  const row0 = rows[range.rowStart];
  if (!row0) return { ok: false, message: "选区为空" };

  const startRaw = String(row0.modbusAddress ?? "").trim();
  if (!startRaw) return { ok: false, message: "填充地址需要选区第一行先填写起始地址" };

  const parsed = parseHumanAddress(startRaw, params.readArea);
  if (!parsed.ok) return parsed;
  const area = parsed.area;

  let curStart0 = parsed.start0Based;
  const edits: Array<{ rowIndex: number; value: string }> = [];

  for (let r = range.rowStart; r <= range.rowEnd; r++) {
    if (r > range.rowStart) {
      const prev = rows[r - 1];
      if (!prev) break;
      const prevSpan = spanForArea(area, prev.dataType);
      if (prevSpan === null) {
        return { ok: false, message: `数据类型 ${prev.dataType} 与读取区域 ${area} 不匹配（行 ${r}）` };
      }
      curStart0 += prevSpan;
    }

    edits.push({ rowIndex: r, value: formatHumanAddressFrom0Based(area, curStart0) });
  }

  return { ok: true, edits, changed: edits.length };
}
