//! 交付版导出：通讯地址表.xlsx（三张冻结表 + 可选 Results）。
//!
//! 硬约束：
//! - 三张固定 sheet 的列名与顺序逐字冻结，不允许改动。
//! - 任何新增信息不得通过改列实现（Results sheet 为可选附加，不计入冻结三表）。

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use rust_xlsxwriter::{Format, Workbook, XlsxError};
use uuid::Uuid;

use crate::comm::core::model::{
    ByteOrder32, CommExportDiagnostics, CommPoint, CommWarning, ConnectionProfile, DataType,
    ExportedRows, Quality, RegisterArea, RunStats, SampleResult, SerialParity,
};

pub const TCP_SHEET_NAME_V1: &str = "TCP通讯地址表";
pub const RTU485_SHEET_NAME_V1: &str = "485通讯地址表";
pub const PARAMS_SHEET_NAME_V1: &str = "通讯参数";
pub const RESULTS_SHEET_NAME: &str = "采集结果";

/// TCP通讯地址表（5列，冻结 v1）
pub const TCP_HEADERS_V1: [&str; 5] = super::export_xlsx::HEADERS_TCP;
/// 485通讯地址表（5列，冻结 v1）
pub const RTU485_HEADERS_V1: [&str; 5] = super::export_xlsx::HEADERS_RTU485;
/// 通讯参数（14列，冻结 v1）
pub const PARAM_HEADERS_V1: [&str; 14] = super::export_xlsx::HEADERS_PARAMS;

