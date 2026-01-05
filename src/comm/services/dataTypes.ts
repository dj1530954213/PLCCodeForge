import type { DataType, RegisterArea } from "../api";

/**
 * 数据类型信息接口
 * 遵循SRP：只负责数据类型的元数据定义
 */
export interface DataTypeInfo {
  name: DataType;
  displayName: string;
  registerSpan: number;  // 占用寄存器数量
  byteSize: number;      // 字节大小
  category: 'integer' | 'float' | 'boolean';
  signed: boolean;
}

/**
 * 数据类型元数据映射表
 * 使用常量对象确保类型安全和性能
 */
const DATA_TYPE_INFO_MAP: Record<DataType, DataTypeInfo> = {
  Bool: {
    name: "Bool",
    displayName: "布尔型",
    registerSpan: 1,
    byteSize: 1,
    category: 'boolean',
    signed: false,
  },
  Int16: {
    name: "Int16",
    displayName: "16位有符号整数",
    registerSpan: 1,
    byteSize: 2,
    category: 'integer',
    signed: true,
  },
  UInt16: {
    name: "UInt16",
    displayName: "16位无符号整数",
    registerSpan: 1,
    byteSize: 2,
    category: 'integer',
    signed: false,
  },
  Int32: {
    name: "Int32",
    displayName: "32位有符号整数",
    registerSpan: 2,
    byteSize: 4,
    category: 'integer',
    signed: true,
  },
  UInt32: {
    name: "UInt32",
    displayName: "32位无符号整数",
    registerSpan: 2,
    byteSize: 4,
    category: 'integer',
    signed: false,
  },
  Int64: {
    name: "Int64",
    displayName: "64位有符号整数",
    registerSpan: 4,
    byteSize: 8,
    category: 'integer',
    signed: true,
  },
  UInt64: {
    name: "UInt64",
    displayName: "64位无符号整数",
    registerSpan: 4,
    byteSize: 8,
    category: 'integer',
    signed: false,
  },
  Float32: {
    name: "Float32",
    displayName: "32位浮点数",
    registerSpan: 2,
    byteSize: 4,
    category: 'float',
    signed: true,
  },
  Float64: {
    name: "Float64",
    displayName: "64位浮点数",
    registerSpan: 4,
    byteSize: 8,
    category: 'float',
    signed: true,
  },
  Unknown: {
    name: "Unknown",
    displayName: "未知类型",
    registerSpan: 0,
    byteSize: 0,
    category: 'integer',
    signed: false,
  },
};

/**
 * 获取数据类型的完整信息
 * @param dataType 数据类型
 * @returns 数据类型信息对象
 */
export function getDataTypeInfo(dataType: DataType): DataTypeInfo {
  return DATA_TYPE_INFO_MAP[dataType];
}

/**
 * 获取数据类型占用的寄存器数量
 * @param dataType 数据类型
 * @returns 寄存器数量（Bool/Int16/UInt16=1, Int32/UInt32/Float32=2, Int64/UInt64/Float64=4）
 */
export function getRegisterSpan(dataType: DataType): number {
  return DATA_TYPE_INFO_MAP[dataType].registerSpan;
}

/**
 * 判断数据类型是否适用于指定的寄存器区域
 * @param dataType 数据类型
 * @param area 寄存器区域
 * @returns 是否兼容
 */
export function isValidForArea(dataType: DataType, area: RegisterArea): boolean {
  // Holding 和 Input 区域支持所有非布尔类型
  if (area === "Holding" || area === "Input") {
    return dataType !== "Bool" && dataType !== "Unknown";
  }
  
  // Coil 和 Discrete 区域只支持布尔类型
  if (area === "Coil" || area === "Discrete") {
    return dataType === "Bool";
  }
  
  return false;
}

/**
 * 获取指定区域支持的数据类型列表
 * @param area 寄存器区域
 * @returns 支持的数据类型数组
 */
export function getSupportedDataTypes(area: RegisterArea): DataType[] {
  if (area === "Holding" || area === "Input") {
    return ["Int16", "UInt16", "Int32", "UInt32", "Int64", "UInt64", "Float32", "Float64"];
  }
  
  if (area === "Coil" || area === "Discrete") {
    return ["Bool"];
  }
  
  return [];
}

/**
 * 获取数据类型的显示名称
 * @param dataType 数据类型
 * @returns 中文显示名称
 */
export function getDataTypeDisplayName(dataType: DataType): string {
  return DATA_TYPE_INFO_MAP[dataType].displayName;
}
