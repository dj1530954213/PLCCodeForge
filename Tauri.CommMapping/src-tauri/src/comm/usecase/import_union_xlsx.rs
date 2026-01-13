//! “联合 xlsx（IO+设备表）→ CommPoint/Profiles” 映射层（TASK-17）。
//!
//! 目标：
//! - 读取一份联合 xlsx（只取第一张 sheet）
//! - 从表格中提取通讯相关字段，生成 `PointsV1 + ProfilesV1`
//! - pointKey 采用确定性算法（UUID v5 / SHA1），禁止随机
//! - 对缺失/冲突/无法识别的字段输出 warnings（不 panic）

use std::collections::{HashMap, HashSet};
use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::comm::core::model::{
    ByteOrder32, CommPoint, CommWarning, ConnectionProfile, DataType, PointsV1, ProfilesV1,
    RegisterArea, SerialParity, SCHEMA_VERSION_V1,
};
use crate::comm::core::union_spec_v1 as spec_v1;
use crate::comm::error::{ImportUnionError, ImportUnionErrorDetails, ImportUnionErrorKind};

pub use spec_v1::AddressBase;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportUnionOptions {
    /// strict=true 时对 sheet/列名/枚举值进行硬失败校验；默认 false（保持宽松导入）。
    #[serde(default)]
    pub strict: Option<bool>,
    /// 指定读取的 sheet；strict=true 找不到则失败。
    #[serde(default)]
    pub sheet_name: Option<String>,
    /// 起始地址基准：zero/one；默认按规范 v1。
    #[serde(default)]
    pub address_base: Option<AddressBase>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportUnionDiagnostics {
    pub detected_sheets: Vec<String>,
    pub detected_columns: Vec<String>,
    pub used_sheet: String,
    pub strict: bool,
    pub address_base_used: AddressBase,
    pub rows_scanned: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_columns: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_protocols: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_datatypes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_byte_orders: Option<Vec<String>>,
}

#[derive(Debug, Error)]
pub enum ImportUnionXlsxError {
    #[error("failed to open workbook: {0}")]
    OpenWorkbook(String),

    #[error("workbook has no worksheet")]
    NoWorksheet,

    #[error("worksheet has no header row")]
    NoHeaderRow,

    #[error("strict: sheet not found: {sheet_name}")]
    MissingSheet {
        sheet_name: String,
        detected_sheets: Vec<String>,
        diagnostics: ImportUnionDiagnostics,
    },

    #[error("strict: missing required columns: {missing_columns:?}")]
    MissingRequiredColumns {
        missing_columns: Vec<String>,
        detected_columns: Vec<String>,
        diagnostics: ImportUnionDiagnostics,
    },

    #[error("strict: invalid value at row {row_index} column {column_name}: {raw_value}")]
    InvalidRequiredValue {
        row_index: u32,
        column_name: String,
        raw_value: String,
        allowed_values: Vec<String>,
        diagnostics: ImportUnionDiagnostics,
    },
}

impl ImportUnionXlsxError {
    pub fn diagnostics(&self) -> Option<&ImportUnionDiagnostics> {
        match self {
            ImportUnionXlsxError::MissingSheet { diagnostics, .. } => Some(diagnostics),
            ImportUnionXlsxError::MissingRequiredColumns { diagnostics, .. } => Some(diagnostics),
            ImportUnionXlsxError::InvalidRequiredValue { diagnostics, .. } => Some(diagnostics),
            _ => None,
        }
    }

    pub fn to_import_error(&self) -> ImportUnionError {
        match self {
            ImportUnionXlsxError::OpenWorkbook(message) => ImportUnionError {
                kind: ImportUnionErrorKind::UnionXlsxReadError,
                message: format!("failed to open workbook: {message}"),
                details: None,
            },
            ImportUnionXlsxError::NoWorksheet => ImportUnionError {
                kind: ImportUnionErrorKind::UnionXlsxReadError,
                message: "workbook has no worksheet".to_string(),
                details: None,
            },
            ImportUnionXlsxError::NoHeaderRow => ImportUnionError {
                kind: ImportUnionErrorKind::UnionXlsxReadError,
                message: "worksheet has no header row".to_string(),
                details: None,
            },
            ImportUnionXlsxError::MissingSheet {
                sheet_name,
                detected_sheets,
                diagnostics,
            } => ImportUnionError {
                kind: ImportUnionErrorKind::UnionXlsxInvalidSheet,
                message: format!(
                    "strict: sheet not found: '{sheet_name}', available: {detected_sheets:?}"
                ),
                details: Some(ImportUnionErrorDetails {
                    sheet_name: Some(sheet_name.clone()),
                    detected_sheets: Some(detected_sheets.clone()),
                    address_base_used: Some(diagnostics.address_base_used),
                    ..Default::default()
                }),
            },
            ImportUnionXlsxError::MissingRequiredColumns {
                missing_columns,
                detected_columns,
                diagnostics,
            } => ImportUnionError {
                kind: ImportUnionErrorKind::UnionXlsxMissingColumns,
                message: format!(
                    "strict: missing required columns: {missing_columns:?}, detected: {detected_columns:?}"
                ),
                details: Some(ImportUnionErrorDetails {
                    missing_columns: Some(missing_columns.clone()),
                    detected_columns: Some(detected_columns.clone()),
                    address_base_used: Some(diagnostics.address_base_used),
                    ..Default::default()
                }),
            },
            ImportUnionXlsxError::InvalidRequiredValue {
                row_index,
                column_name,
                raw_value,
                allowed_values,
                diagnostics,
            } => {
                let kind = match column_name.as_str() {
                    v if v == spec_v1::REQUIRED_COLUMNS_V1[4]
                        || v == spec_v1::REQUIRED_COLUMNS_V1[1]
                        || v == spec_v1::REQUIRED_COLUMNS_V1[2]
                        || v == spec_v1::OPTIONAL_COLUMNS_V1[3] =>
                    {
                        ImportUnionErrorKind::UnionXlsxInvalidEnum
                    }
                    _ => ImportUnionErrorKind::UnionXlsxInvalidRow,
                };

                ImportUnionError {
                    kind,
                    message: format!(
                        "strict: invalid value at row {row_index} column '{column_name}': '{raw_value}', allowed: {allowed_values:?}"
                    ),
                    details: Some(ImportUnionErrorDetails {
                        row_index: Some(*row_index),
                        column_name: Some(column_name.clone()),
                        raw_value: Some(raw_value.clone()),
                        allowed_values: Some(allowed_values.clone()),
                        address_base_used: Some(diagnostics.address_base_used),
                        ..Default::default()
                    }),
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportUnionXlsxOutcome {
    pub points: PointsV1,
    pub profiles: ProfilesV1,
    pub warnings: Vec<CommWarning>,
    pub diagnostics: ImportUnionDiagnostics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ProtocolKind {
    Tcp,
    Rtu485,
}

#[derive(Clone, Debug)]
struct RowRecord {
    row_index: usize,
    hmi_name: String,
    base_channel_name: String,
    final_channel_name: String,
    protocol: ProtocolKind,
    device_id: u8,
    read_area: RegisterArea,
    point_start_address: Option<u16>,
    /// profile 范围长度（寄存器/线圈数量），0 表示“未提供，需要推导/默认”。
    profile_length: u16,
    timeout_ms: u32,
    retry_count: u32,
    poll_interval_ms: u32,
    data_type: DataType,
    byte_order: ByteOrder32,
    address_offset: Option<u16>,
    scale: f64,
    tcp_ip: String,
    tcp_port: u16,
    rtu_serial_port: String,
    rtu_baud_rate: u32,
    rtu_parity: SerialParity,
    rtu_data_bits: u8,
    rtu_stop_bits: u8,
}

const POINTKEY_NAMESPACE: Uuid = Uuid::from_u128(0x9a1d_6e2a_0d92_4a1e_9f2f_4a7d_7a8b_1234);

fn stable_point_key(hmi_name: &str, base_channel_name: &str, device_id: u8) -> Uuid {
    let name = format!("{hmi_name}|{base_channel_name}|{device_id}");
    Uuid::new_v5(&POINTKEY_NAMESPACE, name.as_bytes())
}

fn profile_channel_name(profile: &ConnectionProfile) -> &str {
    match profile {
        ConnectionProfile::Tcp { channel_name, .. } => channel_name.as_str(),
        ConnectionProfile::Rtu485 { channel_name, .. } => channel_name.as_str(),
    }
}

fn cell_string(cell: &Data) -> Option<String> {
    match cell {
        Data::Empty => None,
        Data::String(s) => Some(s.trim().to_string()),
        Data::Float(v) => Some(format!("{v}")),
        Data::Int(v) => Some(format!("{v}")),
        Data::Bool(v) => Some(if *v { "1".to_string() } else { "0".to_string() }),
        other => Some(format!("{other:?}")),
    }
}

fn cell_u32(cell: &Data) -> Option<u32> {
    match cell {
        Data::Int(v) => (*v).try_into().ok(),
        Data::Float(v) => (*v).round().to_string().parse().ok(),
        Data::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn cell_u16(cell: &Data) -> Option<u16> {
    cell_u32(cell)?.try_into().ok()
}

fn cell_u8(cell: &Data) -> Option<u8> {
    cell_u32(cell)?.try_into().ok()
}

fn cell_f64(cell: &Data) -> Option<f64> {
    match cell {
        Data::Float(v) => Some(*v),
        Data::Int(v) => Some(*v as f64),
        Data::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn parse_protocol_loose(value: &str) -> Option<ProtocolKind> {
    let v = spec_v1::normalize_token_loose(value).to_uppercase();
    match v.as_str() {
        "TCP" => Some(ProtocolKind::Tcp),
        "485" | "RTU" | "RTU485" => Some(ProtocolKind::Rtu485),
        _ => None,
    }
}

fn parse_protocol_strict(value: &str) -> Option<ProtocolKind> {
    let v = spec_v1::normalize_token_loose(value).to_uppercase();
    match v.as_str() {
        "TCP" => Some(ProtocolKind::Tcp),
        "485" => Some(ProtocolKind::Rtu485),
        _ => None,
    }
}

fn parse_data_type_loose(value: &str) -> Option<DataType> {
    let v = spec_v1::normalize_token_loose(value).to_uppercase();
    match v.as_str() {
        "BOOL" => Some(DataType::Bool),
        "INT16" | "I16" => Some(DataType::Int16),
        "UINT16" | "U16" => Some(DataType::UInt16),
        "INT32" | "I32" => Some(DataType::Int32),
        "UINT32" | "U32" => Some(DataType::UInt32),
        "FLOAT32" | "F32" | "FLOAT" => Some(DataType::Float32),
        _ => None,
    }
}

fn parse_data_type_strict(value: &str) -> Option<DataType> {
    let v = spec_v1::normalize_token_loose(value).to_uppercase();
    match v.as_str() {
        "BOOL" => Some(DataType::Bool),
        "INT16" => Some(DataType::Int16),
        "UINT16" => Some(DataType::UInt16),
        "INT32" => Some(DataType::Int32),
        "UINT32" => Some(DataType::UInt32),
        "FLOAT32" => Some(DataType::Float32),
        _ => None,
    }
}

fn parse_byte_order_loose(value: &str) -> Option<ByteOrder32> {
    let v = spec_v1::normalize_token_loose(value).to_uppercase();
    match v.as_str() {
        "ABCD" => Some(ByteOrder32::ABCD),
        "BADC" => Some(ByteOrder32::BADC),
        "CDAB" => Some(ByteOrder32::CDAB),
        "DCBA" => Some(ByteOrder32::DCBA),
        _ => None,
    }
}

fn parse_byte_order_strict(value: &str) -> Option<ByteOrder32> {
    let v = spec_v1::normalize_token_loose(value).to_uppercase();
    match v.as_str() {
        "ABCD" => Some(ByteOrder32::ABCD),
        "BADC" => Some(ByteOrder32::BADC),
        "CDAB" => Some(ByteOrder32::CDAB),
        "DCBA" => Some(ByteOrder32::DCBA),
        _ => None,
    }
}

fn parse_read_area(value: &str) -> Option<RegisterArea> {
    let v = spec_v1::normalize_token_loose(value).to_uppercase();
    match v.as_str() {
        "HOLDING" => Some(RegisterArea::Holding),
        "INPUT" => Some(RegisterArea::Input),
        "COIL" => Some(RegisterArea::Coil),
        "DISCRETE" => Some(RegisterArea::Discrete),
        _ => None,
    }
}

fn parse_parity(value: &str) -> Option<SerialParity> {
    match spec_v1::normalize_token_loose(value)
        .to_uppercase()
        .as_str()
    {
        "NONE" => Some(SerialParity::None),
        "EVEN" => Some(SerialParity::Even),
        "ODD" => Some(SerialParity::Odd),
        _ => None,
    }
}

fn header_index(headers: &[String], candidates: &[&str]) -> Option<usize> {
    for cand in candidates {
        let cand_norm = spec_v1::normalize_header_loose(cand);
        if let Some(idx) = headers
            .iter()
            .position(|h| spec_v1::normalize_header_loose(h) == cand_norm)
        {
            return Some(idx);
        }
    }
    None
}

pub fn import_union_xlsx(path: &Path) -> Result<ImportUnionXlsxOutcome, ImportUnionXlsxError> {
    import_union_xlsx_with_options(path, None)
}

pub fn import_union_xlsx_with_options(
    path: &Path,
    options: Option<ImportUnionOptions>,
) -> Result<ImportUnionXlsxOutcome, ImportUnionXlsxError> {
    let options = options.unwrap_or_default();
    let strict = options.strict.unwrap_or(false);
    let address_base_used = options.address_base.unwrap_or_default();
    let requested_sheet = options
        .sheet_name
        .clone()
        .unwrap_or_else(|| spec_v1::DEFAULT_SHEET_V1.to_string());

    let mut workbook =
        open_workbook_auto(path).map_err(|e| ImportUnionXlsxError::OpenWorkbook(e.to_string()))?;

    let detected_sheets = workbook.sheet_names().to_owned();
    if detected_sheets.is_empty() {
        return Err(ImportUnionXlsxError::NoWorksheet);
    }

    let mut warnings: Vec<CommWarning> = Vec::new();

    let used_sheet = if detected_sheets.iter().any(|s| s == &requested_sheet) {
        requested_sheet.clone()
    } else if strict {
        return Err(ImportUnionXlsxError::MissingSheet {
            sheet_name: requested_sheet.clone(),
            detected_sheets: detected_sheets.clone(),
            diagnostics: ImportUnionDiagnostics {
                detected_sheets,
                detected_columns: Vec::new(),
                used_sheet: requested_sheet,
                strict,
                address_base_used,
                rows_scanned: 0,
                spec_version: Some(spec_v1::SPEC_VERSION_V1.to_string()),
                required_columns: Some(
                    spec_v1::REQUIRED_COLUMNS_V1
                        .iter()
                        .map(|v| v.to_string())
                        .collect(),
                ),
                allowed_protocols: Some(
                    spec_v1::ALLOWED_PROTOCOLS_V1
                        .iter()
                        .map(|v| v.to_string())
                        .collect(),
                ),
                allowed_datatypes: Some(
                    spec_v1::ALLOWED_DATATYPES_V1
                        .iter()
                        .map(|v| v.to_string())
                        .collect(),
                ),
                allowed_byte_orders: Some(
                    spec_v1::ALLOWED_BYTEORDERS_V1
                        .iter()
                        .map(|v| v.to_string())
                        .collect(),
                ),
            },
        });
    } else {
        let fallback = detected_sheets[0].clone();
        warnings.push(CommWarning {
            code: "SHEET_NOT_FOUND_FALLBACK_FIRST".to_string(),
            message: format!(
                "requested sheet '{}' not found; fallback to first sheet '{}'",
                requested_sheet, fallback
            ),
            point_key: None,
            hmi_name: None,
        });
        fallback
    };

    let range = workbook
        .worksheet_range(&used_sheet)
        .map_err(|e| ImportUnionXlsxError::OpenWorkbook(e.to_string()))?;

    let mut rows = range.rows();
    let header_row = rows.next().ok_or(ImportUnionXlsxError::NoHeaderRow)?;
    let detected_columns: Vec<String> = header_row
        .iter()
        .map(|c| cell_string(c).unwrap_or_default())
        .collect();

    let headers: Vec<String> = detected_columns
        .iter()
        .map(|s| s.trim().to_string())
        .collect();

    // 冻结 v1：统一的列名真源（避免散落字符串导致“文档/实现漂移”）。
    let col_hmi = spec_v1::REQUIRED_COLUMNS_V1[0];
    let col_data_type = spec_v1::REQUIRED_COLUMNS_V1[1];
    let col_byte_order = spec_v1::REQUIRED_COLUMNS_V1[2];
    let col_channel = spec_v1::REQUIRED_COLUMNS_V1[3];
    let col_protocol = spec_v1::REQUIRED_COLUMNS_V1[4];
    let col_device_id = spec_v1::REQUIRED_COLUMNS_V1[5];
    let col_point_start = spec_v1::OPTIONAL_COLUMNS_V1[0];
    let col_length = spec_v1::OPTIONAL_COLUMNS_V1[1];
    let col_read_area = spec_v1::OPTIONAL_COLUMNS_V1[3];

    // strict=true：必填列逐字匹配（冻结 v1）；strict=false：宽松匹配（兼容历史候选列名）。
    let (idx_hmi, idx_data_type, idx_byte_order, idx_channel, idx_protocol, idx_device_id) =
        if strict {
            let mut map: HashMap<&str, usize> = HashMap::new();
            for (i, h) in headers.iter().enumerate() {
                if !h.is_empty() {
                    map.insert(h.as_str(), i);
                }
            }

            let mut missing: Vec<String> = Vec::new();
            for required in spec_v1::REQUIRED_COLUMNS_V1 {
                if !map.contains_key(required) {
                    missing.push(required.to_string());
                }
            }

            if !missing.is_empty() {
                return Err(ImportUnionXlsxError::MissingRequiredColumns {
                    missing_columns: missing,
                    detected_columns: headers.clone(),
                    diagnostics: ImportUnionDiagnostics {
                        detected_sheets,
                        detected_columns: headers.clone(),
                        used_sheet,
                        strict,
                        address_base_used,
                        rows_scanned: 0,
                        spec_version: Some(spec_v1::SPEC_VERSION_V1.to_string()),
                        required_columns: Some(
                            spec_v1::REQUIRED_COLUMNS_V1
                                .iter()
                                .map(|v| v.to_string())
                                .collect(),
                        ),
                        allowed_protocols: Some(
                            spec_v1::ALLOWED_PROTOCOLS_V1
                                .iter()
                                .map(|v| v.to_string())
                                .collect(),
                        ),
                        allowed_datatypes: Some(
                            spec_v1::ALLOWED_DATATYPES_V1
                                .iter()
                                .map(|v| v.to_string())
                                .collect(),
                        ),
                        allowed_byte_orders: Some(
                            spec_v1::ALLOWED_BYTEORDERS_V1
                                .iter()
                                .map(|v| v.to_string())
                                .collect(),
                        ),
                    },
                });
            }

            (
                Some(*map.get(col_hmi).unwrap()),
                Some(*map.get(col_data_type).unwrap()),
                Some(*map.get(col_byte_order).unwrap()),
                Some(*map.get(col_channel).unwrap()),
                Some(*map.get(col_protocol).unwrap()),
                Some(*map.get(col_device_id).unwrap()),
            )
        } else {
            (
                header_index(&headers, &["变量名称（HMI）", "hminame", "变量名", "hmi"]),
                header_index(&headers, &["数据类型", "datatype"]),
                header_index(&headers, &["字节序", "byteorder"]),
                header_index(
                    &headers,
                    &[
                        "通道名称",
                        "起始TCP通道名称",
                        "起始485通道名称",
                        "channelname",
                    ],
                ),
                header_index(&headers, &["协议类型", "protocoltype"]),
                header_index(
                    &headers,
                    &["设备标识", "站号", "deviceid", "unitid", "slaveid"],
                ),
            )
        };

    let idx_scale = header_index(&headers, &["缩放倍数", "scale"]);
    let idx_read_area = header_index(&headers, &["读取区域", "readarea"]);
    let idx_point_start = header_index(&headers, &["起始地址", "startaddress"]);
    let idx_profile_len = header_index(&headers, &["长度", "length", "len"]);
    let idx_timeout = header_index(&headers, &["超时ms", "timeoutms"]);
    let idx_retry = header_index(&headers, &["重试次数", "retrycount"]);
    let idx_poll = header_index(&headers, &["轮询周期ms", "pollintervalms"]);

    // v1 推荐的分列（strict=true 应使用这些列名）；同时兼容旧版“合并列名”。
    let idx_tcp_ip = header_index(&headers, &["TCP:IP"]);
    let idx_tcp_port = header_index(&headers, &["TCP:端口"]);
    let idx_rtu_port = header_index(&headers, &["485:串口"]);
    let idx_rtu_baud = header_index(&headers, &["485:波特率"]);
    let idx_ip_or_serial =
        header_index(&headers, &["TCP:IP / 485:串口", "ip", "串口", "serialport"]);
    let idx_port_or_baud = header_index(&headers, &["TCP:端口 / 485:波特率", "port", "baudrate"]);

    let idx_parity = header_index(&headers, &["485:校验", "parity"]);
    let idx_data_bits = header_index(&headers, &["485:数据位", "databits"]);
    let idx_stop_bits = header_index(&headers, &["485:停止位", "stopbits"]);
    let idx_point_offset = header_index(&headers, &["地址偏移", "offset", "addressoffset"]);

    let mut rows_scanned: u32 = 0;
    let make_diagnostics = |rows_scanned: u32| ImportUnionDiagnostics {
        detected_sheets: detected_sheets.clone(),
        detected_columns: headers.clone(),
        used_sheet: used_sheet.clone(),
        strict,
        address_base_used,
        rows_scanned,
        spec_version: Some(spec_v1::SPEC_VERSION_V1.to_string()),
        required_columns: Some(
            spec_v1::REQUIRED_COLUMNS_V1
                .iter()
                .map(|v| v.to_string())
                .collect(),
        ),
        allowed_protocols: Some(
            spec_v1::ALLOWED_PROTOCOLS_V1
                .iter()
                .map(|v| v.to_string())
                .collect(),
        ),
        allowed_datatypes: Some(
            spec_v1::ALLOWED_DATATYPES_V1
                .iter()
                .map(|v| v.to_string())
                .collect(),
        ),
        allowed_byte_orders: Some(
            spec_v1::ALLOWED_BYTEORDERS_V1
                .iter()
                .map(|v| v.to_string())
                .collect(),
        ),
    };
    let mut raw_records: Vec<(usize, String, String, ProtocolKind, u8, RowRecord)> = Vec::new();

    for (row_idx, row) in rows.enumerate() {
        let row_index = row_idx + 2; // 1-based excel row index (header is row 1)
        rows_scanned = rows_scanned.saturating_add(1);

        let hmi_name = idx_hmi
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .unwrap_or_default();
        if hmi_name.trim().is_empty() {
            if strict {
                return Err(ImportUnionXlsxError::InvalidRequiredValue {
                    row_index: row_index as u32,
                    column_name: col_hmi.to_string(),
                    raw_value: hmi_name,
                    allowed_values: vec!["non-empty".to_string()],
                    diagnostics: make_diagnostics(rows_scanned),
                });
            }

            warnings.push(CommWarning {
                code: "ROW_MISSING_HMI_NAME".to_string(),
                message: format!("row {row_index}: missing hmiName; skipped"),
                point_key: None,
                hmi_name: None,
            });
            continue;
        }

        let base_channel_name = idx_channel
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .unwrap_or_default();
        if base_channel_name.trim().is_empty() {
            if strict {
                return Err(ImportUnionXlsxError::InvalidRequiredValue {
                    row_index: row_index as u32,
                    column_name: col_channel.to_string(),
                    raw_value: base_channel_name,
                    allowed_values: vec!["non-empty".to_string()],
                    diagnostics: make_diagnostics(rows_scanned),
                });
            }

            warnings.push(CommWarning {
                code: "ROW_MISSING_CHANNEL_NAME".to_string(),
                message: format!("row {row_index}: missing channelName"),
                point_key: None,
                hmi_name: Some(hmi_name.clone()),
            });
        }

        let protocol_raw = idx_protocol
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .unwrap_or_default();

        let protocol = if strict {
            match parse_protocol_strict(&protocol_raw) {
                Some(v) => v,
                None => {
                    return Err(ImportUnionXlsxError::InvalidRequiredValue {
                        row_index: row_index as u32,
                        column_name: col_protocol.to_string(),
                        raw_value: protocol_raw,
                        allowed_values: spec_v1::ALLOWED_PROTOCOLS_V1
                            .iter()
                            .map(|v| v.to_string())
                            .collect(),
                        diagnostics: make_diagnostics(rows_scanned),
                    });
                }
            }
        } else {
            parse_protocol_loose(&protocol_raw)
                .or_else(|| {
                    // 若无协议字段，尝试按 TCP/485 参数列做推断；否则默认 TCP。
                    let ip_guess = idx_tcp_ip
                        .and_then(|i| row.get(i))
                        .and_then(cell_string)
                        .or_else(|| {
                            idx_ip_or_serial
                                .and_then(|i| row.get(i))
                                .and_then(cell_string)
                        })
                        .unwrap_or_default();

                    let serial_guess = idx_rtu_port
                        .and_then(|i| row.get(i))
                        .and_then(cell_string)
                        .or_else(|| {
                            idx_ip_or_serial
                                .and_then(|i| row.get(i))
                                .and_then(cell_string)
                        })
                        .unwrap_or_default();

                    if ip_guess.contains('.') {
                        return Some(ProtocolKind::Tcp);
                    }

                    if serial_guess.to_uppercase().starts_with("COM")
                        || serial_guess.starts_with("/dev/")
                    {
                        return Some(ProtocolKind::Rtu485);
                    }

                    None
                })
                .unwrap_or_else(|| {
                    warnings.push(CommWarning {
                        code: "ROW_PROTOCOL_UNKNOWN_DEFAULT_TCP".to_string(),
                        message: format!("row {row_index}: protocolType unknown; default to TCP"),
                        point_key: None,
                        hmi_name: Some(hmi_name.clone()),
                    });
                    ProtocolKind::Tcp
                })
        };

        let device_cell = idx_device_id.and_then(|i| row.get(i));
        let device_id = match device_cell.and_then(cell_u8) {
            Some(v) => v,
            None => {
                if strict {
                    let raw = device_cell.and_then(cell_string).unwrap_or_default();
                    return Err(ImportUnionXlsxError::InvalidRequiredValue {
                        row_index: row_index as u32,
                        column_name: col_device_id.to_string(),
                        raw_value: raw,
                        allowed_values: vec!["integer".to_string()],
                        diagnostics: make_diagnostics(rows_scanned),
                    });
                }

                warnings.push(CommWarning {
                    code: "ROW_MISSING_DEVICE_ID_DEFAULT_1".to_string(),
                    message: format!("row {row_index}: deviceId missing/invalid; default to 1"),
                    point_key: None,
                    hmi_name: Some(hmi_name.clone()),
                });
                1
            }
        };

        let data_type_raw = idx_data_type
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .unwrap_or_default();

        let data_type = if strict {
            match parse_data_type_strict(&data_type_raw) {
                Some(v) => v,
                None => {
                    return Err(ImportUnionXlsxError::InvalidRequiredValue {
                        row_index: row_index as u32,
                        column_name: col_data_type.to_string(),
                        raw_value: data_type_raw,
                        allowed_values: spec_v1::ALLOWED_DATATYPES_V1
                            .iter()
                            .map(|v| v.to_string())
                            .collect(),
                        diagnostics: make_diagnostics(rows_scanned),
                    });
                }
            }
        } else {
            parse_data_type_loose(&data_type_raw).unwrap_or_else(|| {
                warnings.push(CommWarning {
                    code: "ROW_DATATYPE_UNKNOWN_SKIP".to_string(),
                    message: format!("row {row_index}: dataType unknown; skipped"),
                    point_key: None,
                    hmi_name: Some(hmi_name.clone()),
                });
                DataType::Unknown
            })
        };

        if matches!(data_type, DataType::Unknown) {
            continue;
        }

        let byte_order_raw = idx_byte_order
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .unwrap_or_default();

        let byte_order = if strict {
            match parse_byte_order_strict(&byte_order_raw) {
                Some(v) => v,
                None => {
                    return Err(ImportUnionXlsxError::InvalidRequiredValue {
                        row_index: row_index as u32,
                        column_name: col_byte_order.to_string(),
                        raw_value: byte_order_raw,
                        allowed_values: spec_v1::ALLOWED_BYTEORDERS_V1
                            .iter()
                            .map(|v| v.to_string())
                            .collect(),
                        diagnostics: make_diagnostics(rows_scanned),
                    });
                }
            }
        } else {
            parse_byte_order_loose(&byte_order_raw).unwrap_or_else(|| {
                warnings.push(CommWarning {
                    code: "ROW_BYTEORDER_UNKNOWN_DEFAULT_ABCD".to_string(),
                    message: format!("row {row_index}: byteOrder unknown; default to ABCD"),
                    point_key: None,
                    hmi_name: Some(hmi_name.clone()),
                });
                ByteOrder32::ABCD
            })
        };

        let scale = idx_scale
            .and_then(|i| row.get(i))
            .and_then(cell_f64)
            .unwrap_or(1.0);
        let scale = if scale.is_finite() { scale } else { 1.0 };

        let address_offset = idx_point_offset.and_then(|i| row.get(i)).and_then(cell_u16);

        // Profile-level fields (optional; may be incomplete and should warn).
        let read_area = match idx_read_area.and_then(|i| row.get(i)).and_then(cell_string) {
            None => {
                if matches!(data_type, DataType::Bool) {
                    RegisterArea::Coil
                } else {
                    RegisterArea::Holding
                }
            }
            Some(raw) => {
                if raw.trim().is_empty() {
                    if matches!(data_type, DataType::Bool) {
                        RegisterArea::Coil
                    } else {
                        RegisterArea::Holding
                    }
                } else {
                    match parse_read_area(&raw) {
                        Some(v) => v,
                        None => {
                            if strict {
                                return Err(ImportUnionXlsxError::InvalidRequiredValue {
                                    row_index: row_index as u32,
                                    column_name: col_read_area.to_string(),
                                    raw_value: raw,
                                    allowed_values: spec_v1::ALLOWED_READ_AREAS_V1
                                        .iter()
                                        .map(|v| v.to_string())
                                        .collect(),
                                    diagnostics: make_diagnostics(rows_scanned),
                                });
                            }
                            warnings.push(CommWarning {
                                code: "ROW_READ_AREA_INVALID_DEFAULTED".to_string(),
                                message: format!("row {row_index}: readArea invalid; defaulted"),
                                point_key: None,
                                hmi_name: Some(hmi_name.clone()),
                            });
                            if matches!(data_type, DataType::Bool) {
                                RegisterArea::Coil
                            } else {
                                RegisterArea::Holding
                            }
                        }
                    }
                }
            }
        };

        let point_start_cell = idx_point_start.and_then(|i| row.get(i));
        let point_start_address = match point_start_cell.and_then(cell_string) {
            None => None,
            Some(raw) => {
                if raw.trim().is_empty() {
                    None
                } else {
                    let parsed = point_start_cell
                        .and_then(cell_u16)
                        .or_else(|| raw.trim().parse::<u16>().ok());

                    match parsed {
                        None => {
                            if strict {
                                return Err(ImportUnionXlsxError::InvalidRequiredValue {
                                    row_index: row_index as u32,
                                    column_name: col_point_start.to_string(),
                                    raw_value: raw.clone(),
                                    allowed_values: vec!["integer".to_string()],
                                    diagnostics: make_diagnostics(rows_scanned),
                                });
                            }
                            warnings.push(CommWarning {
                                code: "ROW_START_ADDRESS_INVALID_IGNORED".to_string(),
                                message: format!("row {row_index}: startAddress invalid; ignored"),
                                point_key: None,
                                hmi_name: Some(hmi_name.clone()),
                            });
                            None
                        }
                        Some(v) => {
                            match address_base_used {
                                AddressBase::Zero => Some(v),
                                AddressBase::One => {
                                    if v == 0 {
                                        if strict {
                                            return Err(ImportUnionXlsxError::InvalidRequiredValue {
                                            row_index: row_index as u32,
                                            column_name: col_point_start.to_string(),
                                            raw_value: raw.clone(),
                                            allowed_values: vec![spec_v1::ALLOWED_START_ADDRESS_ONE_BASED_MIN.to_string()],
                                            diagnostics: make_diagnostics(rows_scanned),
                                        });
                                        }
                                        warnings.push(CommWarning {
                                        code: "ROW_START_ADDRESS_ONE_BASED_ZERO".to_string(),
                                        message: format!(
                                            "row {row_index}: startAddress=0 under one-based; treated as 0"
                                        ),
                                        point_key: None,
                                        hmi_name: Some(hmi_name.clone()),
                                    });
                                        Some(0)
                                    } else {
                                        Some(v - 1)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        let profile_length = match idx_profile_len.and_then(|i| row.get(i)) {
            None => 0,
            Some(cell) => match cell_string(cell) {
                None => 0,
                Some(raw) => {
                    if raw.trim().is_empty() {
                        0
                    } else if let Some(v) =
                        cell_u16(cell).or_else(|| raw.trim().parse::<u16>().ok())
                    {
                        v
                    } else if strict {
                        return Err(ImportUnionXlsxError::InvalidRequiredValue {
                            row_index: row_index as u32,
                            column_name: col_length.to_string(),
                            raw_value: raw,
                            allowed_values: vec!["integer".to_string()],
                            diagnostics: make_diagnostics(rows_scanned),
                        });
                    } else {
                        warnings.push(CommWarning {
                            code: "ROW_PROFILE_LENGTH_INVALID_DEFAULTED".to_string(),
                            message: format!("row {row_index}: profile length invalid; defaulted"),
                            point_key: None,
                            hmi_name: Some(hmi_name.clone()),
                        });
                        0
                    }
                }
            },
        };
        let timeout_ms = idx_timeout
            .and_then(|i| row.get(i))
            .and_then(cell_u32)
            .unwrap_or(1000);
        let retry_count = idx_retry
            .and_then(|i| row.get(i))
            .and_then(cell_u32)
            .unwrap_or(0);
        let poll_interval_ms = idx_poll
            .and_then(|i| row.get(i))
            .and_then(cell_u32)
            .unwrap_or(1000);

        let legacy_ip_or_serial = idx_ip_or_serial
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .unwrap_or_default();
        let legacy_port_or_baud = idx_port_or_baud
            .and_then(|i| row.get(i))
            .and_then(cell_u32)
            .unwrap_or(0);

        let mut tcp_ip = idx_tcp_ip
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .unwrap_or_default();
        if tcp_ip.trim().is_empty() {
            tcp_ip = legacy_ip_or_serial.clone();
        }

        let mut tcp_port_u32 = idx_tcp_port
            .and_then(|i| row.get(i))
            .and_then(cell_u32)
            .unwrap_or(0);
        if tcp_port_u32 == 0 {
            tcp_port_u32 = legacy_port_or_baud;
        }
        let tcp_port: u16 = tcp_port_u32.try_into().unwrap_or(0);

        let mut rtu_serial_port = idx_rtu_port
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .unwrap_or_default();
        if rtu_serial_port.trim().is_empty() {
            rtu_serial_port = legacy_ip_or_serial.clone();
        }

        let mut rtu_baud_u32 = idx_rtu_baud
            .and_then(|i| row.get(i))
            .and_then(cell_u32)
            .unwrap_or(0);
        if rtu_baud_u32 == 0 {
            rtu_baud_u32 = legacy_port_or_baud;
        }
        let rtu_baud_rate: u32 = if rtu_baud_u32 == 0 {
            9600
        } else {
            rtu_baud_u32.max(300)
        };

        match protocol {
            ProtocolKind::Tcp => {
                if tcp_ip.trim().is_empty() || tcp_port == 0 {
                    warnings.push(CommWarning {
                        code: "PROFILE_TCP_PARAM_MISSING".to_string(),
                        message: format!(
                            "row {row_index}: TCP profile missing ip/port; a skeleton profile will be generated"
                        ),
                        point_key: None,
                        hmi_name: Some(hmi_name.clone()),
                    });
                }
            }
            ProtocolKind::Rtu485 => {
                if rtu_serial_port.trim().is_empty() {
                    warnings.push(CommWarning {
                        code: "PROFILE_RTU_PARAM_MISSING".to_string(),
                        message: format!(
                            "row {row_index}: 485 profile missing serial port; a skeleton profile will be generated"
                        ),
                        point_key: None,
                        hmi_name: Some(hmi_name.clone()),
                    });
                }
            }
        }

        let rtu_parity = idx_parity
            .and_then(|i| row.get(i))
            .and_then(cell_string)
            .and_then(|s| parse_parity(&s))
            .unwrap_or(SerialParity::None);
        let rtu_data_bits = idx_data_bits
            .and_then(|i| row.get(i))
            .and_then(cell_u8)
            .unwrap_or(8);
        let rtu_stop_bits = idx_stop_bits
            .and_then(|i| row.get(i))
            .and_then(cell_u8)
            .unwrap_or(1);

        let record = RowRecord {
            row_index,
            hmi_name: hmi_name.clone(),
            base_channel_name: base_channel_name.clone(),
            final_channel_name: String::new(), // placeholder; will be set after disambiguation
            protocol,
            device_id,
            read_area,
            point_start_address,
            profile_length,
            timeout_ms,
            retry_count,
            poll_interval_ms,
            data_type,
            byte_order,
            address_offset,
            scale,
            tcp_ip,
            tcp_port,
            rtu_serial_port,
            rtu_baud_rate,
            rtu_parity,
            rtu_data_bits,
            rtu_stop_bits,
        };

        raw_records.push((
            row_index,
            hmi_name,
            base_channel_name,
            protocol,
            device_id,
            record,
        ));
    }

    // 决策：当同一 base channelName + protocol 下出现多个 deviceId 时，为保证 profile channelName 唯一，自动拼接后缀。
    let mut device_ids_by_channel: HashMap<(String, ProtocolKind), HashSet<u8>> = HashMap::new();
    for (_row_index, _hmi, base_channel, protocol, device_id, _record) in &raw_records {
        device_ids_by_channel
            .entry((base_channel.clone(), *protocol))
            .or_default()
            .insert(*device_id);
    }

    let mut records: Vec<RowRecord> = Vec::with_capacity(raw_records.len());
    for (row_index, hmi_name, base_channel, protocol, device_id, mut record) in raw_records {
        let key = (base_channel.clone(), protocol);
        let needs_suffix = device_ids_by_channel
            .get(&key)
            .map(|set| set.len() > 1)
            .unwrap_or(false);

        let final_channel_name = if base_channel.trim().is_empty() {
            warnings.push(CommWarning {
                code: "PROFILE_CHANNEL_NAME_DEFAULTED".to_string(),
                message: format!(
                    "row {row_index}: channelName missing; defaulted to UNNAMED_{device_id}"
                ),
                point_key: None,
                hmi_name: Some(hmi_name.clone()),
            });
            format!("UNNAMED_{device_id}")
        } else if needs_suffix {
            warnings.push(CommWarning {
                code: "PROFILE_CHANNEL_DISAMBIGUATED".to_string(),
                message: format!(
                    "row {row_index}: channelName '{}' has multiple deviceId; using '{}@{}'",
                    base_channel, base_channel, device_id
                ),
                point_key: None,
                hmi_name: Some(hmi_name.clone()),
            });
            format!("{}@{}", base_channel, device_id)
        } else {
            base_channel.clone()
        };

        record.final_channel_name = final_channel_name;
        records.push(record);
    }

    // 将“点位绝对起始地址（起始地址列）”转换为内部 `addressOffset`（相对 profile.startAddress）。
    // - 若存在显式 `addressOffset` 列（legacy），保留；若同时提供起始地址，则起始地址优先并 warning。
    // - profile.startAddress 默认取同一 channel 下所有点位起始地址的最小值；若都未提供则默认 0。
    let mut base_start_by_channel: HashMap<String, u16> = HashMap::new();
    for record in &records {
        let Some(start) = record.point_start_address else {
            continue;
        };
        base_start_by_channel
            .entry(record.final_channel_name.clone())
            .and_modify(|v| *v = (*v).min(start))
            .or_insert(start);
    }

    for record in &records {
        base_start_by_channel
            .entry(record.final_channel_name.clone())
            .or_insert(0);
    }

    for record in records.iter_mut() {
        if let Some(start) = record.point_start_address {
            let base = *base_start_by_channel
                .get(record.final_channel_name.as_str())
                .unwrap_or(&0);

            if record.address_offset.is_some() {
                warnings.push(CommWarning {
                    code: "ROW_BOTH_START_AND_OFFSET_START_WINS".to_string(),
                    message: format!(
                        "row {}: both startAddress and addressOffset provided; startAddress wins",
                        record.row_index
                    ),
                    point_key: None,
                    hmi_name: Some(record.hmi_name.clone()),
                });
            }

            record.address_offset = Some(start.saturating_sub(base));
        }
    }

    // 聚合 profiles（按 final channelName）
    #[derive(Clone, Debug)]
    struct ProfileSeed {
        protocol: ProtocolKind,
        channel_name: String,
        device_id: u8,
        read_area: RegisterArea,
        start_address: u16,
        length: u16,
        ip: String,
        port: u16,
        serial_port: String,
        baud_rate: u32,
        parity: SerialParity,
        data_bits: u8,
        stop_bits: u8,
        timeout_ms: u32,
        retry_count: u32,
        poll_interval_ms: u32,
    }

    let mut profiles_by_channel: HashMap<String, ProfileSeed> = HashMap::new();
    for record in &records {
        let base_start = *base_start_by_channel
            .get(record.final_channel_name.as_str())
            .unwrap_or(&0);
        profiles_by_channel
            .entry(record.final_channel_name.clone())
            .and_modify(|seed| {
                // 简单冲突检测：仅在不同值时告警，保持 first-wins。
                if seed.device_id != record.device_id {
                    warnings.push(CommWarning {
                        code: "PROFILE_CONFLICT_DEVICE_ID".to_string(),
                        message: format!(
                            "channelName='{}': deviceId conflict; keep first({})",
                            record.final_channel_name, seed.device_id
                        ),
                        point_key: None,
                        hmi_name: None,
                    });
                }
            })
            .or_insert_with(|| ProfileSeed {
                protocol: record.protocol,
                channel_name: record.final_channel_name.clone(),
                device_id: record.device_id,
                read_area: record.read_area.clone(),
                start_address: base_start,
                length: record.profile_length,
                ip: record.tcp_ip.clone(),
                port: record.tcp_port,
                serial_port: record.rtu_serial_port.clone(),
                baud_rate: record.rtu_baud_rate,
                parity: record.rtu_parity.clone(),
                data_bits: record.rtu_data_bits,
                stop_bits: record.rtu_stop_bits,
                timeout_ms: record.timeout_ms,
                retry_count: record.retry_count,
                poll_interval_ms: record.poll_interval_ms,
            });
    }

    // 如果 profile.length 未提供，则按点位推导一个最小长度：
    // - `sum_units`：把所有点位按“无间隙顺排”时的最小长度（覆盖无 offset 的情况）
    // - `max_end`：存在显式地址/偏移时的最大 end（覆盖显式 gap 的情况）
    let mut len_stats_by_channel: HashMap<String, (u16, u16)> = HashMap::new(); // (sum_units, max_end)
    for record in &records {
        let unit_len: u16 = match record.data_type {
            DataType::Bool => 1,
            DataType::Int16 | DataType::UInt16 => 1,
            DataType::Int32 | DataType::UInt32 | DataType::Float32 => 2,
            DataType::Int64 | DataType::UInt64 | DataType::Float64 => 4,
            DataType::Unknown => 1,
        };

        let entry = len_stats_by_channel
            .entry(record.final_channel_name.clone())
            .or_insert((0, 0));
        entry.0 = entry.0.saturating_add(unit_len);

        if let Some(offset) = record.address_offset {
            let end = offset.saturating_add(unit_len);
            entry.1 = entry.1.max(end);
        }
    }

    for (channel, (sum_units, max_end)) in &len_stats_by_channel {
        let required_len = (*sum_units).max(*max_end).max(1);
        if let Some(seed) = profiles_by_channel.get_mut(channel) {
            if seed.length == 0 {
                seed.length = required_len;
                warnings.push(CommWarning {
                    code: "PROFILE_LENGTH_DEFAULTED".to_string(),
                    message: format!(
                        "channelName='{}': profile length missing; defaulted to {}",
                        channel, seed.length
                    ),
                    point_key: None,
                    hmi_name: None,
                });
            }
        }
    }

    let mut profiles: Vec<ConnectionProfile> = Vec::new();
    for seed in profiles_by_channel.values() {
        match seed.protocol {
            ProtocolKind::Tcp => profiles.push(ConnectionProfile::Tcp {
                channel_name: seed.channel_name.clone(),
                device_id: seed.device_id,
                read_area: seed.read_area.clone(),
                start_address: seed.start_address,
                length: seed.length,
                ip: seed.ip.clone(),
                port: seed.port,
                timeout_ms: seed.timeout_ms,
                retry_count: seed.retry_count,
                poll_interval_ms: seed.poll_interval_ms,
            }),
            ProtocolKind::Rtu485 => profiles.push(ConnectionProfile::Rtu485 {
                channel_name: seed.channel_name.clone(),
                device_id: seed.device_id,
                read_area: seed.read_area.clone(),
                start_address: seed.start_address,
                length: seed.length,
                serial_port: seed.serial_port.clone(),
                baud_rate: seed.baud_rate,
                parity: seed.parity.clone(),
                data_bits: seed.data_bits,
                stop_bits: seed.stop_bits,
                timeout_ms: seed.timeout_ms,
                retry_count: seed.retry_count,
                poll_interval_ms: seed.poll_interval_ms,
            }),
        }
    }

    // 确定性排序：按 channelName 排序，便于输出稳定。
    profiles.sort_by(|a, b| profile_channel_name(a).cmp(profile_channel_name(b)));

    let mut points: Vec<CommPoint> = Vec::new();
    let mut seen_keys: HashSet<Uuid> = HashSet::new();

    for record in &records {
        let point_key = stable_point_key(
            &record.hmi_name,
            &record.base_channel_name,
            record.device_id,
        );
        if !seen_keys.insert(point_key) {
            warnings.push(CommWarning {
                code: "DUPLICATE_POINT_KEY_SKIP".to_string(),
                message: format!(
                    "duplicate pointKey for hmiName='{}', channelName='{}', deviceId={}; keep first",
                    record.hmi_name, record.base_channel_name, record.device_id
                ),
                point_key: Some(point_key),
                hmi_name: Some(record.hmi_name.clone()),
            });
            continue;
        }

        points.push(CommPoint {
            point_key,
            hmi_name: record.hmi_name.clone(),
            data_type: record.data_type.clone(),
            byte_order: record.byte_order.clone(),
            channel_name: record.final_channel_name.clone(),
            address_offset: record.address_offset,
            scale: record.scale,
        });
    }

    Ok(ImportUnionXlsxOutcome {
        points: PointsV1 {
            schema_version: SCHEMA_VERSION_V1,
            points,
        },
        profiles: ProfilesV1 {
            schema_version: SCHEMA_VERSION_V1,
            profiles,
        },
        warnings,
        diagnostics: make_diagnostics(rows_scanned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_xlsxwriter::Workbook;
    use std::path::PathBuf;

    #[test]
    fn point_key_is_deterministic_for_same_inputs() {
        let a = stable_point_key("TEMP", "tcp-1", 1);
        let b = stable_point_key("TEMP", "tcp-1", 1);
        assert_eq!(a, b);

        let c = stable_point_key("TEMP", "tcp-1", 2);
        assert_ne!(a, c);
    }

    fn temp_xlsx_path(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("plccodeforge_{prefix}_{}.xlsx", Uuid::new_v4()))
    }

    fn write_xlsx(path: &Path, sheet_name: &str, headers: &[&str], rows: &[Vec<&str>]) {
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_name(sheet_name).unwrap();

        for (col, header) in headers.iter().enumerate() {
            sheet.write_string(0, col as u16, *header).unwrap();
        }

        for (row_idx, row) in rows.iter().enumerate() {
            let excel_row = (row_idx + 1) as u32;
            for (col, value) in row.iter().enumerate() {
                sheet.write_string(excel_row, col as u16, *value).unwrap();
            }
        }

        workbook.save(path).unwrap();
    }

    #[test]
    fn strict_missing_sheet_fails_with_available_sheet_list() {
        let path = temp_xlsx_path("missing_sheet");
        write_xlsx(&path, "OtherSheet", &spec_v1::REQUIRED_COLUMNS_V1, &[]);

        let err = import_union_xlsx_with_options(
            &path,
            Some(ImportUnionOptions {
                strict: Some(true),
                sheet_name: Some(spec_v1::DEFAULT_SHEET_V1.to_string()),
                address_base: None,
            }),
        )
        .unwrap_err();

        let ImportUnionXlsxError::MissingSheet {
            sheet_name,
            detected_sheets,
            ..
        } = &err
        else {
            panic!("expected MissingSheet error, got: {err:?}");
        };

        assert_eq!(sheet_name.as_str(), spec_v1::DEFAULT_SHEET_V1);
        assert!(detected_sheets.iter().any(|s| s == "OtherSheet"));

        let import_error = err.to_import_error();
        assert_eq!(
            import_error.kind,
            ImportUnionErrorKind::UnionXlsxInvalidSheet
        );
        assert!(import_error.message.contains("sheet not found"));
        assert!(import_error
            .details
            .as_ref()
            .and_then(|d| d.detected_sheets.as_ref())
            .is_some());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn strict_missing_required_columns_fails_with_missing_list() {
        let path = temp_xlsx_path("missing_columns");
        let headers = [
            "变量名称（HMI）",
            "数据类型",
            // 缺少：字节序
            "通道名称",
            "协议类型",
            "设备标识",
        ];
        write_xlsx(&path, spec_v1::DEFAULT_SHEET_V1, &headers, &[]);

        let err = import_union_xlsx_with_options(
            &path,
            Some(ImportUnionOptions {
                strict: Some(true),
                sheet_name: None,
                address_base: None,
            }),
        )
        .unwrap_err();

        let ImportUnionXlsxError::MissingRequiredColumns {
            missing_columns, ..
        } = &err
        else {
            panic!("expected MissingRequiredColumns error, got: {err:?}");
        };

        assert!(missing_columns.iter().any(|c| c == "字节序"));
        let import_error = err.to_import_error();
        assert_eq!(
            import_error.kind,
            ImportUnionErrorKind::UnionXlsxMissingColumns
        );
        assert!(import_error
            .details
            .as_ref()
            .and_then(|d| d.missing_columns.as_ref())
            .map(|v| v.iter().any(|c| c == "字节序"))
            .unwrap_or(false));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn strict_invalid_data_type_fails_with_row_index() {
        let path = temp_xlsx_path("invalid_datatype");
        let headers = spec_v1::REQUIRED_COLUMNS_V1;
        let rows = vec![vec!["TEMP_1", "BADTYPE", "ABCD", "tcp-1", "TCP", "1"]];
        write_xlsx(&path, spec_v1::DEFAULT_SHEET_V1, &headers, &rows);

        let err = import_union_xlsx_with_options(
            &path,
            Some(ImportUnionOptions {
                strict: Some(true),
                sheet_name: None,
                address_base: None,
            }),
        )
        .unwrap_err();

        let ImportUnionXlsxError::InvalidRequiredValue {
            row_index,
            column_name,
            ..
        } = &err
        else {
            panic!("expected InvalidRequiredValue error, got: {err:?}");
        };

        assert_eq!(*row_index, 2);
        assert_eq!(column_name.as_str(), "数据类型");
        let import_error = err.to_import_error();
        assert_eq!(
            import_error.kind,
            ImportUnionErrorKind::UnionXlsxInvalidEnum
        );
        assert_eq!(
            import_error.details.as_ref().and_then(|d| d.row_index),
            Some(2)
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn loose_mode_returns_warnings_instead_of_failing() {
        let path = temp_xlsx_path("loose_mode_warns");
        let headers = spec_v1::REQUIRED_COLUMNS_V1;
        let rows = vec![vec!["TEMP_1", "BADTYPE", "ABCD", "tcp-1", "TCP", "1"]];
        write_xlsx(&path, spec_v1::DEFAULT_SHEET_V1, &headers, &rows);

        let outcome = import_union_xlsx_with_options(
            &path,
            Some(ImportUnionOptions {
                strict: Some(false),
                sheet_name: None,
                address_base: None,
            }),
        )
        .unwrap();

        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.code == "ROW_DATATYPE_UNKNOWN_SKIP"));

        let _ = std::fs::remove_file(&path);
    }
}