/// 可选 Results sheet 的表头（不属于冻结三表）。
pub const RESULTS_HEADERS: [&str; 6] = [
    "变量名称（HMI）",
    "quality",
    "valueDisplay",
    "errorMessage",
    "timestamp",
    "durationMs",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportDeliveryHeaders {
    pub tcp: Vec<String>,
    pub rtu: Vec<String>,
    pub params: Vec<String>,
}

impl ExportDeliveryHeaders {
    pub fn from_consts() -> Self {
        Self {
            tcp: TCP_HEADERS_V1.iter().map(|s| s.to_string()).collect(),
            rtu: RTU485_HEADERS_V1.iter().map(|s| s.to_string()).collect(),
            params: PARAM_HEADERS_V1.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExportDeliveryOutcome {
    pub headers: ExportDeliveryHeaders,
    pub warnings: Vec<CommWarning>,
    pub diagnostics: CommExportDiagnostics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelKind {
    Tcp,
    Rtu485,
}

#[derive(Clone, Debug)]
struct RowSeed {
    index: usize,
    point_key: Uuid,
    hmi_name: String,
    data_type: String,
    byte_order: String,
    channel_name: String,
    scale: f64,
}

pub fn export_delivery_xlsx(
    out_path: &Path,
    profiles: &[ConnectionProfile],
    points: &[CommPoint],
    include_results: bool,
    last_results: Option<&[SampleResult]>,
    _last_stats: Option<&RunStats>,
) -> Result<ExportDeliveryOutcome, XlsxError> {
    let started = Instant::now();
    let mut warnings: Vec<CommWarning> = Vec::new();

    let mut profile_kind_by_channel: HashMap<&str, ChannelKind> = HashMap::new();
    for p in profiles {
        match p {
            ConnectionProfile::Tcp { channel_name, .. } => {
                profile_kind_by_channel.insert(channel_name.as_str(), ChannelKind::Tcp);
            }
            ConnectionProfile::Rtu485 { channel_name, .. } => {
                profile_kind_by_channel.insert(channel_name.as_str(), ChannelKind::Rtu485);
            }
        }
    }

    let mut tcp_rows: Vec<RowSeed> = Vec::new();
    let mut rtu_rows: Vec<RowSeed> = Vec::new();

    for (index, point) in points.iter().enumerate() {
        let seed = RowSeed {
            index,
            point_key: point.point_key,
            hmi_name: point.hmi_name.clone(),
            data_type: data_type_to_str(&point.data_type).to_string(),
            byte_order: byte_order_to_str(&point.byte_order).to_string(),
            channel_name: point.channel_name.clone(),
            scale: point.scale,
        };

        match profile_kind_by_channel.get(point.channel_name.as_str()) {
            Some(ChannelKind::Tcp) => tcp_rows.push(seed),
            Some(ChannelKind::Rtu485) => rtu_rows.push(seed),
            None => {
                warnings.push(CommWarning {
                    code: "POINT_MISSING_PROFILE_DEFAULT_TCP".to_string(),
                    message: format!(
                        "point channelName='{}' has no matching profile; defaulted to TCP sheet",
                        point.channel_name
                    ),
                    point_key: Some(point.point_key),
                    hmi_name: Some(point.hmi_name.clone()),
                });
                tcp_rows.push(seed);
            }
        }
    }

    // 确定性排序：按 points 原始顺序；tie-break 用 pointKey。
    tcp_rows.sort_by(|a, b| {
        a.index
            .cmp(&b.index)
            .then_with(|| a.point_key.cmp(&b.point_key))
    });
    rtu_rows.sort_by(|a, b| {
        a.index
            .cmp(&b.index)
            .then_with(|| a.point_key.cmp(&b.point_key))
    });

    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold();

    {
        let tcp_sheet = workbook.add_worksheet();
        tcp_sheet.set_name(TCP_SHEET_NAME_V1)?;
        write_headers(tcp_sheet, &TCP_HEADERS_V1, &header_format)?;

        let mut row: u32 = 1;
        for seed in &tcp_rows {
            tcp_sheet.write_string(row, 0, &seed.hmi_name)?;
            tcp_sheet.write_string(row, 1, &seed.data_type)?;
            tcp_sheet.write_string(row, 2, &seed.byte_order)?;
            tcp_sheet.write_string(row, 3, &seed.channel_name)?;
            let scale = if seed.scale.is_finite() {
                seed.scale
            } else {
                0.0
            };
            tcp_sheet.write_number(row, 4, scale)?;
            row += 1;
        }
    }

    {
        let rtu_sheet = workbook.add_worksheet();
        rtu_sheet.set_name(RTU485_SHEET_NAME_V1)?;
        write_headers(rtu_sheet, &RTU485_HEADERS_V1, &header_format)?;

        let mut row: u32 = 1;
        for seed in &rtu_rows {
            rtu_sheet.write_string(row, 0, &seed.hmi_name)?;
            rtu_sheet.write_string(row, 1, &seed.data_type)?;
            rtu_sheet.write_string(row, 2, &seed.byte_order)?;
            rtu_sheet.write_string(row, 3, &seed.channel_name)?;
            let scale = if seed.scale.is_finite() {
                seed.scale
            } else {
                0.0
            };
            rtu_sheet.write_number(row, 4, scale)?;
            row += 1;
        }
    }

    {
        let params_sheet = workbook.add_worksheet();
        params_sheet.set_name(PARAMS_SHEET_NAME_V1)?;
        write_headers(params_sheet, &PARAM_HEADERS_V1, &header_format)?;

        let mut params_row: u32 = 1;
        // 稳定输出：按 channelName + deviceId 排序。
        let mut sorted_profiles: Vec<&ConnectionProfile> = profiles.iter().collect();
        sorted_profiles.sort_by(|a, b| {
            let (a_ch, a_dev) = profile_key(a);
            let (b_ch, b_dev) = profile_key(b);
            a_ch.cmp(b_ch).then_with(|| a_dev.cmp(&b_dev))
        });

        for profile in sorted_profiles {
            match profile {
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
                } => {
                    params_sheet.write_string(params_row, 0, "TCP")?;
                    params_sheet.write_string(params_row, 1, channel_name)?;
                    params_sheet.write_number(params_row, 2, f64::from(*device_id))?;
                    params_sheet.write_string(params_row, 3, register_area_to_str(read_area))?;
                    params_sheet.write_number(params_row, 4, f64::from(*start_address))?;
                    params_sheet.write_number(params_row, 5, f64::from(*length))?;
                    params_sheet.write_string(params_row, 6, ip)?;
                    params_sheet.write_number(params_row, 7, f64::from(*port))?;
                    params_sheet.write_string(params_row, 8, "")?;
                    params_sheet.write_string(params_row, 9, "")?;
                    params_sheet.write_string(params_row, 10, "")?;
                    params_sheet.write_number(params_row, 11, f64::from(*timeout_ms))?;
                    params_sheet.write_number(params_row, 12, f64::from(*retry_count))?;
                    params_sheet.write_number(params_row, 13, f64::from(*poll_interval_ms))?;
                }
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
                } => {
                    params_sheet.write_string(params_row, 0, "485")?;
                    params_sheet.write_string(params_row, 1, channel_name)?;
                    params_sheet.write_number(params_row, 2, f64::from(*device_id))?;
                    params_sheet.write_string(params_row, 3, register_area_to_str(read_area))?;
                    params_sheet.write_number(params_row, 4, f64::from(*start_address))?;
                    params_sheet.write_number(params_row, 5, f64::from(*length))?;
                    params_sheet.write_string(params_row, 6, serial_port)?;
                    params_sheet.write_number(params_row, 7, f64::from(*baud_rate))?;
                    params_sheet.write_string(params_row, 8, serial_parity_to_str(parity))?;
                    params_sheet.write_number(params_row, 9, f64::from(*data_bits))?;
                    params_sheet.write_number(params_row, 10, f64::from(*stop_bits))?;
                    params_sheet.write_number(params_row, 11, f64::from(*timeout_ms))?;
                    params_sheet.write_number(params_row, 12, f64::from(*retry_count))?;
                    params_sheet.write_number(params_row, 13, f64::from(*poll_interval_ms))?;
                }
            }

            params_row += 1;
        }
    }

    if include_results {
        if let Some(results) = last_results {
            let mut by_point_key: HashMap<Uuid, &SampleResult> = HashMap::new();
            for r in results {
                by_point_key.insert(r.point_key, r);
            }

            let results_sheet = workbook.add_worksheet();
            results_sheet.set_name(RESULTS_SHEET_NAME)?;
            write_headers(results_sheet, &RESULTS_HEADERS, &header_format)?;

            let mut row: u32 = 1;
            for point in points {
                results_sheet.write_string(row, 0, &point.hmi_name)?;
                if let Some(r) = by_point_key.get(&point.point_key) {
                    results_sheet.write_string(row, 1, quality_to_str(&r.quality))?;
                    results_sheet.write_string(row, 2, &r.value_display)?;
                    results_sheet.write_string(row, 3, &r.error_message)?;
                    results_sheet.write_string(row, 4, &r.timestamp.to_rfc3339())?;
                    results_sheet.write_number(row, 5, f64::from(r.duration_ms))?;
                } else {
                    results_sheet.write_string(row, 1, "")?;
                    results_sheet.write_string(row, 2, "")?;
                    results_sheet.write_string(row, 3, "")?;
                    results_sheet.write_string(row, 4, "")?;
                    results_sheet.write_number(row, 5, 0.0)?;
                }
                row += 1;
            }
        } else {
            warnings.push(CommWarning {
                code: "DELIVERY_RESULTS_MISSING".to_string(),
                message: "includeResults=true but no last_results available; results sheet skipped"
                    .to_string(),
                point_key: None,
                hmi_name: None,
            });
        }
    }

    workbook.save(out_path)?;

    let duration_ms = started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
    let diagnostics = CommExportDiagnostics {
        exported_rows: ExportedRows {
            tcp: tcp_rows.len().min(u32::MAX as usize) as u32,
            rtu: rtu_rows.len().min(u32::MAX as usize) as u32,
            params: profiles.len().min(u32::MAX as usize) as u32,
        },
        duration_ms,
    };

    Ok(ExportDeliveryOutcome {
        headers: ExportDeliveryHeaders::from_consts(),
        warnings,
        diagnostics,
    })
}

fn profile_key(profile: &ConnectionProfile) -> (&str, u8) {
    match profile {
        ConnectionProfile::Tcp {
            channel_name,
            device_id,
            ..
        } => (channel_name.as_str(), *device_id),
        ConnectionProfile::Rtu485 {
            channel_name,
            device_id,
            ..
        } => (channel_name.as_str(), *device_id),
    }
}

fn write_headers(
    sheet: &mut rust_xlsxwriter::Worksheet,
    headers: &[&str],
    format: &Format,
) -> Result<(), XlsxError> {
    for (col, header) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, col as u16, *header, format)?;
    }
    Ok(())
}

fn data_type_to_str(data_type: &DataType) -> &'static str {
    match data_type {
        DataType::Bool => "Bool",
        DataType::Int16 => "Int16",
        DataType::UInt16 => "UInt16",
        DataType::Int32 => "Int32",
        DataType::UInt32 => "UInt32",
        DataType::Int64 => "Int64",
        DataType::UInt64 => "UInt64",
        DataType::Float32 => "Float32",
        DataType::Float64 => "Float64",
        DataType::Unknown => "",
    }
}

fn byte_order_to_str(byte_order: &ByteOrder32) -> &'static str {
    match byte_order {
        ByteOrder32::ABCD => "ABCD",
        ByteOrder32::BADC => "BADC",
        ByteOrder32::CDAB => "CDAB",
        ByteOrder32::DCBA => "DCBA",
        ByteOrder32::Unknown => "",
    }
}

fn register_area_to_str(area: &RegisterArea) -> &'static str {
    match area {
        RegisterArea::Holding => "Holding",
        RegisterArea::Input => "Input",
        RegisterArea::Coil => "Coil",
        RegisterArea::Discrete => "Discrete",
    }
}

fn serial_parity_to_str(parity: &SerialParity) -> &'static str {
    match parity {
        SerialParity::None => "None",
        SerialParity::Even => "Even",
        SerialParity::Odd => "Odd",
    }
}

fn quality_to_str(quality: &Quality) -> &'static str {
    match quality {
        Quality::Ok => "Ok",
        Quality::Timeout => "Timeout",
        Quality::CommError => "CommError",
        Quality::DecodeError => "DecodeError",
        Quality::ConfigError => "ConfigError",
    }
}
