//! 通讯采集模块：联合表（IO+设备表） × 通讯采集 Stub → UnifiedImport v1（TASK-37）
//!
//! 约束：
//! - UnifiedImport v1 一旦对外使用即冻结：只允许新增可选字段，不得改名/删字段/改语义。
//! - 合并主键：HMI 变量名称（name）。union xlsx 内部 name 重复 => fail-fast。
//! - 以 union xlsx 为主集合：stub 中 unmatched 点位仅输出 warning，不合入 points。
//! - 文件 IO 必须可在 spawn_blocking 中执行（command 不阻塞 UI）。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

use super::bridge_importresult_stub::{ImportResultStubV1, IMPORT_RESULT_STUB_SPEC_VERSION_V1};
use super::bridge_plc_import::{PlcImportBridgeV1PointComm, PlcImportBridgeV1PointVerification};
use super::error::{MergeImportSourcesError, MergeImportSourcesErrorDetails, MergeImportSourcesErrorKind};
use super::import_union_xlsx;
use super::model::{ByteOrder32, CommWarning, ConnectionProfile, DataType, RegisterArea, SCHEMA_VERSION_V1};
use super::union_xlsx_parser;

pub const UNIFIED_IMPORT_SPEC_VERSION_V1: &str = "v1";
pub const MERGE_REPORT_SPEC_VERSION_V1: &str = "v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedImportV1Source {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedImportV1Sources {
    pub union_xlsx: UnifiedImportV1Source,
    pub comm_stub: UnifiedImportV1Source,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedImportV1PointDesign {
    pub channel_name: String,
    pub data_type: DataType,
    pub byte_order: ByteOrder32,
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_offset: Option<u16>,
    /// 原始输入通道名称（来自联合表列“通道名称”）；当同一 base channelName + protocol 下出现多 deviceId 时，
    /// `channelName` 会被自动拼接后缀（如 `CH@1`），此字段用于追溯原始输入。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_channel_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_area: Option<RegisterArea>,
    /// 点位绝对地址（内部 0-based）；若联合表未提供起始地址则为 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_start_address: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_address: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp_ip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtu_serial_port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtu_baud_rate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtu_parity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtu_data_bits: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtu_stop_bits: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_interval_ms: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedImportV1Point {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<UnifiedImportV1PointDesign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comm: Option<PlcImportBridgeV1PointComm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<PlcImportBridgeV1PointVerification>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedImportV1Statistics {
    pub union_points: u32,
    pub stub_points: u32,
    pub matched: u32,
    pub unmatched_stub: u32,
    pub overridden: u32,
    pub conflicts: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedImportV1 {
    pub schema_version: u32,
    pub spec_version: String,
    pub generated_at_utc: DateTime<Utc>,
    pub sources: UnifiedImportV1Sources,
    pub points: Vec<UnifiedImportV1Point>,
    pub device_groups: Vec<JsonValue>,
    pub hardware: JsonValue,
    pub statistics: UnifiedImportV1Statistics,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MergeConflictV1 {
    pub name: String,
    pub field: String,
    pub union_value: String,
    pub stub_value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MergeReportV1 {
    pub schema_version: u32,
    pub spec_version: String,
    pub generated_at_utc: DateTime<Utc>,
    pub sources: UnifiedImportV1Sources,
    pub matched_count: u32,
    pub unmatched_stub_points: Vec<String>,
    pub overridden_count: u32,
    pub conflicts: Vec<MergeConflictV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MergeImportSourcesSummary {
    pub union_points: u32,
    pub stub_points: u32,
    pub matched: u32,
    pub unmatched_stub: u32,
    pub overridden: u32,
    pub conflicts: u32,
    pub union_xlsx_digest: String,
    pub import_result_stub_digest: String,
    pub unified_import_digest: String,
    pub merge_report_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parsed_columns_used: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct MergeImportSourcesOutcome {
    pub out_path: PathBuf,
    pub report_path: PathBuf,
    pub summary: MergeImportSourcesSummary,
    pub warnings: Vec<CommWarning>,
}

fn sha256_prefixed_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    format!("sha256:{hex}")
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), std::io::Error> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn profile_snapshot(profile: &ConnectionProfile) -> (String, u8, RegisterArea, u16, u16) {
    match profile {
        ConnectionProfile::Tcp {
            device_id,
            read_area,
            start_address,
            length,
            ..
        } => (
            "TCP".to_string(),
            *device_id,
            read_area.clone(),
            *start_address,
            *length,
        ),
        ConnectionProfile::Rtu485 {
            device_id,
            read_area,
            start_address,
            length,
            ..
        } => (
            "485".to_string(),
            *device_id,
            read_area.clone(),
            *start_address,
            *length,
        ),
    }
}

fn validate_stub(stub_path: &Path, stub: &ImportResultStubV1) -> Result<(), MergeImportSourcesError> {
    if stub.schema_version != SCHEMA_VERSION_V1 {
        return Err(MergeImportSourcesError {
            kind: MergeImportSourcesErrorKind::ImportResultStubUnsupportedSchemaVersion,
            message: format!("unsupported ImportResultStub schemaVersion: {}", stub.schema_version),
            details: Some(MergeImportSourcesErrorDetails {
                import_result_stub_path: Some(stub_path.to_string_lossy().to_string()),
                ..Default::default()
            }),
        });
    }
    if stub.spec_version != IMPORT_RESULT_STUB_SPEC_VERSION_V1 {
        return Err(MergeImportSourcesError {
            kind: MergeImportSourcesErrorKind::ImportResultStubUnsupportedSpecVersion,
            message: format!("unsupported ImportResultStub specVersion: {}", stub.spec_version),
            details: Some(MergeImportSourcesErrorDetails {
                import_result_stub_path: Some(stub_path.to_string_lossy().to_string()),
                ..Default::default()
            }),
        });
    }

    let mut names: HashSet<String> = HashSet::new();
    for p in &stub.points {
        let name = p.name.trim();
        if name.is_empty() {
            return Err(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::ImportResultStubValidationError,
                message: "stub point name is empty".to_string(),
                details: Some(MergeImportSourcesErrorDetails {
                    import_result_stub_path: Some(stub_path.to_string_lossy().to_string()),
                    field: Some("points.name".to_string()),
                    ..Default::default()
                }),
            });
        }
        if !names.insert(name.to_string()) {
            return Err(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::ImportResultStubValidationError,
                message: "duplicate stub points.name detected".to_string(),
                details: Some(MergeImportSourcesErrorDetails {
                    import_result_stub_path: Some(stub_path.to_string_lossy().to_string()),
                    name: Some(name.to_string()),
                    field: Some("points.name".to_string()),
                    ..Default::default()
                }),
            });
        }
    }

    Ok(())
}

fn fmt_f64(v: f64) -> String {
    if v.is_finite() {
        let s = format!("{v}");
        if s.contains('.') {
            s
        } else {
            format!("{s}.0")
        }
    } else {
        format!("{v}")
    }
}

fn guess_input_channel_name(channel_name: &str, device_id: Option<u8>) -> Option<String> {
    let Some(device_id) = device_id else {
        return None;
    };
    let suffix = format!("@{device_id}");
    if channel_name.ends_with(&suffix) {
        let prefix = channel_name.trim_end_matches(&suffix).to_string();
        let prefix = prefix.trim_end_matches('@').to_string();
        if !prefix.trim().is_empty() {
            return Some(prefix);
        }
    }
    None
}

pub fn merge_import_sources_v1(
    union_xlsx_path: &Path,
    import_result_stub_path: &Path,
    out_path: &Path,
    report_path: &Path,
) -> Result<MergeImportSourcesOutcome, MergeImportSourcesError> {
    let union_bytes = std::fs::read(union_xlsx_path).map_err(|e| MergeImportSourcesError {
        kind: MergeImportSourcesErrorKind::UnionXlsxReadError,
        message: e.to_string(),
        details: Some(MergeImportSourcesErrorDetails {
            union_xlsx_path: Some(union_xlsx_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;
    let union_digest = sha256_prefixed_bytes(&union_bytes);

    let union_out = import_union_xlsx::import_union_xlsx_with_options(
        union_xlsx_path,
        Some(import_union_xlsx::ImportUnionOptions {
            strict: Some(true),
            sheet_name: None,
            address_base: None,
        }),
    )
    .map_err(|e| MergeImportSourcesError {
        kind: MergeImportSourcesErrorKind::UnionXlsxParseError,
        message: format!("{e:?}"),
        details: Some(MergeImportSourcesErrorDetails {
            union_xlsx_path: Some(union_xlsx_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    let columns_info = union_xlsx_parser::columns_info_from_detected(&union_out.diagnostics.detected_columns);
    let mut warnings: Vec<CommWarning> = union_out.warnings.clone();
    warnings.extend(union_xlsx_parser::missing_columns_warnings(&columns_info));

    let mut profiles_by_channel: HashMap<String, ConnectionProfile> = HashMap::new();
    for p in &union_out.profiles.profiles {
        let channel_name = match p {
            ConnectionProfile::Tcp { channel_name, .. } => channel_name.clone(),
            ConnectionProfile::Rtu485 { channel_name, .. } => channel_name.clone(),
        };
        profiles_by_channel.insert(channel_name, p.clone());
    }

    let mut seen_names: HashSet<String> = HashSet::new();
    let mut union_points: Vec<UnifiedImportV1Point> = Vec::with_capacity(union_out.points.points.len());
    for p in &union_out.points.points {
        let name = p.hmi_name.trim().to_string();
        if name.is_empty() {
            return Err(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::UnionXlsxValidationError,
                message: "union xlsx point name is empty".to_string(),
                details: Some(MergeImportSourcesErrorDetails {
                    union_xlsx_path: Some(union_xlsx_path.to_string_lossy().to_string()),
                    field: Some("points.hmiName".to_string()),
                    ..Default::default()
                }),
            });
        }
        if !seen_names.insert(name.clone()) {
            return Err(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::UnionXlsxValidationError,
                message: "duplicate points.name detected in union xlsx (HMI variable name must be unique)".to_string(),
                details: Some(MergeImportSourcesErrorDetails {
                    union_xlsx_path: Some(union_xlsx_path.to_string_lossy().to_string()),
                    name: Some(name),
                    field: Some("points.name".to_string()),
                    ..Default::default()
                }),
            });
        }

        let mut protocol_type: Option<String> = None;
        let mut device_id: Option<u8> = None;
        let mut read_area: Option<RegisterArea> = None;
        let mut start_address: Option<u16> = None;
        let mut length: Option<u16> = None;

        let mut tcp_ip: Option<String> = None;
        let mut tcp_port: Option<u16> = None;
        let mut rtu_serial_port: Option<String> = None;
        let mut rtu_baud_rate: Option<u32> = None;
        let mut rtu_parity: Option<String> = None;
        let mut rtu_data_bits: Option<u8> = None;
        let mut rtu_stop_bits: Option<u8> = None;
        let mut timeout_ms: Option<u32> = None;
        let mut retry_count: Option<u32> = None;
        let mut poll_interval_ms: Option<u32> = None;

        if let Some(profile) = profiles_by_channel.get(&p.channel_name) {
            let (proto, dev, area, start, len) = profile_snapshot(profile);
            protocol_type = Some(proto);
            device_id = Some(dev);
            read_area = Some(area);
            start_address = Some(start);
            length = Some(len);

            match profile {
                ConnectionProfile::Tcp {
                    ip,
                    port,
                    timeout_ms: t,
                    retry_count: r,
                    poll_interval_ms: poll,
                    ..
                } => {
                    tcp_ip = Some(ip.clone());
                    tcp_port = Some(*port);
                    timeout_ms = Some(*t);
                    retry_count = Some(*r);
                    poll_interval_ms = Some(*poll);
                }
                ConnectionProfile::Rtu485 {
                    serial_port,
                    baud_rate,
                    parity,
                    data_bits,
                    stop_bits,
                    timeout_ms: t,
                    retry_count: r,
                    poll_interval_ms: poll,
                    ..
                } => {
                    rtu_serial_port = Some(serial_port.clone());
                    rtu_baud_rate = Some(*baud_rate);
                    rtu_parity = Some(format!("{parity:?}"));
                    rtu_data_bits = Some(*data_bits);
                    rtu_stop_bits = Some(*stop_bits);
                    timeout_ms = Some(*t);
                    retry_count = Some(*r);
                    poll_interval_ms = Some(*poll);
                }
            }
        }

        let point_start_address = match (start_address, p.address_offset) {
            (Some(base), Some(off)) => Some(base.saturating_add(off)),
            _ => None,
        };

        let input_channel_name = guess_input_channel_name(&p.channel_name, device_id);

        union_points.push(UnifiedImportV1Point {
            name,
            design: Some(UnifiedImportV1PointDesign {
                channel_name: p.channel_name.clone(),
                data_type: p.data_type.clone(),
                byte_order: p.byte_order.clone(),
                scale: p.scale,
                address_offset: p.address_offset,
                input_channel_name,
                protocol_type,
                device_id,
                read_area,
                point_start_address,
                start_address,
                length,
                tcp_ip,
                tcp_port,
                rtu_serial_port,
                rtu_baud_rate,
                rtu_parity,
                rtu_data_bits,
                rtu_stop_bits,
                timeout_ms,
                retry_count,
                poll_interval_ms,
            }),
            comm: None,
            verification: None,
        });
    }

    let stub_text = std::fs::read_to_string(import_result_stub_path).map_err(|e| MergeImportSourcesError {
        kind: MergeImportSourcesErrorKind::ImportResultStubReadError,
        message: e.to_string(),
        details: Some(MergeImportSourcesErrorDetails {
            import_result_stub_path: Some(import_result_stub_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;
    let stub_digest = sha256_prefixed_bytes(stub_text.as_bytes());
    let stub: ImportResultStubV1 = serde_json::from_str(&stub_text).map_err(|e| MergeImportSourcesError {
        kind: MergeImportSourcesErrorKind::ImportResultStubDeserializeError,
        message: e.to_string(),
        details: Some(MergeImportSourcesErrorDetails {
            import_result_stub_path: Some(import_result_stub_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;
    validate_stub(import_result_stub_path, &stub)?;

    let mut stub_by_name: HashMap<String, (PlcImportBridgeV1PointComm, PlcImportBridgeV1PointVerification)> =
        HashMap::new();
    for p in &stub.points {
        stub_by_name.insert(
            p.name.trim().to_string(),
            (p.comm.clone(), p.verification.clone()),
        );
    }

    let mut matched = 0u32;
    let mut overridden = 0u32;
    let mut conflicts: Vec<MergeConflictV1> = Vec::new();

    let mut unmatched_stub_points: Vec<String> = Vec::new();
    let union_name_set: HashSet<String> = union_points.iter().map(|p| p.name.clone()).collect();

    for (name, _) in &stub_by_name {
        if !union_name_set.contains(name) {
            unmatched_stub_points.push(name.clone());
            warnings.push(CommWarning {
                code: "UNMATCHED_STUB_POINT".to_string(),
                message: "point exists in comm stub but not found in union xlsx; skipped (union is authoritative)".to_string(),
                point_key: None,
                hmi_name: Some(name.clone()),
            });
        }
    }
    unmatched_stub_points.sort();

    for p in &mut union_points {
        if let Some((comm, ver)) = stub_by_name.get(&p.name) {
            p.comm = Some(comm.clone());
            p.verification = Some(ver.clone());
            matched += 1;
            overridden += 1;

            if let Some(design) = &p.design {
                if design.channel_name != comm.channel_name {
                    conflicts.push(MergeConflictV1 {
                        name: p.name.clone(),
                        field: "channelName".to_string(),
                        union_value: design.channel_name.clone(),
                        stub_value: comm.channel_name.clone(),
                    });
                }
                if design.data_type != comm.data_type {
                    conflicts.push(MergeConflictV1 {
                        name: p.name.clone(),
                        field: "dataType".to_string(),
                        union_value: format!("{:?}", design.data_type),
                        stub_value: format!("{:?}", comm.data_type),
                    });
                }
                if design.byte_order != comm.endian {
                    conflicts.push(MergeConflictV1 {
                        name: p.name.clone(),
                        field: "byteOrder".to_string(),
                        union_value: format!("{:?}", design.byte_order),
                        stub_value: format!("{:?}", comm.endian),
                    });
                }
                if fmt_f64(design.scale) != fmt_f64(comm.scale) {
                    conflicts.push(MergeConflictV1 {
                        name: p.name.clone(),
                        field: "scale".to_string(),
                        union_value: fmt_f64(design.scale),
                        stub_value: fmt_f64(comm.scale),
                    });
                }
            }
        }
    }

    let now = Utc::now();
    let sources = UnifiedImportV1Sources {
        union_xlsx: UnifiedImportV1Source {
            path: union_xlsx_path.to_string_lossy().to_string(),
            digest: Some(union_digest.clone()),
        },
        comm_stub: UnifiedImportV1Source {
            path: import_result_stub_path.to_string_lossy().to_string(),
            digest: Some(stub_digest.clone()),
        },
    };

    let unified = UnifiedImportV1 {
        schema_version: SCHEMA_VERSION_V1,
        spec_version: UNIFIED_IMPORT_SPEC_VERSION_V1.to_string(),
        generated_at_utc: now,
        sources: sources.clone(),
        points: union_points,
        device_groups: union_xlsx_parser::build_device_groups(&union_out.points, &union_out.profiles),
        hardware: union_xlsx_parser::build_hardware_snapshot(&union_out.profiles, &columns_info),
        statistics: UnifiedImportV1Statistics {
            union_points: union_out.points.points.len() as u32,
            stub_points: stub.points.len() as u32,
            matched,
            unmatched_stub: unmatched_stub_points.len() as u32,
            overridden,
            conflicts: conflicts.len() as u32,
        },
    };

    let unified_text = serde_json::to_string_pretty(&unified).map_err(|e| MergeImportSourcesError {
        kind: MergeImportSourcesErrorKind::MergeWriteError,
        message: e.to_string(),
        details: Some(MergeImportSourcesErrorDetails {
            out_path: Some(out_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;
    let unified_digest = sha256_prefixed_bytes(unified_text.as_bytes());

    write_text_atomic(out_path, &unified_text).map_err(|e| MergeImportSourcesError {
        kind: MergeImportSourcesErrorKind::MergeWriteError,
        message: e.to_string(),
        details: Some(MergeImportSourcesErrorDetails {
            out_path: Some(out_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    let report = MergeReportV1 {
        schema_version: SCHEMA_VERSION_V1,
        spec_version: MERGE_REPORT_SPEC_VERSION_V1.to_string(),
        generated_at_utc: now,
        sources,
        matched_count: matched,
        unmatched_stub_points: unmatched_stub_points.clone(),
        overridden_count: overridden,
        conflicts,
    };
    let report_text = serde_json::to_string_pretty(&report).map_err(|e| MergeImportSourcesError {
        kind: MergeImportSourcesErrorKind::MergeWriteError,
        message: e.to_string(),
        details: Some(MergeImportSourcesErrorDetails {
            report_path: Some(report_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;
    let report_digest = sha256_prefixed_bytes(report_text.as_bytes());
    write_text_atomic(report_path, &report_text).map_err(|e| MergeImportSourcesError {
        kind: MergeImportSourcesErrorKind::MergeWriteError,
        message: e.to_string(),
        details: Some(MergeImportSourcesErrorDetails {
            report_path: Some(report_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    Ok(MergeImportSourcesOutcome {
        out_path: out_path.to_path_buf(),
        report_path: report_path.to_path_buf(),
        summary: MergeImportSourcesSummary {
            union_points: union_out.points.points.len() as u32,
            stub_points: stub.points.len() as u32,
            matched,
            unmatched_stub: unmatched_stub_points.len() as u32,
            overridden,
            conflicts: report.conflicts.len() as u32,
            union_xlsx_digest: union_digest,
            import_result_stub_digest: stub_digest,
            unified_import_digest: unified_digest,
            merge_report_digest: report_digest,
            parsed_columns_used: Some(columns_info.parsed_columns_used.clone()),
        },
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::export_ir::CommIrV1AddressSpec;
    use crate::comm::export_plc_import_stub::{export_plc_import_stub_v1, PlcImportStubV1};
    use crate::comm::model::{Quality, RunStats};
    use rust_xlsxwriter::Workbook;
    use uuid::Uuid;

    fn temp_xlsx_path(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("plc-codeforge-merge-{prefix}-{}.xlsx", Uuid::new_v4()))
    }

    fn write_union_xlsx(path: &Path, rows: &[Vec<&str>]) {
        let headers = crate::comm::union_spec_v1::REQUIRED_COLUMNS_V1;
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_name(crate::comm::union_spec_v1::DEFAULT_SHEET_V1).unwrap();

        for (col, header) in headers.iter().enumerate() {
            sheet.write_string(0, col as u16, *header).unwrap();
        }

        for (row_idx, row) in rows.iter().enumerate() {
            let r = (row_idx + 1) as u32;
            for (col, value) in row.iter().enumerate() {
                sheet.write_string(r, col as u16, *value).unwrap();
            }
        }

        workbook.save(path).unwrap();
    }

    fn stub_point(name: &str, channel: &str) -> (String, PlcImportBridgeV1PointComm, PlcImportBridgeV1PointVerification) {
        let comm = PlcImportBridgeV1PointComm {
            channel_name: channel.to_string(),
            address_spec: CommIrV1AddressSpec {
                read_area: Some(RegisterArea::Holding),
                absolute_address: Some(10),
                unit_length: Some(1),
                profile_start_address: Some(0),
                profile_length: Some(100),
                offset_from_profile_start: Some(10),
                job_start_address: Some(0),
                job_length: Some(100),
                address_base: "zero".to_string(),
            },
            data_type: DataType::Int16,
            endian: ByteOrder32::ABCD,
            scale: 1.0,
            rw: "RO".to_string(),
        };

        let ver = PlcImportBridgeV1PointVerification {
            quality: Quality::Ok,
            value_display: "123".to_string(),
            timestamp: Utc::now(),
            message: "".to_string(),
        };

        (name.to_string(), comm, ver)
    }

    fn write_stub(path: &Path, points: Vec<(String, PlcImportBridgeV1PointComm, PlcImportBridgeV1PointVerification)>) {
        let now = Utc::now();
        let stub = ImportResultStubV1 {
            schema_version: SCHEMA_VERSION_V1,
            spec_version: IMPORT_RESULT_STUB_SPEC_VERSION_V1.to_string(),
            generated_at_utc: now,
            source_bridge_path: "bridge.json".to_string(),
            source_bridge_digest: "sha256:deadbeef".to_string(),
            points: points
                .into_iter()
                .map(|(name, comm, verification)| crate::comm::bridge_plc_import::PlcImportBridgeV1Point {
                    name,
                    comm,
                    verification,
                })
                .collect(),
            device_groups: Vec::new(),
            hardware: JsonValue::Object(serde_json::Map::new()),
            statistics: RunStats {
                total: 0,
                ok: 0,
                timeout: 0,
                comm_error: 0,
                decode_error: 0,
                config_error: 0,
            },
        };
        let text = serde_json::to_string_pretty(&stub).unwrap();
        write_text_atomic(path, &text).unwrap();
    }

    #[test]
    fn merge_applies_stub_comm_and_verification_for_matched_points() {
        let xlsx = temp_xlsx_path("matched");
        write_union_xlsx(
            &xlsx,
            &[
                vec!["P1", "Int16", "ABCD", "tcp-1", "TCP", "1"],
                vec!["P2", "Int16", "ABCD", "tcp-1", "TCP", "1"],
            ],
        );

        let dir = std::env::temp_dir().join(format!("plc-codeforge-merge-out-{}", Uuid::new_v4()));
        let stub_path = dir.join("import_result_stub.v1.json");
        write_stub(&stub_path, vec![stub_point("P1", "tcp-1")]);

        let out_path = dir.join("unified_import.v1.json");
        let report_path = dir.join("merge_report.v1.json");

        let outcome = merge_import_sources_v1(&xlsx, &stub_path, &out_path, &report_path).unwrap();
        assert_eq!(outcome.summary.union_points, 2);
        assert_eq!(outcome.summary.stub_points, 1);
        assert_eq!(outcome.summary.matched, 1);

        let unified_text = std::fs::read_to_string(&out_path).unwrap();
        let unified: UnifiedImportV1 = serde_json::from_str(&unified_text).unwrap();
        let p1 = unified.points.iter().find(|p| p.name == "P1").unwrap();
        assert!(p1.comm.is_some());
        assert!(p1.verification.is_some());
        let p2 = unified.points.iter().find(|p| p.name == "P2").unwrap();
        assert!(p2.comm.is_none());
        assert!(p2.verification.is_none());

        let _ = std::fs::remove_file(&xlsx);
    }

    #[test]
    fn merge_keeps_union_as_authoritative_and_reports_unmatched_stub_points_as_warning() {
        let xlsx = temp_xlsx_path("unmatched");
        write_union_xlsx(&xlsx, &[vec!["P1", "Int16", "ABCD", "tcp-1", "TCP", "1"]]);

        let dir = std::env::temp_dir().join(format!("plc-codeforge-merge-out-{}", Uuid::new_v4()));
        let stub_path = dir.join("import_result_stub.v1.json");
        write_stub(&stub_path, vec![stub_point("P1", "tcp-1"), stub_point("EXTRA", "tcp-1")]);

        let out_path = dir.join("unified_import.v1.json");
        let report_path = dir.join("merge_report.v1.json");

        let outcome = merge_import_sources_v1(&xlsx, &stub_path, &out_path, &report_path).unwrap();
        assert_eq!(outcome.summary.unmatched_stub, 1);
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.code == "UNMATCHED_STUB_POINT" && w.hmi_name.as_deref() == Some("EXTRA")));

        let unified_text = std::fs::read_to_string(&out_path).unwrap();
        let unified: UnifiedImportV1 = serde_json::from_str(&unified_text).unwrap();
        assert_eq!(unified.points.len(), 1);

        let _ = std::fs::remove_file(&xlsx);
    }

    #[test]
    fn mapping_consistency_unified_to_plc_stub_keeps_comm_and_verification() {
        let xlsx = temp_xlsx_path("consistency");
        write_union_xlsx(
            &xlsx,
            &[
                vec!["P1", "Int16", "ABCD", "tcp-1", "TCP", "1"],
                vec!["P2", "Int16", "ABCD", "tcp-1", "TCP", "1"],
            ],
        );

        let dir = std::env::temp_dir().join(format!("plc-codeforge-consistency-{}", Uuid::new_v4()));
        let stub_path = dir.join("import_result_stub.v1.json");
        write_stub(
            &stub_path,
            vec![stub_point("P1", "tcp-1"), stub_point("P2", "tcp-1")],
        );

        let unified_path = dir.join("unified_import.v1.json");
        let report_path = dir.join("merge_report.v1.json");
        let merged = merge_import_sources_v1(&xlsx, &stub_path, &unified_path, &report_path).unwrap();
        assert_eq!(merged.summary.matched, 2);

        let plc_stub_path = dir.join("plc_import.v1.json");
        let _ = export_plc_import_stub_v1(&unified_path, &plc_stub_path).unwrap();

        let unified_text = std::fs::read_to_string(&unified_path).unwrap();
        let unified: UnifiedImportV1 = serde_json::from_str(&unified_text).unwrap();

        let plc_text = std::fs::read_to_string(&plc_stub_path).unwrap();
        let plc: PlcImportStubV1 = serde_json::from_str(&plc_text).unwrap();

        assert_eq!(unified.points.len(), plc.points.len());
        for (u, p) in unified.points.iter().zip(plc.points.iter()) {
            assert_eq!(u.name, p.name);
            assert_eq!(u.comm, p.comm);
            assert_eq!(u.verification, p.verification);
        }

        let _ = std::fs::remove_file(&xlsx);
    }
}
