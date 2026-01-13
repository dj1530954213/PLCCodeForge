import type { ByteOrder32, DataType } from "./api";

export const COMM_DATA_TYPES: DataType[] = [
  "Bool",
  "Int16",
  "UInt16",
  "Int32",
  "UInt32",
  "Int64",
  "UInt64",
  "Float32",
  "Float64",
];

export const COMM_BYTE_ORDERS_32: ByteOrder32[] = ["ABCD", "BADC", "CDAB", "DCBA"];

