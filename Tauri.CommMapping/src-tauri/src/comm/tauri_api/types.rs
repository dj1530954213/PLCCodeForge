use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::comm::core::model::{
    CommExportDiagnostics, CommProjectV1, CommWarning, PointsV1, ProfilesV1, RunStats,
    SampleResult,
};
use crate::comm::core::plan::{PlanOptions, ReadPlan};
use crate::comm::{
    bridge_importresult_stub, bridge_plc_import, export_ir, export_plc_import_stub,
};
use crate::comm::usecase::{evidence_pack, import_union_xlsx, merge_unified_import};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommPingResponse {
    pub ok: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommProjectCreateRequest {
    pub name: String,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommProjectsListRequest {
    #[serde(default)]
    pub include_deleted: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommProjectsListResponse {
    pub projects: Vec<CommProjectV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommProjectCopyRequest {
    pub project_id: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommPlanBuildRequest {
    #[serde(default)]
    pub options: Option<PlanOptions>,
    #[serde(default)]
    pub profiles: Option<ProfilesV1>,
    #[serde(default)]
    pub points: Option<PointsV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanV1 {
    pub schema_version: u32,
    #[serde(flatten)]
    pub plan: ReadPlan,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CommDriverKind {
    Tcp,
    Rtu485,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunStartRequest {
    #[serde(default)]
    pub driver: Option<CommDriverKind>,
    #[serde(default)]
    pub profiles: Option<ProfilesV1>,
    #[serde(default)]
    pub points: Option<PointsV1>,
    #[serde(default)]
    pub plan: Option<ReadPlan>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunStartResponse {
    pub run_id: Uuid,
}

/// Run 启动：结构化可观测返回（用于 UI 稳定展示；不依赖 reject）。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunStartObsResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::CommRunError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunLatestResponse {
    pub results: Vec<SampleResult>,
    pub stats: RunStats,
    pub updated_at_utc: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_warnings: Option<Vec<CommWarning>>,
}

/// Run latest：结构化可观测返回（用于 UI 稳定展示；不依赖 reject）。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunLatestObsResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<CommRunLatestResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::CommRunError>,
}

/// Run stop：结构化可观测返回（用于 UI 稳定展示；不依赖 reject）。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunStopObsResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::CommRunError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportXlsxRequest {
    pub out_path: String,
    #[serde(default)]
    pub profiles: Option<ProfilesV1>,
    #[serde(default)]
    pub points: Option<PointsV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportXlsxHeaders {
    pub tcp_sheet: Vec<String>,
    pub rtu485_sheet: Vec<String>,
    pub params_sheet: Vec<String>,

    // 兼容冻结验收口径：headers.tcp/rtu/params
    #[serde(default)]
    pub tcp: Vec<String>,
    #[serde(default)]
    pub rtu: Vec<String>,
    #[serde(default)]
    pub params: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportXlsxResponse {
    pub out_path: String,
    pub headers: CommExportXlsxHeaders,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<CommWarning>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<CommExportDiagnostics>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportDeliveryXlsxRequest {
    pub out_path: String,
    #[serde(default)]
    pub include_results: Option<bool>,
    #[serde(default)]
    pub results_source: Option<DeliveryResultsSource>,
    /// 当 resultsSource=runLatest 时，由前端传入 latest results（避免后端读取 AppData）。
    #[serde(default)]
    pub results: Option<Vec<SampleResult>>,
    #[serde(default)]
    pub stats: Option<RunStats>,
    #[serde(default)]
    pub profiles: Option<ProfilesV1>,
    #[serde(default)]
    pub points: Option<PointsV1>,
}

/// 交付导出中 Results sheet 的来源策略。
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeliveryResultsSource {
    Appdata,
    RunLatest,
}

/// 交付导出中 Results sheet 的写入状态（可解释缺失策略）。
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeliveryResultsStatus {
    Written,
    Missing,
    Skipped,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportDeliveryXlsxHeaders {
    pub tcp: Vec<String>,
    pub rtu: Vec<String>,
    pub params: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportDeliveryXlsxResponse {
    pub out_path: String,
    pub headers: CommExportDeliveryXlsxHeaders,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results_status: Option<DeliveryResultsStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<CommWarning>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<CommExportDiagnostics>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportIrV1Request {
    /// 可选：来源联合表路径（用于 IR 可追溯）。
    #[serde(default)]
    pub union_xlsx_path: Option<String>,
    /// 可选：覆盖 resultsSource（默认：有 latestResults 则 runLatest，否则 appdata）。
    #[serde(default)]
    pub results_source: Option<export_ir::CommIrResultsSource>,
    #[serde(default)]
    pub profiles: Option<ProfilesV1>,
    #[serde(default)]
    pub points: Option<PointsV1>,
    /// 可选：latest results（若提供则写入 verification.results）。
    #[serde(default)]
    pub latest_results: Option<Vec<SampleResult>>,
    /// 可选：latest stats（若未提供则由 results 推导）。
    #[serde(default)]
    pub stats: Option<RunStats>,
    /// 可选：来自前端 mapper 的 decisions（用于 decisionsSummary 统计）。
    #[serde(default)]
    pub decisions: Option<JsonValue>,
    /// 可选：来自前端 mapper 的 conflictReport（会写入 IR.conflicts 精简版/或原样写入）。
    #[serde(default)]
    pub conflict_report: Option<JsonValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommExportIrV1Response {
    pub ir_path: String,
    pub summary: export_ir::CommIrExportSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommBridgeToPlcImportV1Request {
    pub ir_path: String,
    #[serde(default)]
    pub out_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommBridgeToPlcImportV1Response {
    pub out_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<bridge_plc_import::PlcImportBridgeExportSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::PlcBridgeError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommBridgeConsumeCheckRequest {
    pub bridge_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommBridgeConsumeCheckResponse {
    pub out_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<bridge_plc_import::BridgeConsumerSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::BridgeCheckError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommBridgeExportImportResultStubV1Request {
    pub bridge_path: String,
    #[serde(default)]
    pub out_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommBridgeExportImportResultStubV1Response {
    pub out_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<bridge_importresult_stub::ImportResultStubExportSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::ImportResultStubError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommMergeImportSourcesV1Request {
    pub union_xlsx_path: String,
    pub import_result_stub_path: String,
    #[serde(default)]
    pub out_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommMergeImportSourcesV1Response {
    pub out_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<merge_unified_import::MergeImportSourcesSummary>,
    #[serde(default)]
    pub warnings: Vec<CommWarning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::MergeImportSourcesError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommUnifiedExportPlcImportStubV1Request {
    pub unified_import_path: String,
    #[serde(default)]
    pub out_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommUnifiedExportPlcImportStubV1Response {
    pub out_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<export_plc_import_stub::PlcImportStubExportSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::UnifiedPlcImportStubError>,
}

pub use evidence_pack::{
    CommEvidencePackRequest, CommEvidencePackResponse, CommEvidenceVerifyV1Response,
    EvidenceVerifyCheck, EvidenceVerifyError, EvidenceVerifyErrorDetails, EvidenceVerifyErrorKind,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommImportUnionXlsxResponse {
    pub points: PointsV1,
    pub profiles: ProfilesV1,
    #[serde(default)]
    pub warnings: Vec<CommWarning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<import_union_xlsx::ImportUnionDiagnostics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::comm::error::ImportUnionError>,
}
