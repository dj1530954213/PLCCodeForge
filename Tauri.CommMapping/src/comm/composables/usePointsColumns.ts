import { computed, type ComputedRef, type Ref } from "vue";
import type { ColumnRegular } from "@revolist/vue3-datagrid";

import type { ByteOrder32, DataType } from "../api";
import type { PointRowLike } from "./usePointsRows";

export type FocusedIssueCell<T extends PointRowLike> = {
  pointKey: string;
  field: keyof T;
};

export interface UsePointsColumnsOptions<T extends PointRowLike> {
  dataTypeOptions: ComputedRef<DataType[]>;
  byteOrders: ByteOrder32[];
  showAllValidation: Ref<boolean>;
  touchedRowKeys: Ref<Record<string, boolean>>;
  focusedIssueCell: Ref<FocusedIssueCell<T> | null>;
  hmiDuplicateByPointKey: ComputedRef<Record<string, string>>;
  addressConflictByPointKey: ComputedRef<Record<string, string>>;
  validationIssueByPointKey: ComputedRef<Record<string, string>>;
  validateHmiName: (row: T) => string | null;
  validateScale: (row: T) => string | null;
  validateModbusAddress: (row: T) => string | null;
}

export function usePointsColumns<T extends PointRowLike>(options: UsePointsColumnsOptions<T>) {
  const {
    dataTypeOptions,
    byteOrders,
    showAllValidation,
    touchedRowKeys,
    focusedIssueCell,
    hmiDuplicateByPointKey,
    addressConflictByPointKey,
    validationIssueByPointKey,
    validateHmiName,
    validateScale,
    validateModbusAddress,
  } = options;

  function rowCellProps(field: keyof T) {
    return ({ model }: any) => {
      const row = model as T;
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
      return Object.keys(classes).length > 0
        ? { class: classes, title: err ?? validationIssueByPointKey.value[row.pointKey] }
        : {};
    };
  }

  const columns = computed<ColumnRegular[]>(() => [
    {
      prop: "hmiName",
      name: "变量名称（HMI）*",
      size: 220,
      minSize: 160,
      autoSize: true,
      editor: "comm-text",
      cellProperties: rowCellProps("hmiName"),
    },
    {
      prop: "modbusAddress",
      name: "点位地址（从 1 开始）",
      size: 120,
      minSize: 110,
      editor: "comm-text",
      cellProperties: rowCellProps("modbusAddress"),
    },
    {
      prop: "dataType",
      name: "数据类型",
      size: 110,
      minSize: 100,
      editor: "comm-select",
      editorOptions: dataTypeOptions.value.map((value) => ({ label: value, value })),
    },
    {
      prop: "byteOrder",
      name: "字节序",
      size: 90,
      minSize: 90,
      editor: "comm-select",
      editorOptions: byteOrders.map((value) => ({ label: value, value })),
    },
    { prop: "scale", name: "缩放倍数", size: 90, minSize: 90, editor: "comm-number", cellProperties: rowCellProps("scale") },
    { prop: "quality", name: "质量", size: 90, minSize: 90, readonly: true },
    { prop: "valueDisplay", name: "实时值", size: 160, minSize: 140, autoSize: true, readonly: true },
    { prop: "timestamp", name: "时间戳", size: 180, minSize: 160, readonly: true },
    { prop: "durationMs", name: "耗时(ms)", size: 90, minSize: 90, readonly: true },
    { prop: "errorMessage", name: "错误信息", size: 220, minSize: 180, readonly: true },
  ]);

  const colIndexByProp = computed<Record<string, number>>(() => {
    const out: Record<string, number> = {};
    for (let i = 0; i < columns.value.length; i++) {
      out[String(columns.value[i].prop)] = i;
    }
    return out;
  });

  return {
    columns,
    colIndexByProp,
  };
}
