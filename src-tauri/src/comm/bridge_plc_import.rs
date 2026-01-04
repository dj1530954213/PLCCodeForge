//! 通讯采集模块：CommIR v1 → PLC 自动编辑 ImportResult 的桥接导出（TASK-33 / TASK-34）
//!
//! 约束：
//! - 仅做“映射 + 校验 + 落盘”，不接入 plc_core、不做程序生成。
//! - 字段冻结：PlcImportBridge v1 一旦对外使用，只允许新增可选字段，不得改名/删字段/改语义。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::error::{
    BridgeCheckError, BridgeCheckErrorDetails, BridgeCheckErrorKind, PlcBridgeError,
    PlcBridgeErrorDetails, PlcBridgeErrorKind,
};
use super::export_ir::{CommIrV1, CommIrV1Point, CommIrV1Result, CommIrResultsSource, COMM_IR_SPEC_VERSION_V1};
use super::model::{ByteOrder32, DataType, Quality, RegisterArea, RunStats, SCHEMA_VERSION_V1};

pub const PLC_IMPORT_BRIDGE_SPEC_VERSION_V1: &str = "v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportBridgeV1PointComm {
    pub channel_name: String,
    pub address_spec: super::export_ir::CommIrV1AddressSpec,
    pub data_type: DataType,
    pub endian: ByteOrder32,
    pub scale: f64,
    pub rw: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportBridgeV1PointVerification {
    pub quality: Quality,
    pub value_display: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportBridgeV1Point {
    pub name: String,
    pub comm: PlcImportBridgeV1PointComm,
    pub verification: PlcImportBridgeV1PointVerification,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportBridgeV1 {
    pub schema_version: u32,
    pub spec_version: String,
    pub generated_at_utc: DateTime<Utc>,
    pub source_ir_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ir_digest: Option<String>,
    pub sources: PlcImportBridgeV1Sources,
    pub points: Vec<PlcImportBridgeV1Point>,
    pub statistics: RunStats,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportBridgeV1Sources {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub union_xlsx_path: Option<String>,
    pub results_source: CommIrResultsSource,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportBridgeExportSummary {
    pub points: u32,
    pub stats: RunStats,
    pub source_ir_digest: String,
    pub plc_bridge_digest: String,
}

#[derive(Clone, Debug)]
pub struct PlcImportBridgeExportOutcome {
    pub out_path: PathBuf,
    pub summary: PlcImportBridgeExportSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConsumerSummaryPoint {
    pub name: String,
    pub channel_name: String,
    pub read_area: Option<RegisterArea>,
    pub absolute_address: Option<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConsumerSummary {
    pub schema_version: u32,
    pub spec_version: String,
    pub generated_at_utc: DateTime<Utc>,
    pub bridge_path: String,
    pub total_points: u32,
    pub by_channel: BTreeMap<String, u32>,
    pub by_quality: BTreeMap<String, u32>,
    pub first10: Vec<BridgeConsumerSummaryPoint>,
}

#[derive(Clone, Debug)]
pub struct BridgeConsumerOutcome {
    pub out_path: PathBuf,
    pub summary: BridgeConsumerSummary,
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

fn compute_stats_from_qualities(qualities: impl Iterator<Item = Quality>) -> RunStats {
    let mut stats = RunStats {
        total: 0,
        ok: 0,
        timeout: 0,
        comm_error: 0,
        decode_error: 0,
        config_error: 0,
    };

    for q in qualities {
        stats.total += 1;
        match q {
            Quality::Ok => stats.ok += 1,
            Quality::Timeout => stats.timeout += 1,
            Quality::CommError => stats.comm_error += 1,
            Quality::DecodeError => stats.decode_error += 1,
            Quality::ConfigError => stats.config_error += 1,
        }
    }

    stats
}

fn validate_ir_points(ir_path: &Path, ir: &CommIrV1) -> Result<(), PlcBridgeError> {
    if ir.schema_version != SCHEMA_VERSION_V1 {
        return Err(PlcBridgeError {
            kind: PlcBridgeErrorKind::CommIrUnsupportedSchemaVersion,
            message: format!("unsupported CommIR schemaVersion: {}", ir.schema_version),
            details: Some(PlcBridgeErrorDetails {
                ir_path: Some(ir_path.to_string_lossy().to_string()),
                schema_version: Some(ir.schema_version),
                spec_version: Some(ir.spec_version.clone()),
                ..Default::default()
            }),
        });
    }
    if ir.spec_version != COMM_IR_SPEC_VERSION_V1 {
        return Err(PlcBridgeError {
            kind: PlcBridgeErrorKind::CommIrUnsupportedSpecVersion,
            message: format!("unsupported CommIR specVersion: {}", ir.spec_version),
            details: Some(PlcBridgeErrorDetails {
                ir_path: Some(ir_path.to_string_lossy().to_string()),
                schema_version: Some(ir.schema_version),
                spec_version: Some(ir.spec_version.clone()),
                ..Default::default()
            }),
        });
    }

    let mut seen: HashSet<uuid::Uuid> = HashSet::new();
    for p in &ir.mapping.points {
        if !seen.insert(p.point_key) {
            return Err(PlcBridgeError {
                kind: PlcBridgeErrorKind::CommIrValidationError,
                message: "duplicate pointKey detected in CommIR".to_string(),
                details: Some(PlcBridgeErrorDetails {
                    ir_path: Some(ir_path.to_string_lossy().to_string()),
                    point_key: Some(p.point_key.to_string()),
                    field: Some("pointKey".to_string()),
                    ..Default::default()
                }),
            });
        }

        if p.hmi_name.trim().is_empty() {
            return Err(PlcBridgeError {
                kind: PlcBridgeErrorKind::CommIrValidationError,
                message: "hmiName is empty".to_string(),
                details: Some(PlcBridgeErrorDetails {
                    ir_path: Some(ir_path.to_string_lossy().to_string()),
                    point_key: Some(p.point_key.to_string()),
                    hmi_name: Some(p.hmi_name.clone()),
                    field: Some("hmiName".to_string()),
                    ..Default::default()
                }),
            });
        }

        if matches!(p.data_type, DataType::Unknown) {
            return Err(PlcBridgeError {
                kind: PlcBridgeErrorKind::CommIrValidationError,
                message: "dataType is Unknown (not allowed in bridge v1)".to_string(),
                details: Some(PlcBridgeErrorDetails {
                    ir_path: Some(ir_path.to_string_lossy().to_string()),
                    point_key: Some(p.point_key.to_string()),
                    hmi_name: Some(p.hmi_name.clone()),
                    channel_name: Some(p.channel_name.clone()),
                    field: Some("dataType".to_string()),
                    raw_value: Some("Unknown".to_string()),
                    allowed_values: Some(vec![
                        "Bool".to_string(),
                        "Int16".to_string(),
                        "UInt16".to_string(),
                        "Int32".to_string(),
                        "UInt32".to_string(),
                        "Float32".to_string(),
                    ]),
                    ..Default::default()
                }),
            });
        }

        if matches!(p.endian, ByteOrder32::Unknown) {
            return Err(PlcBridgeError {
                kind: PlcBridgeErrorKind::CommIrValidationError,
                message: "endian is Unknown (not allowed in bridge v1)".to_string(),
                details: Some(PlcBridgeErrorDetails {
                    ir_path: Some(ir_path.to_string_lossy().to_string()),
                    point_key: Some(p.point_key.to_string()),
                    hmi_name: Some(p.hmi_name.clone()),
                    channel_name: Some(p.channel_name.clone()),
                    field: Some("endian".to_string()),
                    raw_value: Some("Unknown".to_string()),
                    allowed_values: Some(vec![
                        "ABCD".to_string(),
                        "BADC".to_string(),
                        "CDAB".to_string(),
                        "DCBA".to_string(),
                    ]),
                    ..Default::default()
                }),
            });
        }

        let area = p.address_spec.read_area.clone();
        match area {
            Some(RegisterArea::Holding) | Some(RegisterArea::Coil) => {}
            Some(other) => {
                return Err(PlcBridgeError {
                    kind: PlcBridgeErrorKind::CommIrValidationError,
                    message: "readArea must be Holding/Coil for MVP".to_string(),
                    details: Some(PlcBridgeErrorDetails {
                        ir_path: Some(ir_path.to_string_lossy().to_string()),
                        point_key: Some(p.point_key.to_string()),
                        hmi_name: Some(p.hmi_name.clone()),
                        channel_name: Some(p.channel_name.clone()),
                        field: Some("addressSpec.readArea".to_string()),
                        raw_value: Some(format!("{other:?}")),
                        allowed_values: Some(vec!["Holding".to_string(), "Coil".to_string()]),
                        ..Default::default()
                    }),
                });
            }
            None => {
                return Err(PlcBridgeError {
                    kind: PlcBridgeErrorKind::CommIrValidationError,
                    message: "readArea missing (profile not resolved?)".to_string(),
                    details: Some(PlcBridgeErrorDetails {
                        ir_path: Some(ir_path.to_string_lossy().to_string()),
                        point_key: Some(p.point_key.to_string()),
                        hmi_name: Some(p.hmi_name.clone()),
                        channel_name: Some(p.channel_name.clone()),
                        field: Some("addressSpec.readArea".to_string()),
                        allowed_values: Some(vec!["Holding".to_string(), "Coil".to_string()]),
                        ..Default::default()
                    }),
                });
            }
        }
    }

    Ok(())
}

fn build_verification_index(ir: &CommIrV1) -> Result<HashMap<uuid::Uuid, CommIrV1Result>, PlcBridgeError> {
    let mut out = HashMap::new();
    for r in &ir.verification.results {
        if out.insert(r.point_key, r.clone()).is_some() {
            return Err(PlcBridgeError {
                kind: PlcBridgeErrorKind::CommIrValidationError,
                message: "duplicate verification result pointKey detected in CommIR".to_string(),
                details: Some(PlcBridgeErrorDetails {
                    point_key: Some(r.point_key.to_string()),
                    field: Some("verification.results.pointKey".to_string()),
                    ..Default::default()
                }),
            });
        }
    }
    Ok(out)
}

fn map_point(ir_point: &CommIrV1Point, result: Option<&CommIrV1Result>, fallback_ts: DateTime<Utc>) -> PlcImportBridgeV1Point {
    let verification = if let Some(r) = result {
        PlcImportBridgeV1PointVerification {
            quality: r.quality.clone(),
            value_display: r.value_display.clone(),
            timestamp: r.timestamp,
            message: r.message.clone(),
        }
    } else {
        PlcImportBridgeV1PointVerification {
            quality: Quality::ConfigError,
            value_display: "".to_string(),
            timestamp: fallback_ts,
            message: "missing result for pointKey".to_string(),
        }
    };

    PlcImportBridgeV1Point {
        name: ir_point.hmi_name.clone(),
        comm: PlcImportBridgeV1PointComm {
            channel_name: ir_point.channel_name.clone(),
            address_spec: ir_point.address_spec.clone(),
            data_type: ir_point.data_type.clone(),
            endian: ir_point.endian.clone(),
            scale: ir_point.scale,
            rw: ir_point.rw.clone(),
        },
        verification,
    }
}

pub fn export_plc_import_bridge_v1(ir_path: &Path, out_path: &Path) -> Result<PlcImportBridgeExportOutcome, PlcBridgeError> {
    let ir_text = std::fs::read_to_string(ir_path).map_err(|e| PlcBridgeError {
        kind: PlcBridgeErrorKind::CommIrReadError,
        message: e.to_string(),
        details: Some(PlcBridgeErrorDetails {
            ir_path: Some(ir_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    let ir_bytes = ir_text.as_bytes();
    let source_ir_digest = sha256_prefixed_bytes(ir_bytes);

    let ir: CommIrV1 = serde_json::from_str(&ir_text).map_err(|e| PlcBridgeError {
        kind: PlcBridgeErrorKind::CommIrDeserializeError,
        message: e.to_string(),
        details: Some(PlcBridgeErrorDetails {
            ir_path: Some(ir_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    validate_ir_points(ir_path, &ir)?;
    let verification_index = build_verification_index(&ir)?;

    // 输出确定性（TASK-35）：points 顺序严格保持为 CommIR.mapping.points 的原始顺序（冻结规则）。
    let points: Vec<PlcImportBridgeV1Point> = ir
        .mapping
        .points
        .iter()
        .map(|p| map_point(p, verification_index.get(&p.point_key), ir.generated_at_utc))
        .collect();

    let stats = compute_stats_from_qualities(points.iter().map(|p| p.verification.quality.clone()));
    let now = Utc::now();

    let bridge = PlcImportBridgeV1 {
        schema_version: SCHEMA_VERSION_V1,
        spec_version: PLC_IMPORT_BRIDGE_SPEC_VERSION_V1.to_string(),
        generated_at_utc: now,
        source_ir_path: ir_path.to_string_lossy().to_string(),
        source_ir_digest: Some(source_ir_digest.clone()),
        sources: PlcImportBridgeV1Sources {
            union_xlsx_path: ir.sources.union_xlsx_path.clone(),
            results_source: ir.sources.results_source,
        },
        points,
        statistics: stats.clone(),
    };

    let json_text = serde_json::to_string_pretty(&bridge).map_err(|e| PlcBridgeError {
        kind: PlcBridgeErrorKind::PlcBridgeWriteError,
        message: format!("serialize bridge json failed: {e}"),
        details: None,
    })?;
    let bridge_digest = sha256_prefixed_bytes(json_text.as_bytes());

    write_text_atomic(out_path, &json_text).map_err(|e| PlcBridgeError {
        kind: PlcBridgeErrorKind::PlcBridgeWriteError,
        message: e.to_string(),
        details: None,
    })?;

    Ok(PlcImportBridgeExportOutcome {
        out_path: out_path.to_path_buf(),
        summary: PlcImportBridgeExportSummary {
            points: stats.total,
            stats,
            source_ir_digest,
            plc_bridge_digest: bridge_digest,
        },
    })
}

pub fn consume_bridge_and_write_summary(
    bridge_path: &Path,
    out_dir: &Path,
) -> Result<BridgeConsumerOutcome, BridgeCheckError> {
    let text = std::fs::read_to_string(bridge_path).map_err(|e| BridgeCheckError {
        kind: BridgeCheckErrorKind::PlcBridgeReadError,
        message: e.to_string(),
        details: Some(BridgeCheckErrorDetails {
            bridge_path: Some(bridge_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    let bridge: PlcImportBridgeV1 = serde_json::from_str(&text).map_err(|e| BridgeCheckError {
        kind: BridgeCheckErrorKind::PlcBridgeDeserializeError,
        message: e.to_string(),
        details: Some(BridgeCheckErrorDetails {
            bridge_path: Some(bridge_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    if bridge.schema_version != SCHEMA_VERSION_V1 {
        return Err(BridgeCheckError {
            kind: BridgeCheckErrorKind::PlcBridgeUnsupportedSchemaVersion,
            message: format!("unsupported PlcImportBridge schemaVersion: {}", bridge.schema_version),
            details: Some(BridgeCheckErrorDetails {
                bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                schema_version: Some(bridge.schema_version),
                spec_version: Some(bridge.spec_version.clone()),
                ..Default::default()
            }),
        });
    }
    if bridge.spec_version != PLC_IMPORT_BRIDGE_SPEC_VERSION_V1 {
        return Err(BridgeCheckError {
            kind: BridgeCheckErrorKind::PlcBridgeUnsupportedSpecVersion,
            message: format!("unsupported PlcImportBridge specVersion: {}", bridge.spec_version),
            details: Some(BridgeCheckErrorDetails {
                bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                schema_version: Some(bridge.schema_version),
                spec_version: Some(bridge.spec_version.clone()),
                ..Default::default()
            }),
        });
    }

    let mut by_channel: BTreeMap<String, u32> = BTreeMap::new();
    let mut by_quality: BTreeMap<String, u32> = BTreeMap::new();
    for p in &bridge.points {
        if p.name.trim().is_empty() {
            return Err(BridgeCheckError {
                kind: BridgeCheckErrorKind::PlcBridgeValidationError,
                message: "point name is empty".to_string(),
                details: Some(BridgeCheckErrorDetails {
                    bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                    message: Some("point name is empty".to_string()),
                    ..Default::default()
                }),
            });
        }
        *by_channel.entry(p.comm.channel_name.clone()).or_insert(0) += 1;
        *by_quality.entry(format!("{:?}", p.verification.quality)).or_insert(0) += 1;
    }

    let mut first10: Vec<BridgeConsumerSummaryPoint> = Vec::new();
    for p in bridge.points.iter().take(10) {
        first10.push(BridgeConsumerSummaryPoint {
            name: p.name.clone(),
            channel_name: p.comm.channel_name.clone(),
            read_area: p.comm.address_spec.read_area.clone(),
            absolute_address: p.comm.address_spec.absolute_address,
        });
    }

    let summary = BridgeConsumerSummary {
        schema_version: SCHEMA_VERSION_V1,
        spec_version: PLC_IMPORT_BRIDGE_SPEC_VERSION_V1.to_string(),
        generated_at_utc: Utc::now(),
        bridge_path: bridge_path.to_string_lossy().to_string(),
        total_points: bridge.points.len() as u32,
        by_channel,
        by_quality,
        first10,
    };

    std::fs::create_dir_all(out_dir).map_err(|e| BridgeCheckError {
        kind: BridgeCheckErrorKind::BridgeSummaryWriteError,
        message: e.to_string(),
        details: Some(BridgeCheckErrorDetails {
            bridge_path: Some(bridge_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    let out_path = out_dir.join("summary.json");
    let json_text = serde_json::to_string_pretty(&summary).map_err(|e| BridgeCheckError {
        kind: BridgeCheckErrorKind::BridgeSummaryWriteError,
        message: e.to_string(),
        details: Some(BridgeCheckErrorDetails {
            bridge_path: Some(bridge_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    write_text_atomic(&out_path, &json_text).map_err(|e| BridgeCheckError {
        kind: BridgeCheckErrorKind::BridgeSummaryWriteError,
        message: e.to_string(),
        details: Some(BridgeCheckErrorDetails {
            bridge_path: Some(bridge_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    Ok(BridgeConsumerOutcome { out_path, summary })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::export_ir;
    use crate::comm::model::{ByteOrder32, CommPoint, ConnectionProfile, PointsV1, ProfilesV1, RegisterArea, SampleResult};

    fn make_valid_ir(tmp_dir: &Path) -> PathBuf {
        let profiles = ProfilesV1 {
            schema_version: 1,
            profiles: vec![ConnectionProfile::Tcp {
                channel_name: "ch1".to_string(),
                device_id: 1,
                read_area: RegisterArea::Holding,
                start_address: 0,
                length: 10,
                ip: "127.0.0.1".to_string(),
                port: 502,
                timeout_ms: 1000,
                retry_count: 0,
                poll_interval_ms: 500,
            }],
        };
        let points = PointsV1 {
            schema_version: 1,
            points: vec![CommPoint {
                point_key: uuid::Uuid::from_u128(1),
                hmi_name: "P1".to_string(),
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                channel_name: "ch1".to_string(),
                address_offset: Some(1),
                scale: 1.0,
            }],
        };
        let results = vec![SampleResult {
            point_key: uuid::Uuid::from_u128(1),
            value_display: "1".to_string(),
            quality: Quality::Ok,
            timestamp: Utc::now(),
            duration_ms: 1,
            error_message: "".to_string(),
        }];

        let out_dir = tmp_dir.join("ir");
        let outcome = export_ir::export_comm_ir_v1(
            &out_dir,
            &points,
            &profiles,
            None,
            CommIrResultsSource::RunLatest,
            &results,
            None,
            None,
            None,
        )
        .unwrap();
        outcome.ir_path
    }

    #[test]
    fn bridge_export_writes_file_and_contains_points_and_stats() {
        let base = std::env::temp_dir().join(format!("plc-codeforge-bridge-{}", uuid::Uuid::new_v4()));
        let ir_path = make_valid_ir(&base);
        let out_path = base.join("bridge").join("plc_import_bridge.v1.test.json");
        let outcome = export_plc_import_bridge_v1(&ir_path, &out_path).unwrap();

        assert!(outcome.out_path.exists());
        assert_eq!(outcome.summary.points, 1);
        assert!(outcome.summary.plc_bridge_digest.starts_with("sha256:"));

        let text = std::fs::read_to_string(&outcome.out_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json.get("schemaVersion").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(json.get("specVersion").and_then(|v| v.as_str()), Some("v1"));
        assert_eq!(json.pointer("/points/0/name").and_then(|v| v.as_str()), Some("P1"));
        assert!(json.pointer("/statistics/ok").is_some());
    }

    #[test]
    fn bridge_export_fails_on_duplicate_point_key() {
        let base = std::env::temp_dir().join(format!("plc-codeforge-bridge-dup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();

        let dup = uuid::Uuid::from_u128(42);
        let ir = CommIrV1 {
            schema_version: 1,
            spec_version: COMM_IR_SPEC_VERSION_V1.to_string(),
            generated_at_utc: Utc::now(),
            sources: export_ir::CommIrV1Sources {
                union_xlsx_path: None,
                results_source: CommIrResultsSource::RunLatest,
            },
            mapping: export_ir::CommIrV1Mapping {
                points: vec![
                    CommIrV1Point {
                        point_key: dup,
                        hmi_name: "A".to_string(),
                        channel_name: "ch1".to_string(),
                        data_type: DataType::UInt16,
                        endian: ByteOrder32::ABCD,
                        scale: 1.0,
                        rw: "R".to_string(),
                        address_spec: export_ir::CommIrV1AddressSpec {
                            read_area: Some(RegisterArea::Holding),
                            absolute_address: Some(0),
                            unit_length: Some(1),
                            profile_start_address: None,
                            profile_length: None,
                            offset_from_profile_start: None,
                            job_start_address: None,
                            job_length: None,
                            address_base: "zero".to_string(),
                        },
                    },
                    CommIrV1Point {
                        point_key: dup,
                        hmi_name: "B".to_string(),
                        channel_name: "ch1".to_string(),
                        data_type: DataType::UInt16,
                        endian: ByteOrder32::ABCD,
                        scale: 1.0,
                        rw: "R".to_string(),
                        address_spec: export_ir::CommIrV1AddressSpec {
                            read_area: Some(RegisterArea::Holding),
                            absolute_address: Some(1),
                            unit_length: Some(1),
                            profile_start_address: None,
                            profile_length: None,
                            offset_from_profile_start: None,
                            job_start_address: None,
                            job_length: None,
                            address_base: "zero".to_string(),
                        },
                    },
                ],
                profiles: vec![],
            },
            verification: export_ir::CommIrV1Verification {
                results: vec![],
                stats: RunStats {
                    total: 0,
                    ok: 0,
                    timeout: 0,
                    comm_error: 0,
                    decode_error: 0,
                    config_error: 0,
                },
            },
            decisions_summary: export_ir::CommIrDecisionsSummary::default(),
            conflicts: None,
        };

        let ir_path = base.join("ir.json");
        std::fs::write(&ir_path, serde_json::to_string_pretty(&ir).unwrap()).unwrap();
        let out_path = base.join("bridge.json");
        let err = export_plc_import_bridge_v1(&ir_path, &out_path).unwrap_err();
        assert_eq!(err.kind, PlcBridgeErrorKind::CommIrValidationError);
        assert!(err.message.contains("duplicate pointKey"));
        let dup_text = dup.to_string();
        assert_eq!(
            err.details.as_ref().and_then(|d| d.point_key.as_deref()),
            Some(dup_text.as_str())
        );

        // 便于 Docs/ExecResults 引用：落一份结构化错误样例文件。
        let err_path = base.join("bridge_error.json");
        let err_json = serde_json::to_string_pretty(&err).unwrap();
        std::fs::write(&err_path, err_json).unwrap();
    }

    #[test]
    fn consume_bridge_writes_summary_json() {
        let base = std::env::temp_dir().join(format!("plc-codeforge-bridge-consume-{}", uuid::Uuid::new_v4()));
        let ir_path = make_valid_ir(&base);
        let bridge_path = base.join("bridge").join("plc_import_bridge.v1.test.json");
        export_plc_import_bridge_v1(&ir_path, &bridge_path).unwrap();

        let out_dir = base.join("bridge_check").join("t1");
        let outcome = consume_bridge_and_write_summary(&bridge_path, &out_dir).unwrap();
        assert!(outcome.out_path.exists());
        assert_eq!(outcome.summary.total_points, 1);

        let text = std::fs::read_to_string(&outcome.out_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json.get("totalPoints").and_then(|v| v.as_u64()), Some(1));
        assert!(json.get("byChannel").is_some());
        assert!(json.get("byQuality").is_some());
    }

    #[test]
    fn bridge_export_matches_golden_fixture_ignoring_generated_at_and_source_path() {
        let ir_text = include_str!("fixtures/comm_ir.sample.v1.json");
        let expected_text = include_str!("fixtures/plc_import_bridge.expected.v1.json");

        let base = std::env::temp_dir().join(format!("plc-codeforge-bridge-golden-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        let ir_path = base.join("comm_ir.sample.v1.json");
        std::fs::write(&ir_path, ir_text).unwrap();
        let out_path = base.join("plc_import_bridge.actual.v1.json");

        let _ = export_plc_import_bridge_v1(&ir_path, &out_path).unwrap();

        let actual_json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out_path).unwrap()).unwrap();
        let mut expected_json: serde_json::Value = serde_json::from_str(expected_text).unwrap();

        fn normalize(mut v: serde_json::Value) -> serde_json::Value {
            if let Some(obj) = v.as_object_mut() {
                obj.remove("generatedAtUtc");
                obj.remove("sourceIrPath");
            }
            v
        }

        let actual_norm = normalize(actual_json);
        if let Some(obj) = expected_json.as_object_mut() {
            obj.insert("sourceIrPath".to_string(), serde_json::Value::String(ir_path.to_string_lossy().to_string()));
        }
        let expected_norm = normalize(expected_json);

        assert_eq!(actual_norm, expected_norm);
    }
}
