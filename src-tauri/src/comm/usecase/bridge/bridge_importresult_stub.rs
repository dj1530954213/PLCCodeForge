//! 通讯采集模块：PlcImportBridge v1 → “拟 ImportResult” Stub（TASK-36）
//!
//! 约束：
//! - 不接入 plc_core orchestrate、不加载模板、不做程序生成。
//! - Stub schema 一旦对外使用即冻结：只允许新增可选字段，不得改名/删字段/改语义。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

use super::bridge_plc_import::{PlcImportBridgeV1, PLC_IMPORT_BRIDGE_SPEC_VERSION_V1};
use crate::comm::core::model::{RunStats, SCHEMA_VERSION_V1};
use crate::comm::error::{
    ImportResultStubError, ImportResultStubErrorDetails, ImportResultStubErrorKind,
};

pub const IMPORT_RESULT_STUB_SPEC_VERSION_V1: &str = "v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultStubV1 {
    pub schema_version: u32,
    pub spec_version: String,
    pub generated_at_utc: DateTime<Utc>,
    pub source_bridge_path: String,
    pub source_bridge_digest: String,
    pub points: Vec<super::bridge_plc_import::PlcImportBridgeV1Point>,
    pub device_groups: Vec<JsonValue>,
    pub hardware: JsonValue,
    pub statistics: RunStats,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultStubExportSummary {
    pub points: u32,
    pub stats: RunStats,
    pub source_bridge_digest: String,
    pub import_result_stub_digest: String,
}

