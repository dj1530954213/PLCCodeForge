//! 联合 xlsx 解析辅助（用于 UnifiedImport / evidence 可追溯）（TASK-39）
//!
//! 注意：
//! - 本模块不替代 `import_union_xlsx.rs` 的严格/宽松导入逻辑；
//!   仅提供“列存在性/使用清单/缺列告警/最小 deviceGroups/hardware 组装”等能力。
//! - 任何缺字段不得 panic：通过 warnings（MISSING_COLUMN）可观测。

use std::collections::{BTreeMap, HashMap};

use serde_json::{json, Value as JsonValue};

use super::model::{CommWarning, ConnectionProfile, PointsV1, ProfilesV1};
use super::union_spec_v1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionXlsxColumnsInfo {
    pub detected_columns: Vec<String>,
    pub parsed_columns_used: Vec<String>,
    pub missing_optional_columns: Vec<String>,
}

pub fn columns_info_from_detected(detected_columns: &[String]) -> UnionXlsxColumnsInfo {
    let set: std::collections::HashSet<&str> = detected_columns.iter().map(|s| s.as_str()).collect();

    let mut parsed: Vec<String> = Vec::new();
    for c in union_spec_v1::REQUIRED_COLUMNS_V1 {
        if set.contains(c) {
            parsed.push(c.to_string());
        }
    }
    for c in union_spec_v1::OPTIONAL_COLUMNS_V1 {
        if set.contains(c) {
            parsed.push(c.to_string());
        }
    }

    let mut missing_optional: Vec<String> = Vec::new();
    for c in union_spec_v1::OPTIONAL_COLUMNS_V1 {
        if !set.contains(c) {
            missing_optional.push(c.to_string());
        }
    }

    UnionXlsxColumnsInfo {
        detected_columns: detected_columns.to_vec(),
        parsed_columns_used: parsed,
        missing_optional_columns: missing_optional,
    }
}

pub fn missing_columns_warnings(info: &UnionXlsxColumnsInfo) -> Vec<CommWarning> {
    info.missing_optional_columns
        .iter()
        .map(|c| CommWarning {
            code: "MISSING_COLUMN".to_string(),
            message: format!("union xlsx missing column '{c}' (values may be defaulted)"),
            point_key: None,
            hmi_name: None,
        })
        .collect()
}

pub fn build_device_groups(points: &PointsV1, profiles: &ProfilesV1) -> Vec<JsonValue> {
    let mut points_by_channel: HashMap<String, Vec<String>> = HashMap::new();
    for p in &points.points {
        points_by_channel
            .entry(p.channel_name.clone())
            .or_default()
            .push(p.hmi_name.clone());
    }

    let mut groups: Vec<JsonValue> = Vec::new();
    for profile in &profiles.profiles {
        let (protocol_type, channel_name, device_id, read_area, start_address, length, conn) = match profile {
            ConnectionProfile::Tcp {
                channel_name,
                device_id,
                read_area,
                start_address,
                length,
                ip,
                port,
                timeout_ms,
                retry_count,
                poll_interval_ms,
            } => (
                "TCP",
                channel_name.clone(),
                *device_id,
                format!("{read_area:?}"),
                *start_address,
                *length,
                json!({
                    "tcp": { "ip": ip, "port": port },
                    "timeoutMs": timeout_ms,
                    "retryCount": retry_count,
                    "pollIntervalMs": poll_interval_ms,
                }),
            ),
            ConnectionProfile::Rtu485 {
                channel_name,
                device_id,
                read_area,
                start_address,
                length,
                serial_port,
                baud_rate,
                parity,
                data_bits,
                stop_bits,
                timeout_ms,
                retry_count,
                poll_interval_ms,
            } => (
                "485",
                channel_name.clone(),
                *device_id,
                format!("{read_area:?}"),
                *start_address,
                *length,
                json!({
                    "rtu485": {
                        "serialPort": serial_port,
                        "baudRate": baud_rate,
                        "parity": format!("{parity:?}"),
                        "dataBits": data_bits,
                        "stopBits": stop_bits,
                    },
                    "timeoutMs": timeout_ms,
                    "retryCount": retry_count,
                    "pollIntervalMs": poll_interval_ms,
                }),
            ),
        };

        let points_list = points_by_channel
            .get(channel_name.as_str())
            .cloned()
            .unwrap_or_default();

        groups.push(json!({
            "protocolType": protocol_type,
            "channelName": channel_name,
            "deviceId": device_id,
            "readArea": read_area,
            "startAddress": start_address,
            "length": length,
            "points": points_list,
            "connection": conn,
        }));
    }

    groups
}

pub fn build_hardware_snapshot(
    profiles: &ProfilesV1,
    columns: &UnionXlsxColumnsInfo,
) -> JsonValue {
    // MVP：当前模块仅能从联合表中解析“通讯相关字段”；
    // deviceGroups/hardware 结构为“可扩展的通用 JSON”，便于 UIA/PLC Generator 后续合并消费。
    let mut protocols: BTreeMap<String, u32> = BTreeMap::new();
    for p in &profiles.profiles {
        let proto = match p {
            ConnectionProfile::Tcp { .. } => "TCP",
            ConnectionProfile::Rtu485 { .. } => "485",
        };
        *protocols.entry(proto.to_string()).or_insert(0) += 1;
    }

    json!({
        "comm": {
            "profilesCount": profiles.profiles.len(),
            "profilesByProtocol": protocols,
        },
        "unionXlsx": {
            "parsedColumnsUsed": columns.parsed_columns_used.clone(),
            "missingOptionalColumns": columns.missing_optional_columns.clone(),
        }
    })
}
