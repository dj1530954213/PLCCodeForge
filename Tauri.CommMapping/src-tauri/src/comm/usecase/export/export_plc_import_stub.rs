//! 通讯采集模块：UnifiedImport v1 → PLC Import stub（TASK-38）
//!
//! 约束：
//! - 不接入 plc_core，不调用 orchestrate，仅做“最小转换 + 校验 + 落盘”。
//! - plc_import_stub.v1 一旦对外使用即冻结：只允许新增可选字段，不得改名/删字段/改语义。
//! - 输出点位顺序必须稳定：按 UnifiedImport.points 原始顺序输出（确定性）。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

use crate::comm::core::model::{RegisterArea, SCHEMA_VERSION_V1};
use crate::comm::error::{
    UnifiedPlcImportStubError, UnifiedPlcImportStubErrorDetails, UnifiedPlcImportStubErrorKind,
};
use crate::comm::usecase::bridge::bridge_plc_import::{
    PlcImportBridgeV1PointComm, PlcImportBridgeV1PointVerification,
};
use crate::comm::usecase::merge_unified_import::{
    UnifiedImportV1, UnifiedImportV1PointDesign, UNIFIED_IMPORT_SPEC_VERSION_V1,
};

pub const PLC_IMPORT_STUB_SPEC_VERSION_V1: &str = "v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportStubV1Point {
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
pub struct PlcImportStubV1Statistics {
    pub points: u32,
    pub comm_covered: u32,
    pub verified: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportStubV1 {
    pub schema_version: u32,
    pub spec_version: String,
    pub generated_at_utc: DateTime<Utc>,
    pub source_unified_import_path: String,
    pub source_unified_import_digest: String,
    pub points: Vec<PlcImportStubV1Point>,
    pub device_groups: Vec<JsonValue>,
    pub hardware: JsonValue,
    pub statistics: PlcImportStubV1Statistics,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlcImportStubExportSummary {
    pub points: u32,
    pub comm_covered: u32,
    pub verified: u32,
    pub source_unified_import_digest: String,
    pub plc_import_stub_digest: String,
}

#[derive(Clone, Debug)]
pub struct PlcImportStubExportOutcome {
    pub out_path: PathBuf,
    pub summary: PlcImportStubExportSummary,
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

fn validate_unified(
    unified_path: &Path,
    unified: &UnifiedImportV1,
) -> Result<(), UnifiedPlcImportStubError> {
    if unified.schema_version != SCHEMA_VERSION_V1 {
        return Err(UnifiedPlcImportStubError {
            kind: UnifiedPlcImportStubErrorKind::UnifiedImportUnsupportedSchemaVersion,
            message: format!(
                "unsupported UnifiedImport schemaVersion: {}",
                unified.schema_version
            ),
            details: Some(UnifiedPlcImportStubErrorDetails {
                unified_import_path: Some(unified_path.to_string_lossy().to_string()),
                schema_version: Some(unified.schema_version),
                spec_version: Some(unified.spec_version.clone()),
                ..Default::default()
            }),
        });
    }
    if unified.spec_version != UNIFIED_IMPORT_SPEC_VERSION_V1 {
        return Err(UnifiedPlcImportStubError {
            kind: UnifiedPlcImportStubErrorKind::UnifiedImportUnsupportedSpecVersion,
            message: format!(
                "unsupported UnifiedImport specVersion: {}",
                unified.spec_version
            ),
            details: Some(UnifiedPlcImportStubErrorDetails {
                unified_import_path: Some(unified_path.to_string_lossy().to_string()),
                schema_version: Some(unified.schema_version),
                spec_version: Some(unified.spec_version.clone()),
                ..Default::default()
            }),
        });
    }

    let mut names: HashSet<String> = HashSet::new();
    for p in &unified.points {
        let name = p.name.trim();
        if name.is_empty() {
            return Err(UnifiedPlcImportStubError {
                kind: UnifiedPlcImportStubErrorKind::UnifiedImportValidationError,
                message: "points.name is empty".to_string(),
                details: Some(UnifiedPlcImportStubErrorDetails {
                    unified_import_path: Some(unified_path.to_string_lossy().to_string()),
                    field: Some("points.name".to_string()),
                    ..Default::default()
                }),
            });
        }
        if !names.insert(name.to_string()) {
            return Err(UnifiedPlcImportStubError {
                kind: UnifiedPlcImportStubErrorKind::UnifiedImportValidationError,
                message: "duplicate points.name detected".to_string(),
                details: Some(UnifiedPlcImportStubErrorDetails {
                    unified_import_path: Some(unified_path.to_string_lossy().to_string()),
                    name: Some(name.to_string()),
                    field: Some("points.name".to_string()),
                    ..Default::default()
                }),
            });
        }

        // MVP：仅允许 Holding/Coil；底层结构已预留 Input/Discrete，未来扩展需 bump specVersion。
        if let Some(comm) = &p.comm {
            match comm.address_spec.read_area {
                Some(RegisterArea::Holding) | Some(RegisterArea::Coil) | None => {}
                Some(RegisterArea::Input) => {
                    return Err(UnifiedPlcImportStubError {
                        kind: UnifiedPlcImportStubErrorKind::UnifiedImportValidationError,
                        message: "readArea=Input is not supported in plc_import_stub.v1 (MVP)"
                            .to_string(),
                        details: Some(UnifiedPlcImportStubErrorDetails {
                            unified_import_path: Some(unified_path.to_string_lossy().to_string()),
                            name: Some(name.to_string()),
                            field: Some("points.comm.addressSpec.readArea".to_string()),
                            raw_value: Some("Input".to_string()),
                            allowed_values: Some(vec!["Holding".to_string(), "Coil".to_string()]),
                            ..Default::default()
                        }),
                    });
                }
                Some(RegisterArea::Discrete) => {
                    return Err(UnifiedPlcImportStubError {
                        kind: UnifiedPlcImportStubErrorKind::UnifiedImportValidationError,
                        message: "readArea=Discrete is not supported in plc_import_stub.v1 (MVP)"
                            .to_string(),
                        details: Some(UnifiedPlcImportStubErrorDetails {
                            unified_import_path: Some(unified_path.to_string_lossy().to_string()),
                            name: Some(name.to_string()),
                            field: Some("points.comm.addressSpec.readArea".to_string()),
                            raw_value: Some("Discrete".to_string()),
                            allowed_values: Some(vec!["Holding".to_string(), "Coil".to_string()]),
                            ..Default::default()
                        }),
                    });
                }
            }
        }
    }

    Ok(())
}

pub fn export_plc_import_stub_v1(
    unified_import_path: &Path,
    out_path: &Path,
) -> Result<PlcImportStubExportOutcome, UnifiedPlcImportStubError> {
    let unified_text =
        std::fs::read_to_string(unified_import_path).map_err(|e| UnifiedPlcImportStubError {
            kind: UnifiedPlcImportStubErrorKind::UnifiedImportReadError,
            message: e.to_string(),
            details: Some(UnifiedPlcImportStubErrorDetails {
                unified_import_path: Some(unified_import_path.to_string_lossy().to_string()),
                ..Default::default()
            }),
        })?;
    let unified_digest = sha256_prefixed_bytes(unified_text.as_bytes());

    let unified: UnifiedImportV1 =
        serde_json::from_str(&unified_text).map_err(|e| UnifiedPlcImportStubError {
            kind: UnifiedPlcImportStubErrorKind::UnifiedImportDeserializeError,
            message: e.to_string(),
            details: Some(UnifiedPlcImportStubErrorDetails {
                unified_import_path: Some(unified_import_path.to_string_lossy().to_string()),
                ..Default::default()
            }),
        })?;
    validate_unified(unified_import_path, &unified)?;

    let mut comm_covered = 0u32;
    let mut verified = 0u32;
    let points: Vec<PlcImportStubV1Point> = unified
        .points
        .iter()
        .map(|p| {
            if p.comm.is_some() {
                comm_covered += 1;
            }
            if p.verification.is_some() {
                verified += 1;
            }
            PlcImportStubV1Point {
                name: p.name.clone(),
                design: p.design.clone(),
                comm: p.comm.clone(),
                verification: p.verification.clone(),
            }
        })
        .collect();

    let now = Utc::now();
    let stub = PlcImportStubV1 {
        schema_version: SCHEMA_VERSION_V1,
        spec_version: PLC_IMPORT_STUB_SPEC_VERSION_V1.to_string(),
        generated_at_utc: now,
        source_unified_import_path: unified_import_path.to_string_lossy().to_string(),
        source_unified_import_digest: unified_digest.clone(),
        points,
        device_groups: unified.device_groups.clone(),
        hardware: unified.hardware.clone(),
        statistics: PlcImportStubV1Statistics {
            points: unified.points.len() as u32,
            comm_covered,
            verified,
        },
    };

    let text = serde_json::to_string_pretty(&stub).map_err(|e| UnifiedPlcImportStubError {
        kind: UnifiedPlcImportStubErrorKind::PlcImportStubWriteError,
        message: e.to_string(),
        details: Some(UnifiedPlcImportStubErrorDetails {
            unified_import_path: Some(unified_import_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;
    let stub_digest = sha256_prefixed_bytes(text.as_bytes());

    write_text_atomic(out_path, &text).map_err(|e| UnifiedPlcImportStubError {
        kind: UnifiedPlcImportStubErrorKind::PlcImportStubWriteError,
        message: e.to_string(),
        details: Some(UnifiedPlcImportStubErrorDetails {
            unified_import_path: Some(unified_import_path.to_string_lossy().to_string()),
            ..Default::default()
        }),
    })?;

    Ok(PlcImportStubExportOutcome {
        out_path: out_path.to_path_buf(),
        summary: PlcImportStubExportSummary {
            points: unified.points.len() as u32,
            comm_covered,
            verified,
            source_unified_import_digest: unified_digest,
            plc_import_stub_digest: stub_digest,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::export_ir::CommIrV1AddressSpec;
    use crate::comm::merge_unified_import::{
        UnifiedImportV1Point, UnifiedImportV1Source, UnifiedImportV1Sources,
        UnifiedImportV1Statistics,
    };
    use crate::comm::model::{ByteOrder32, DataType, Quality};
    use uuid::Uuid;

    #[test]
    fn unified_to_plc_import_stub_preserves_points_order() {
        let dir = std::env::temp_dir().join(format!("plc-codeforge-plc-import-{}", Uuid::new_v4()));
        let unified_path = dir.join("unified_import.v1.test.json");
        let out_path = dir.join("plc_import.v1.test.json");

        let now = Utc::now();
        let unified = UnifiedImportV1 {
            schema_version: SCHEMA_VERSION_V1,
            spec_version: UNIFIED_IMPORT_SPEC_VERSION_V1.to_string(),
            generated_at_utc: now,
            sources: UnifiedImportV1Sources {
                union_xlsx: UnifiedImportV1Source {
                    path: "union.xlsx".to_string(),
                    digest: Some("sha256:union".to_string()),
                },
                comm_stub: UnifiedImportV1Source {
                    path: "stub.json".to_string(),
                    digest: Some("sha256:stub".to_string()),
                },
            },
            points: vec![
                UnifiedImportV1Point {
                    name: "A".to_string(),
                    design: None,
                    comm: None,
                    verification: None,
                },
                UnifiedImportV1Point {
                    name: "B".to_string(),
                    design: None,
                    comm: Some(PlcImportBridgeV1PointComm {
                        channel_name: "ch".to_string(),
                        address_spec: CommIrV1AddressSpec {
                            read_area: Some(RegisterArea::Holding),
                            absolute_address: Some(0),
                            unit_length: Some(1),
                            profile_start_address: Some(0),
                            profile_length: Some(10),
                            offset_from_profile_start: Some(0),
                            job_start_address: Some(0),
                            job_length: Some(10),
                            address_base: "zero".to_string(),
                        },
                        data_type: DataType::Int16,
                        endian: ByteOrder32::ABCD,
                        scale: 1.0,
                        rw: "RO".to_string(),
                    }),
                    verification: Some(PlcImportBridgeV1PointVerification {
                        quality: Quality::Ok,
                        value_display: "1".to_string(),
                        timestamp: Utc::now(),
                        message: "".to_string(),
                    }),
                },
            ],
            device_groups: Vec::new(),
            hardware: JsonValue::Object(serde_json::Map::new()),
            statistics: UnifiedImportV1Statistics {
                union_points: 2,
                stub_points: 1,
                matched: 1,
                unmatched_stub: 0,
                overridden: 1,
                conflicts: 0,
            },
        };
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            &unified_path,
            serde_json::to_string_pretty(&unified).unwrap(),
        )
        .unwrap();

        let outcome = export_plc_import_stub_v1(&unified_path, &out_path).unwrap();
        assert_eq!(outcome.summary.points, 2);
        assert_eq!(outcome.summary.comm_covered, 1);
        assert_eq!(outcome.summary.verified, 1);

        let stub_text = std::fs::read_to_string(&out_path).unwrap();
        let stub: PlcImportStubV1 = serde_json::from_str(&stub_text).unwrap();
        let names: Vec<String> = stub.points.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["A".to_string(), "B".to_string()]);
    }
}
