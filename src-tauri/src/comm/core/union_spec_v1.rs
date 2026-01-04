//! 联合 xlsx 输入规范（冻结 v1）：代码侧单一真源常量。

use serde::{Deserialize, Serialize};

pub const SPEC_VERSION_V1: &str = "v1";

/// 联合 xlsx（冻结 v1）默认目标 sheet 名。
pub const DEFAULT_SHEET_V1: &str = "联合点表";

/// 冻结 v1：允许的 sheet 名清单（strict=true 时用于校验/提示）。
pub const REQUIRED_SHEETS_V1: [&str; 1] = [DEFAULT_SHEET_V1];

/// 冻结 v1：必填列（逐字匹配；实现会对表头做 trim() 后比对）。
pub const REQUIRED_COLUMNS_V1: [&str; 6] = [
    "变量名称（HMI）",
    "数据类型",
    "字节序",
    "通道名称",
    "协议类型",
    "设备标识",
];

/// 冻结 v1：可选列（存在则解析；不存在不影响导入）。
pub const OPTIONAL_COLUMNS_V1: [&str; 14] = [
    "起始地址",
    "长度",
    "缩放倍数",
    "读取区域",
    "TCP:IP",
    "TCP:端口",
    "485:串口",
    "485:波特率",
    "485:校验",
    "485:数据位",
    "485:停止位",
    "超时ms",
    "重试次数",
    "轮询周期ms",
];

pub const ALLOWED_PROTOCOLS_V1: [&str; 2] = ["TCP", "485"];
pub const ALLOWED_DATATYPES_V1: [&str; 6] =
    ["Bool", "Int16", "UInt16", "Int32", "UInt32", "Float32"];
pub const ALLOWED_BYTEORDERS_V1: [&str; 4] = ["ABCD", "BADC", "CDAB", "DCBA"];
pub const ALLOWED_READ_AREAS_V1: [&str; 4] = ["Holding", "Input", "Coil", "Discrete"];

/// 严格模式下，当 `addressBase=one` 时，起始地址必须满足的最小值提示。
pub const ALLOWED_START_ADDRESS_ONE_BASED_MIN: &str = ">= 1 (when addressBase=one)";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AddressBase {
    Zero,
    One,
}

impl Default for AddressBase {
    fn default() -> Self {
        // v1 规范默认 1-based 输入（导入时转换为内部 0-based）。
        Self::One
    }
}

pub const DEFAULT_ADDRESS_BASE_V1: AddressBase = AddressBase::One;

pub fn normalize_header_loose(s: &str) -> String {
    s.trim()
        .replace(' ', "")
        .replace('\u{3000}', "")
        .to_lowercase()
}

pub fn normalize_token_loose(value: &str) -> String {
    to_halfwidth_ascii(value).trim().to_string()
}

pub fn to_halfwidth_ascii(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            // 全角空格
            '\u{3000}' => ' ',
            // 全角 ASCII（！到～）
            '\u{FF01}'..='\u{FF5E}' => {
                let code = (c as u32).saturating_sub(0xFEE0);
                char::from_u32(code).unwrap_or(c)
            }
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_v1_required_columns_snapshot() {
        assert_eq!(
            REQUIRED_COLUMNS_V1,
            [
                "变量名称（HMI）",
                "数据类型",
                "字节序",
                "通道名称",
                "协议类型",
                "设备标识",
            ]
        );
        assert_eq!(REQUIRED_COLUMNS_V1.len(), 6);
    }

    #[test]
    fn spec_v1_allowed_enums_snapshot() {
        assert_eq!(ALLOWED_PROTOCOLS_V1, ["TCP", "485"]);
        assert_eq!(
            ALLOWED_DATATYPES_V1,
            ["Bool", "Int16", "UInt16", "Int32", "UInt32", "Float32"]
        );
        assert_eq!(ALLOWED_BYTEORDERS_V1, ["ABCD", "BADC", "CDAB", "DCBA"]);
    }
}
