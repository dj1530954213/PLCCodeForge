//! XLSX 导出（冻结规范 v1）。
//!
//! 输出工作簿必须包含 3 个 sheet，且列名与顺序逐字匹配：
//! - TCP通讯地址表（5列）
//! - 485通讯地址表（5列）
//! - 通讯参数（14列）

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use rust_xlsxwriter::{Format, Workbook, XlsxError};
use thiserror::Error;

use crate::comm::core::model::{
    ByteOrder32, CommExportDiagnostics, CommPoint, CommWarning, ConnectionProfile, DataType,
    ExportedRows, RegisterArea, SerialParity,
};

pub const TCP_SHEET_NAME: &str = "TCP通讯地址表";
pub const RTU485_SHEET_NAME: &str = "485通讯地址表";
pub const PARAMS_SHEET_NAME: &str = "通讯参数";

pub const HEADERS_TCP: [&str; 5] = [
    "变量名称（HMI）",
    "数据类型",
    "字节序",
    "起始TCP通道名称",
    "缩放倍数",
];

pub const HEADERS_RTU485: [&str; 5] = [
    "变量名称（HMI）",
    "数据类型",
    "字节序",
    "起始485通道名称",
    "缩放倍数",
];

pub const HEADERS_PARAMS: [&str; 14] = [
    "协议类型",
    "通道名称",
    "设备标识",
    "读取区域",
    "起始地址",
    "长度",
    "TCP:IP / 485:串口",
    "TCP:端口 / 485:波特率",
    "485:校验",
    "485:数据位",
    "485:停止位",
    "超时ms",
    "重试次数",
    "轮询周期ms",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportHeaders {
    pub tcp_sheet: Vec<String>,
    pub rtu485_sheet: Vec<String>,
    pub params_sheet: Vec<String>,
}

impl ExportHeaders {
    pub fn from_consts() -> Self {
        Self {
            tcp_sheet: HEADERS_TCP.iter().map(|s| (*s).to_string()).collect(),
            rtu485_sheet: HEADERS_RTU485.iter().map(|s| (*s).to_string()).collect(),
            params_sheet: HEADERS_PARAMS.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExportXlsxError {
    #[error("xlsx error: {0}")]
    Xlsx(#[from] XlsxError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExportOutcome {
    pub headers: ExportHeaders,
    pub warnings: Vec<CommWarning>,
    pub diagnostics: CommExportDiagnostics,
}

pub fn export_comm_address_xlsx(
    out_path: &Path,
    profiles: &[ConnectionProfile],
    points: &[CommPoint],
) -> Result<ExportOutcome, ExportXlsxError> {
    let started = Instant::now();
    let channel_kind = build_channel_kind_map(profiles);
    let mut warnings: Vec<CommWarning> = Vec::new();

    #[derive(Clone, Debug)]
    struct PointRowSeed {
        index: usize,
        point_key: uuid::Uuid,
        hmi_name: String,
        data_type: String,
        byte_order: String,
        channel_name: String,
        scale: f64,
    }

    let mut tcp_rows: Vec<PointRowSeed> = Vec::new();
    let mut rtu_rows: Vec<PointRowSeed> = Vec::new();

    for (index, point) in points.iter().enumerate() {
        if point.hmi_name.trim().is_empty() {
            warnings.push(CommWarning {
                code: "POINT_MISSING_HMI_NAME".to_string(),
                message: "point hmiName is empty".to_string(),
                point_key: Some(point.point_key),
                hmi_name: Some(point.hmi_name.clone()),
            });
        }

        if matches!(point.data_type, DataType::Unknown) {
            warnings.push(CommWarning {
                code: "POINT_DATATYPE_UNKNOWN".to_string(),
                message: "point dataType is unknown".to_string(),
                point_key: Some(point.point_key),
                hmi_name: Some(point.hmi_name.clone()),
            });
        }

        if matches!(point.byte_order, ByteOrder32::Unknown) {
            warnings.push(CommWarning {
                code: "POINT_BYTEORDER_UNKNOWN".to_string(),
                message: "point byteOrder is unknown".to_string(),
                point_key: Some(point.point_key),
                hmi_name: Some(point.hmi_name.clone()),
            });
        }

        if point.channel_name.trim().is_empty() {
            warnings.push(CommWarning {
                code: "POINT_MISSING_CHANNEL_NAME".to_string(),
                message: "point channelName is empty".to_string(),
                point_key: Some(point.point_key),
                hmi_name: Some(point.hmi_name.clone()),
            });
        }

        if !point.scale.is_finite() {
            warnings.push(CommWarning {
                code: "POINT_SCALE_INVALID".to_string(),
                message: "point scale is not a finite number; exported cell will use 0".to_string(),
                point_key: Some(point.point_key),
                hmi_name: Some(point.hmi_name.clone()),
            });
        } else if point.scale.abs() > 1.0e9 {
            warnings.push(CommWarning {
                code: "POINT_SCALE_TOO_LARGE".to_string(),
                message: format!("point scale is very large: {}", point.scale),
                point_key: Some(point.point_key),
                hmi_name: Some(point.hmi_name.clone()),
            });
        }

        let seed = PointRowSeed {
            index,
            point_key: point.point_key,
            hmi_name: point.hmi_name.clone(),
            data_type: data_type_to_str(&point.data_type).to_string(),
            byte_order: byte_order_to_str(&point.byte_order).to_string(),
            channel_name: point.channel_name.clone(),
            scale: point.scale,
        };

        match channel_kind.get(point.channel_name.as_str()) {
            Some(ChannelKind::Tcp) => tcp_rows.push(seed),
            Some(ChannelKind::Rtu485) => rtu_rows.push(seed),
            None => {
                if point.channel_name.trim().is_empty() {
                    warnings.push(CommWarning {
                        code: "POINT_NO_SHEET_MATCH".to_string(),
                        message: "point has no channelName, cannot determine TCP/485 sheet; defaulted to TCP sheet".to_string(),
                        point_key: Some(point.point_key),
                        hmi_name: Some(point.hmi_name.clone()),
                    });
                } else {
                    warnings.push(CommWarning {
                        code: "POINT_MISSING_PROFILE".to_string(),
                        message: format!(
                            "point references channelName='{}' but no profile exists; defaulted to TCP sheet",
                            point.channel_name
                        ),
                        point_key: Some(point.point_key),
                        hmi_name: Some(point.hmi_name.clone()),
                    });
                }

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
        tcp_sheet.set_name(TCP_SHEET_NAME)?;
        write_headers(tcp_sheet, &HEADERS_TCP, &header_format)?;

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
        rtu_sheet.set_name(RTU485_SHEET_NAME)?;
        write_headers(rtu_sheet, &HEADERS_RTU485, &header_format)?;

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
        params_sheet.set_name(PARAMS_SHEET_NAME)?;
        write_headers(params_sheet, &HEADERS_PARAMS, &header_format)?;

        let mut params_row: u32 = 1;
        for profile in profiles {
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
                    ..
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
                    ..
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

    Ok(ExportOutcome {
        headers: ExportHeaders::from_consts(),
        warnings,
        diagnostics,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelKind {
    Tcp,
    Rtu485,
}

fn build_channel_kind_map(profiles: &[ConnectionProfile]) -> HashMap<&str, ChannelKind> {
    let mut map: HashMap<&str, ChannelKind> = HashMap::new();
    for profile in profiles {
        match profile {
            ConnectionProfile::Tcp { channel_name, .. } => {
                map.insert(channel_name.as_str(), ChannelKind::Tcp);
            }
            ConnectionProfile::Rtu485 { channel_name, .. } => {
                map.insert(channel_name.as_str(), ChannelKind::Rtu485);
            }
        }
    }
    map
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
        DataType::Float32 => "Float32",
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

#[cfg(test)]
mod tests {
    use super::*;

    use uuid::Uuid;

    #[test]
    fn export_xlsx_writes_file_and_returns_frozen_headers() {
        let profiles = vec![
            ConnectionProfile::Tcp {
                channel_name: "tcp-1".to_string(),
                device_id: 1,
                read_area: RegisterArea::Holding,
                start_address: 0,
                length: 10,
                ip: "127.0.0.1".to_string(),
                port: 502,
                timeout_ms: 1000,
                retry_count: 1,
                poll_interval_ms: 500,
            },
            ConnectionProfile::Rtu485 {
                channel_name: "485-1".to_string(),
                device_id: 2,
                read_area: RegisterArea::Coil,
                start_address: 0,
                length: 10,
                serial_port: "COM1".to_string(),
                baud_rate: 9600,
                parity: SerialParity::None,
                data_bits: 8,
                stop_bits: 1,
                timeout_ms: 1000,
                retry_count: 1,
                poll_interval_ms: 500,
            },
        ];

        let points = vec![
            CommPoint {
                point_key: Uuid::from_u128(1),
                hmi_name: "P1".to_string(),
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                channel_name: "tcp-1".to_string(),
                address_offset: None,
                scale: 1.0,
            },
            CommPoint {
                point_key: Uuid::from_u128(2),
                hmi_name: "P2".to_string(),
                data_type: DataType::Bool,
                byte_order: ByteOrder32::ABCD,
                channel_name: "485-1".to_string(),
                address_offset: None,
                scale: 1.0,
            },
        ];

        let out_path = std::env::temp_dir().join(format!(
            "PLCCodeForge-TASK-10-通讯地址表-{}.xlsx",
            Uuid::new_v4()
        ));
        let outcome = export_comm_address_xlsx(&out_path, &profiles, &points).unwrap();

        println!("outPath={}", out_path.display());
        println!("headers.tcp={:?}", outcome.headers.tcp_sheet);
        println!("headers.rtu={:?}", outcome.headers.rtu485_sheet);
        println!("headers.params={:?}", outcome.headers.params_sheet);
        println!("warnings={:?}", outcome.warnings);
        println!("diagnostics={:?}", outcome.diagnostics);

        assert_eq!(outcome.headers, ExportHeaders::from_consts());
        assert!(out_path.exists());
        assert!(std::fs::metadata(&out_path).unwrap().len() > 0);
    }

    #[test]
    fn export_xlsx_emits_warnings_without_changing_frozen_headers() {
        let profiles = vec![ConnectionProfile::Tcp {
            channel_name: "tcp-1".to_string(),
            device_id: 1,
            read_area: RegisterArea::Holding,
            start_address: 0,
            length: 10,
            ip: "127.0.0.1".to_string(),
            port: 502,
            timeout_ms: 1000,
            retry_count: 1,
            poll_interval_ms: 500,
        }];

        let points = vec![
            // 缺 channelName
            CommPoint {
                point_key: Uuid::from_u128(10),
                hmi_name: "MISSING_CHANNEL".to_string(),
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                channel_name: "".to_string(),
                address_offset: None,
                scale: 1.0,
            },
            // 缺 profile
            CommPoint {
                point_key: Uuid::from_u128(11),
                hmi_name: "MISSING_PROFILE".to_string(),
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                channel_name: "tcp-missing".to_string(),
                address_offset: None,
                scale: 1.0,
            },
            // unknown dataType / byteOrder / scale
            CommPoint {
                point_key: Uuid::from_u128(12),
                hmi_name: "UNKNOWN_TYPES".to_string(),
                data_type: DataType::Unknown,
                byte_order: ByteOrder32::Unknown,
                channel_name: "tcp-1".to_string(),
                address_offset: None,
                scale: f64::NAN,
            },
        ];

        let out_path = std::env::temp_dir().join(format!(
            "PLCCodeForge-TASK-15-通讯地址表-warnings-{}.xlsx",
            Uuid::new_v4()
        ));
        let outcome = export_comm_address_xlsx(&out_path, &profiles, &points).unwrap();

        let warning_codes: Vec<String> = outcome.warnings.iter().map(|w| w.code.clone()).collect();
        println!("outPath={}", out_path.display());
        println!("warningCodes={warning_codes:?}");
        println!("warnings={:?}", outcome.warnings);
        println!("diagnostics={:?}", outcome.diagnostics);

        assert_eq!(outcome.headers, ExportHeaders::from_consts());
        assert!(warning_codes.contains(&"POINT_MISSING_CHANNEL_NAME".to_string()));
        assert!(warning_codes
            .iter()
            .any(|c| c == "POINT_MISSING_PROFILE" || c == "POINT_NO_SHEET_MATCH"));
    }
}
