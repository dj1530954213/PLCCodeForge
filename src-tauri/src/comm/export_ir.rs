//! 通讯采集模块：统一中间数据 IR（CommIR v1）
//!
//! 设计目标（TASK-31）：
//! - 生成跨模块可消费的稳定 JSON（PLC 自动编辑 / UIA 自动化）
//! - 字段冻结：一旦对外使用，只允许新增可选字段，不得改名/删字段/改语义

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

use super::model::{ConnectionProfile, DataType, PointsV1, ProfilesV1, Quality, RegisterArea, RunStats, SampleResult, SCHEMA_VERSION_V1};
use super::plan::{build_read_plan, PlanOptions, ReadPlan};

pub const COMM_IR_SPEC_VERSION_V1: &str = "v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CommIrResultsSource {
    Appdata,
    RunLatest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrV1Sources {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub union_xlsx_path: Option<String>,
    pub results_source: CommIrResultsSource,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrV1AddressSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_area: Option<RegisterArea>,
    /// 内部 0-based 的绝对地址（寄存器/线圈单位）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absolute_address: Option<u16>,
    /// 点位占用的寄存器/线圈数量。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_length: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_start_address: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_length: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_from_profile_start: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_start_address: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_length: Option<u16>,
    /// 说明内部地址语义：统一为 0-based。
    pub address_base: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrV1Point {
    pub point_key: uuid::Uuid,
    pub hmi_name: String,
    pub channel_name: String,
    pub data_type: DataType,
    /// 32-bit 的字节序（对 Bool/16-bit 可忽略，但字段仍保留以便统一消费）。
    pub endian: super::model::ByteOrder32,
    pub scale: f64,
    /// R/W 语义（MVP 目前仅采集读取）。
    pub rw: String,
    pub address_spec: CommIrV1AddressSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrV1Mapping {
    pub points: Vec<CommIrV1Point>,
    pub profiles: Vec<ConnectionProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrV1Result {
    pub point_key: uuid::Uuid,
    pub value_display: String,
    pub quality: Quality,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u32,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrV1Verification {
    pub results: Vec<CommIrV1Result>,
    pub stats: RunStats,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrDecisionsSummary {
    pub reused_key_v2: u32,
    pub reused_key_v2_no_device: u32,
    pub reused_key_v1: u32,
    pub created_new: u32,
    pub conflicts: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrV1 {
    pub schema_version: u32,
    pub spec_version: String,
    pub generated_at_utc: DateTime<Utc>,
    pub sources: CommIrV1Sources,
    pub mapping: CommIrV1Mapping,
    pub verification: CommIrV1Verification,
    pub decisions_summary: CommIrDecisionsSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<JsonValue>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommIrExportSummary {
    pub points: u32,
    pub profiles: u32,
    pub results: u32,
    pub conflicts: u32,
    pub ir_digest: String,
}

#[derive(Clone, Debug)]
pub struct CommIrExportOutcome {
    pub ir_path: PathBuf,
    pub ir_digest: String,
    pub summary: CommIrExportSummary,
}

fn sha256_digest_prefixed(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    format!("sha256:{hex}")
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    std::fs::rename(tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn plan_lookup(plan: &ReadPlan) -> std::collections::HashMap<uuid::Uuid, (super::plan::ReadJob, super::plan::PlannedPointRead)> {
    let mut out = std::collections::HashMap::new();
    for job in &plan.jobs {
        for p in &job.points {
            out.insert(p.point_key, (job.clone(), p.clone()));
        }
    }
    out
}

fn profile_channel_name(profile: &ConnectionProfile) -> &str {
    match profile {
        ConnectionProfile::Tcp { channel_name, .. } => channel_name,
        ConnectionProfile::Rtu485 { channel_name, .. } => channel_name,
    }
}

fn profile_read_area(profile: &ConnectionProfile) -> RegisterArea {
    match profile {
        ConnectionProfile::Tcp { read_area, .. } => read_area.clone(),
        ConnectionProfile::Rtu485 { read_area, .. } => read_area.clone(),
    }
}

fn profile_start_address(profile: &ConnectionProfile) -> u16 {
    match profile {
        ConnectionProfile::Tcp { start_address, .. } => *start_address,
        ConnectionProfile::Rtu485 { start_address, .. } => *start_address,
    }
}

fn profile_total_length(profile: &ConnectionProfile) -> u16 {
    match profile {
        ConnectionProfile::Tcp { length, .. } => *length,
        ConnectionProfile::Rtu485 { length, .. } => *length,
    }
}

fn unit_length_for_point(profile_area: RegisterArea, data_type: &DataType) -> Option<u16> {
    match (profile_area, data_type) {
        (RegisterArea::Coil | RegisterArea::Discrete, DataType::Bool) => Some(1),
        (RegisterArea::Holding | RegisterArea::Input, DataType::Int16 | DataType::UInt16) => Some(1),
        (RegisterArea::Holding | RegisterArea::Input, DataType::Int32 | DataType::UInt32 | DataType::Float32) => Some(2),
        _ => None,
    }
}

fn build_decisions_summary(decisions: Option<&JsonValue>, conflict_report: Option<&JsonValue>) -> CommIrDecisionsSummary {
    let mut out = CommIrDecisionsSummary::default();
    if let Some(JsonValue::Array(items)) = decisions {
        for item in items {
            if let Some(v) = item.get("reuseDecision").and_then(|v| v.as_str()) {
                match v {
                    "reused:keyV2" => out.reused_key_v2 += 1,
                    "reused:keyV2NoDevice" => out.reused_key_v2_no_device += 1,
                    "reused:keyV1" => out.reused_key_v1 += 1,
                    "created:new" => out.created_new += 1,
                    _ => {}
                }
            }
        }
    }

    if let Some(v) = conflict_report
        .and_then(|v| v.get("conflicts"))
        .and_then(|v| v.as_array())
    {
        out.conflicts = v.len() as u32;
    }

    out
}

fn compute_stats_from_results(results: &[CommIrV1Result]) -> RunStats {
    let mut stats = RunStats {
        total: results.len() as u32,
        ok: 0,
        timeout: 0,
        comm_error: 0,
        decode_error: 0,
        config_error: 0,
    };

    for r in results {
        match r.quality {
            Quality::Ok => stats.ok += 1,
            Quality::Timeout => stats.timeout += 1,
            Quality::CommError => stats.comm_error += 1,
            Quality::DecodeError => stats.decode_error += 1,
            Quality::ConfigError => stats.config_error += 1,
        }
    }

    stats
}

pub fn export_comm_ir_v1(
    out_dir: &Path,
    points: &PointsV1,
    profiles: &ProfilesV1,
    union_xlsx_path: Option<String>,
    results_source: CommIrResultsSource,
    results: &[SampleResult],
    stats: Option<&RunStats>,
    decisions: Option<&JsonValue>,
    conflict_report: Option<&JsonValue>,
) -> Result<CommIrExportOutcome, String> {
    if points.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!("unsupported schemaVersion: {}", points.schema_version));
    }
    if profiles.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!("unsupported schemaVersion: {}", profiles.schema_version));
    }

    let now = Utc::now();

    let plan = build_read_plan(&profiles.profiles, &points.points, PlanOptions::default()).ok();
    let plan_index = plan.as_ref().map(plan_lookup);

    let mut profile_index: std::collections::HashMap<&str, &ConnectionProfile> = std::collections::HashMap::new();
    for p in &profiles.profiles {
        profile_index.insert(profile_channel_name(p), p);
    }

    let mapped_points: Vec<CommIrV1Point> = points
        .points
        .iter()
        .map(|p| {
            let prof = profile_index.get(p.channel_name.as_str()).copied();
            let planned = plan_index
                .as_ref()
                .and_then(|idx| idx.get(&p.point_key))
                .cloned();

            let (job, planned_point) = planned
                .map(|(j, r)| (Some(j), Some(r)))
                .unwrap_or((None, None));

            let profile_area = prof.map(profile_read_area);
            let profile_start = prof.map(profile_start_address);
            let profile_len = prof.map(profile_total_length);

            let absolute_address = if let (Some(j), Some(r)) = (&job, &planned_point) {
                Some(j.start_address.saturating_add(r.offset))
            } else if let (Some(start), Some(offset)) = (profile_start, p.address_offset) {
                Some(start.saturating_add(offset))
            } else {
                None
            };

            let unit_length = if let Some(r) = &planned_point {
                Some(r.length)
            } else if let Some(area) = profile_area.clone() {
                unit_length_for_point(area, &p.data_type)
            } else {
                None
            };

            let offset_from_profile_start = match (absolute_address, profile_start) {
                (Some(abs), Some(base)) if abs >= base => Some(abs - base),
                _ => None,
            };

            CommIrV1Point {
                point_key: p.point_key,
                hmi_name: p.hmi_name.clone(),
                channel_name: p.channel_name.clone(),
                data_type: p.data_type.clone(),
                endian: p.byte_order.clone(),
                scale: p.scale,
                rw: "R".to_string(),
                address_spec: CommIrV1AddressSpec {
                    read_area: profile_area,
                    absolute_address,
                    unit_length,
                    profile_start_address: profile_start,
                    profile_length: profile_len,
                    offset_from_profile_start,
                    job_start_address: job.as_ref().map(|v| v.start_address),
                    job_length: job.as_ref().map(|v| v.length),
                    address_base: "zero".to_string(),
                },
            }
        })
        .collect();

    let verification_results: Vec<CommIrV1Result> = results
        .iter()
        .map(|r| CommIrV1Result {
            point_key: r.point_key,
            value_display: r.value_display.clone(),
            quality: r.quality.clone(),
            timestamp: r.timestamp,
            duration_ms: r.duration_ms,
            message: r.error_message.clone(),
        })
        .collect();

    let verification_stats = stats
        .cloned()
        .unwrap_or_else(|| compute_stats_from_results(&verification_results));

    let decisions_summary = build_decisions_summary(decisions, conflict_report);

    let ir = CommIrV1 {
        schema_version: SCHEMA_VERSION_V1,
        spec_version: COMM_IR_SPEC_VERSION_V1.to_string(),
        generated_at_utc: now,
        sources: CommIrV1Sources {
            union_xlsx_path,
            results_source,
        },
        mapping: CommIrV1Mapping {
            points: mapped_points,
            profiles: profiles.profiles.clone(),
        },
        verification: CommIrV1Verification {
            results: verification_results,
            stats: verification_stats.clone(),
        },
        decisions_summary: decisions_summary.clone(),
        conflicts: conflict_report.cloned(),
    };

    let json_text = serde_json::to_string_pretty(&ir).map_err(|e| e.to_string())?;
    let digest = sha256_digest_prefixed(&json_text);

    let ts = format!("{}-{}", now.format("%Y%m%dT%H%M%SZ"), now.timestamp_millis());
    let file_name = format!("comm_ir.v1.{ts}.json");
    let ir_path = out_dir.join(file_name);
    write_text_atomic(&ir_path, &json_text)?;

    let summary = CommIrExportSummary {
        points: points.points.len() as u32,
        profiles: profiles.profiles.len() as u32,
        results: results.len() as u32,
        conflicts: decisions_summary.conflicts,
        ir_digest: digest.clone(),
    };

    Ok(CommIrExportOutcome {
        ir_path,
        ir_digest: digest,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::model::{ByteOrder32, CommPoint, ConnectionProfile, DataType, RegisterArea};

    #[test]
    fn export_ir_v1_writes_file_and_contains_required_fields() {
        let base = std::env::temp_dir().join(format!("plc-codeforge-ir-{}", uuid::Uuid::new_v4()));
        let out_dir = base.join("ir");

        let profiles = ProfilesV1 {
            schema_version: SCHEMA_VERSION_V1,
            profiles: vec![ConnectionProfile::Tcp {
                channel_name: "ch1".to_string(),
                device_id: 1,
                read_area: RegisterArea::Holding,
                start_address: 100,
                length: 10,
                ip: "127.0.0.1".to_string(),
                port: 502,
                timeout_ms: 1000,
                retry_count: 0,
                poll_interval_ms: 500,
            }],
        };

        let points = PointsV1 {
            schema_version: SCHEMA_VERSION_V1,
            points: vec![CommPoint {
                point_key: uuid::Uuid::from_u128(1),
                hmi_name: "P1".to_string(),
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                channel_name: "ch1".to_string(),
                address_offset: Some(2),
                scale: 1.0,
            }],
        };

        let results = vec![SampleResult {
            point_key: uuid::Uuid::from_u128(1),
            value_display: "123".to_string(),
            quality: Quality::Ok,
            timestamp: Utc::now(),
            duration_ms: 1,
            error_message: "".to_string(),
        }];

        let outcome = export_comm_ir_v1(
            &out_dir,
            &points,
            &profiles,
            Some("C:\\temp\\union.xlsx".to_string()),
            CommIrResultsSource::RunLatest,
            &results,
            None,
            None,
            None,
        )
        .unwrap();

        assert!(outcome.ir_path.exists());
        assert!(outcome.ir_digest.starts_with("sha256:"));

        let text = std::fs::read_to_string(&outcome.ir_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json.get("schemaVersion").and_then(|v| v.as_u64()), Some(1));
        assert!(json.get("generatedAtUtc").is_some());
        assert_eq!(
            json.pointer("/sources/resultsSource")
                .and_then(|v| v.as_str()),
            Some("runLatest")
        );
        assert_eq!(
            json.pointer("/mapping/points/0/hmiName")
                .and_then(|v| v.as_str()),
            Some("P1")
        );
        assert_eq!(
            json.pointer("/mapping/points/0/addressSpec/absoluteAddress")
                .and_then(|v| v.as_u64()),
            Some(102)
        );
        assert!(json.pointer("/verification/results/0/quality").is_some());
        assert!(json.pointer("/verification/stats/ok").is_some());
    }
}