#[derive(Clone, Debug)]
pub struct ImportResultStubExportOutcome {
    pub out_path: PathBuf,
    pub summary: ImportResultStubExportSummary,
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

fn validate_bridge_for_stub(
    bridge_path: &Path,
    bridge: &PlcImportBridgeV1,
) -> Result<(), ImportResultStubError> {
    if bridge.schema_version != SCHEMA_VERSION_V1 {
        return Err(ImportResultStubError {
            kind: ImportResultStubErrorKind::PlcBridgeUnsupportedSchemaVersion,
            message: format!(
                "unsupported PlcImportBridge schemaVersion: {}",
                bridge.schema_version
            ),
            details: Some(ImportResultStubErrorDetails {
                bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                schema_version: Some(bridge.schema_version),
                spec_version: Some(bridge.spec_version.clone()),
                ..Default::default()
            }),
        });
    }
    if bridge.spec_version != PLC_IMPORT_BRIDGE_SPEC_VERSION_V1 {
        return Err(ImportResultStubError {
            kind: ImportResultStubErrorKind::PlcBridgeUnsupportedSpecVersion,
            message: format!(
                "unsupported PlcImportBridge specVersion: {}",
                bridge.spec_version
            ),
            details: Some(ImportResultStubErrorDetails {
                bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                schema_version: Some(bridge.schema_version),
                spec_version: Some(bridge.spec_version.clone()),
                ..Default::default()
            }),
        });
    }

    let mut names: HashSet<String> = HashSet::new();
    for p in &bridge.points {
        let name = p.name.trim();
        if name.is_empty() {
            return Err(ImportResultStubError {
                kind: ImportResultStubErrorKind::ImportResultStubValidationError,
                message: "point name is empty".to_string(),
                details: Some(ImportResultStubErrorDetails {
                    bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                    field: Some("points.name".to_string()),
                    ..Default::default()
                }),
            });
        }
        if !names.insert(name.to_string()) {
            return Err(ImportResultStubError {
                kind: ImportResultStubErrorKind::ImportResultStubValidationError,
                message: "duplicate points.name detected".to_string(),
                details: Some(ImportResultStubErrorDetails {
                    bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                    name: Some(name.to_string()),
                    field: Some("points.name".to_string()),
                    ..Default::default()
                }),
            });
        }

        if p.comm.channel_name.trim().is_empty() {
            return Err(ImportResultStubError {
                kind: ImportResultStubErrorKind::ImportResultStubValidationError,
                message: "comm.channelName is empty".to_string(),
                details: Some(ImportResultStubErrorDetails {
                    bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                    name: Some(name.to_string()),
                    field: Some("points.comm.channelName".to_string()),
                    ..Default::default()
                }),
            });
        }
    }

    Ok(())
}

pub fn export_import_result_stub_v1(
    bridge_path: &Path,
    out_path: &Path,
) -> Result<ImportResultStubExportOutcome, ImportResultStubError> {
    let bridge_text = std::fs::read_to_string(bridge_path).map_err(|e| ImportResultStubError {
        kind: ImportResultStubErrorKind::PlcBridgeReadError,
        message: e.to_string(),
        details: Some(ImportResultStubErrorDetails {
            bridge_path: Some(bridge_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;
    let bridge_digest = sha256_prefixed_bytes(bridge_text.as_bytes());

    let bridge: PlcImportBridgeV1 =
        serde_json::from_str(&bridge_text).map_err(|e| ImportResultStubError {
            kind: ImportResultStubErrorKind::PlcBridgeDeserializeError,
            message: e.to_string(),
            details: Some(ImportResultStubErrorDetails {
                bridge_path: Some(bridge_path.to_string_lossy().to_string()),
                ..Default::default()
            }),
        })?;

    validate_bridge_for_stub(bridge_path, &bridge)?;

    let now = Utc::now();
    let stub = ImportResultStubV1 {
        schema_version: SCHEMA_VERSION_V1,
        spec_version: IMPORT_RESULT_STUB_SPEC_VERSION_V1.to_string(),
        generated_at_utc: now,
        source_bridge_path: bridge_path.to_string_lossy().to_string(),
        source_bridge_digest: bridge_digest.clone(),
        points: bridge.points.clone(),
        device_groups: Vec::new(),
        hardware: JsonValue::Object(serde_json::Map::new()),
        statistics: bridge.statistics.clone(),
    };

    let json_text = serde_json::to_string_pretty(&stub).map_err(|e| ImportResultStubError {
        kind: ImportResultStubErrorKind::ImportResultStubWriteError,
        message: format!("serialize stub json failed: {e}"),
        details: Some(ImportResultStubErrorDetails {
            bridge_path: Some(bridge_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;
    let stub_digest = sha256_prefixed_bytes(json_text.as_bytes());

    write_text_atomic(out_path, &json_text).map_err(|e| ImportResultStubError {
        kind: ImportResultStubErrorKind::ImportResultStubWriteError,
        message: e.to_string(),
        details: Some(ImportResultStubErrorDetails {
            bridge_path: Some(bridge_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    Ok(ImportResultStubExportOutcome {
        out_path: out_path.to_path_buf(),
        summary: ImportResultStubExportSummary {
            points: stub.statistics.total,
            stats: stub.statistics,
            source_bridge_digest: bridge_digest,
            import_result_stub_digest: stub_digest,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::bridge_plc_import::{
        consume_bridge_and_write_summary, export_plc_import_bridge_v1,
    };
    use crate::comm::export_ir;
    use crate::comm::model::{
        ByteOrder32, CommPoint, ConnectionProfile, DataType, PointsV1, ProfilesV1, Quality,
        RegisterArea, SampleResult,
    };

    fn make_bridge(tmp_dir: &Path) -> PathBuf {
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

        let ir_out_dir = tmp_dir.join("ir");
        let ir_outcome = export_ir::export_comm_ir_v1(
            &ir_out_dir,
            &points,
            &profiles,
            None,
            export_ir::CommIrResultsSource::RunLatest,
            &results,
            None,
            None,
            None,
        )
        .unwrap();

        let bridge_path = tmp_dir
            .join("bridge")
            .join("plc_import_bridge.v1.test.json");
        export_plc_import_bridge_v1(&ir_outcome.ir_path, &bridge_path).unwrap();

        // 额外调用一次 consumer，确保与 TASK-34 的链路相容（不作为断言重点）
        let _ = consume_bridge_and_write_summary(
            &bridge_path,
            &tmp_dir.join("bridge_check").join("t1"),
        )
        .unwrap();

        bridge_path
    }

    #[test]
    fn export_stub_writes_file_and_has_required_sections() {
        let base =
            std::env::temp_dir().join(format!("plc-codeforge-stub-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        let bridge_path = make_bridge(&base);

        let out_path = base.join("stub").join("import_result_stub.v1.test.json");
        let outcome = export_import_result_stub_v1(&bridge_path, &out_path).unwrap();

        assert!(outcome.out_path.exists());
        assert!(outcome
            .summary
            .import_result_stub_digest
            .starts_with("sha256:"));

        let text = std::fs::read_to_string(&outcome.out_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json.get("schemaVersion").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(json.get("specVersion").and_then(|v| v.as_str()), Some("v1"));
        assert!(json.get("deviceGroups").is_some());
        assert!(json.get("hardware").is_some());
        assert!(json.get("statistics").is_some());
        assert!(json.pointer("/points/0/name").is_some());
    }

    #[test]
    fn export_stub_fails_on_duplicate_name() {
        let base =
            std::env::temp_dir().join(format!("plc-codeforge-stub-dup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();

        let bridge = PlcImportBridgeV1 {
            schema_version: 1,
            spec_version: PLC_IMPORT_BRIDGE_SPEC_VERSION_V1.to_string(),
            generated_at_utc: Utc::now(),
            source_ir_path: "x".to_string(),
            source_ir_digest: Some("sha256:0".to_string()),
            sources: super::super::bridge_plc_import::PlcImportBridgeV1Sources {
                union_xlsx_path: None,
                results_source: export_ir::CommIrResultsSource::RunLatest,
            },
            points: vec![
                super::super::bridge_plc_import::PlcImportBridgeV1Point {
                    name: "DUP".to_string(),
                    comm: super::super::bridge_plc_import::PlcImportBridgeV1PointComm {
                        channel_name: "ch1".to_string(),
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
                        data_type: DataType::UInt16,
                        endian: ByteOrder32::ABCD,
                        scale: 1.0,
                        rw: "R".to_string(),
                    },
                    verification:
                        super::super::bridge_plc_import::PlcImportBridgeV1PointVerification {
                            quality: Quality::Ok,
                            value_display: "1".to_string(),
                            timestamp: Utc::now(),
                            message: "".to_string(),
                        },
                },
                super::super::bridge_plc_import::PlcImportBridgeV1Point {
                    name: "DUP".to_string(),
                    comm: super::super::bridge_plc_import::PlcImportBridgeV1PointComm {
                        channel_name: "ch1".to_string(),
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
                        data_type: DataType::UInt16,
                        endian: ByteOrder32::ABCD,
                        scale: 1.0,
                        rw: "R".to_string(),
                    },
                    verification:
                        super::super::bridge_plc_import::PlcImportBridgeV1PointVerification {
                            quality: Quality::Ok,
                            value_display: "2".to_string(),
                            timestamp: Utc::now(),
                            message: "".to_string(),
                        },
                },
            ],
            statistics: RunStats {
                total: 2,
                ok: 2,
                timeout: 0,
                comm_error: 0,
                decode_error: 0,
                config_error: 0,
            },
        };

        let bridge_path = base.join("bridge.json");
        std::fs::write(&bridge_path, serde_json::to_string_pretty(&bridge).unwrap()).unwrap();
        let out_path = base.join("stub.json");
        let err = export_import_result_stub_v1(&bridge_path, &out_path).unwrap_err();
        assert_eq!(
            err.kind,
            ImportResultStubErrorKind::ImportResultStubValidationError
        );
        assert!(err.message.contains("duplicate"));
        assert_eq!(
            err.details.as_ref().and_then(|d| d.name.as_deref()),
            Some("DUP")
        );

        // 便于 Docs/ExecResults 引用：落一份结构化错误样例文件。
        let err_path = base.join("stub_error.json");
        let err_json = serde_json::to_string_pretty(&err).unwrap();
        std::fs::write(&err_path, err_json).unwrap();
    }
}
