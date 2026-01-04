//! 通讯地址采集并生成模块：稳定数据模型与 DTO。
//!
//! 约束（来自 Docs/通讯数据采集验证/执行要求.md）：
//! - 持久化 JSON 顶层必须包含 `schemaVersion: 1`
//! - 点位必须包含稳定且不可变的 `pointKey`（运行期主键）
//! - 点位的业务键为 `hmiName`（变量名称/HMI），可编辑但不作为运行期关联键

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION_V1: u32 = 1;

fn default_scale() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataType {
    Bool,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Float32,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ByteOrder32 {
    ABCD,
    BADC,
    CDAB,
    DCBA,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegisterArea {
    Holding,
    Input,
    Coil,
    Discrete,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SerialParity {
    None,
    Even,
    Odd,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Quality {
    Ok,
    Timeout,
    CommError,
    DecodeError,
    ConfigError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "protocolType", rename_all_fields = "camelCase")]
pub enum ConnectionProfile {
    /// `protocolType: "TCP"`
    #[serde(rename = "TCP")]
    Tcp {
        channel_name: String,
        /// TCP: UnitId
        device_id: u8,
        read_area: RegisterArea,
        /// 内部 0-based
        start_address: u16,
        /// 寄存器/线圈数量
        length: u16,
        ip: String,
        port: u16,
        timeout_ms: u32,
        retry_count: u32,
        poll_interval_ms: u32,
    },

    /// `protocolType: "485"`
    #[serde(rename = "485")]
    Rtu485 {
        channel_name: String,
        /// 485: SlaveId
        device_id: u8,
        read_area: RegisterArea,
        /// 内部 0-based
        start_address: u16,
        /// 寄存器/线圈数量
        length: u16,
        serial_port: String,
        baud_rate: u32,
        parity: SerialParity,
        data_bits: u8,
        stop_bits: u8,
        timeout_ms: u32,
        retry_count: u32,
        poll_interval_ms: u32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommPoint {
    pub point_key: Uuid,
    pub hmi_name: String,
    pub data_type: DataType,
    pub byte_order: ByteOrder32,
    pub channel_name: String,
    /// 相对所属 Profile 的 `startAddress`（内部 0-based）的地址偏移（寄存器/线圈单位）。
    ///
    /// - `Some(offset)`：计划构建时使用 `profile.startAddress + offset` 作为点位地址
    /// - `None`：保持兼容旧行为（按 points 顺序从 `profile.startAddress` 自动顺排）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_offset: Option<u16>,
    #[serde(default = "default_scale")]
    pub scale: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SampleResult {
    pub point_key: Uuid,
    pub value_display: String,
    pub quality: Quality,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u32,
    #[serde(default)]
    pub error_message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunStats {
    pub total: u32,
    pub ok: u32,
    pub timeout: u32,
    pub comm_error: u32,
    pub decode_error: u32,
    pub config_error: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesV1 {
    pub schema_version: u32,
    pub profiles: Vec<ConnectionProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointsV1 {
    pub schema_version: u32,
    pub points: Vec<CommPoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommConfigV1 {
    pub schema_version: u32,
    /// 交付目录出口（TASK-32）：XLSX/IR/evidence 的默认输出根目录。
    pub output_dir: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommWarning {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_key: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hmi_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedRows {
    pub tcp: u32,
    pub rtu: u32,
    pub params: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportDiagnostics {
    pub exported_rows: ExportedRows,
    pub duration_ms: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_v1_json_roundtrip_includes_schema_version_and_point_key() {
        let points = PointsV1 {
            schema_version: SCHEMA_VERSION_V1,
            points: vec![CommPoint {
                point_key: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                hmi_name: "TANK_TEMP".to_string(),
                data_type: DataType::Float32,
                byte_order: ByteOrder32::ABCD,
                channel_name: "tcp-1".to_string(),
                address_offset: None,
                scale: 1.0,
            }],
        };

        let json = serde_json::to_string_pretty(&points).unwrap();
        assert!(json.contains("\"schemaVersion\": 1"));
        assert!(json.contains("\"pointKey\": \"00000000-0000-0000-0000-000000000001\""));
        assert!(json.contains("\"hmiName\": \"TANK_TEMP\""));
        assert!(!json.contains("hmi_name"));

        let decoded: PointsV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, points);
    }

    #[test]
    fn profiles_v1_json_roundtrip() {
        let profiles = ProfilesV1 {
            schema_version: SCHEMA_VERSION_V1,
            profiles: vec![ConnectionProfile::Tcp {
                channel_name: "tcp-1".to_string(),
                device_id: 1,
                read_area: RegisterArea::Holding,
                start_address: 0,
                length: 20,
                ip: "127.0.0.1".to_string(),
                port: 502,
                timeout_ms: 1000,
                retry_count: 2,
                poll_interval_ms: 500,
            }],
        };

        let json = serde_json::to_string_pretty(&profiles).unwrap();
        assert!(json.contains("\"schemaVersion\": 1"));
        assert!(json.contains("\"protocolType\": \"TCP\""));
        assert!(json.contains("\"channelName\": \"tcp-1\""));
        assert!(!json.contains("channel_name"));

        let decoded: ProfilesV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, profiles);
    }
}
