//! Tauri command 层（冻结契约入口）。
//!
//! 注意：一旦 DTO/命令在这里对外暴露，即视为稳定契约（后续只允许新增可选字段）。
//!
//! 硬约束（来自 Docs/通讯数据采集验证/执行要求.md）：
//! - `comm_run_start` 只能 spawn 后台任务；不得在 command 内循环采集
//! - `comm_run_latest` 只读缓存，不触发采集
//! - `comm_run_stop` 必须在 1s 内生效（MVP）
//! - DTO 契约冻结：只允许新增可选字段，不得改名/删字段/改语义

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use super::bridge_importresult_stub;
use super::bridge_plc_import;
use super::driver::modbus_rtu::ModbusRtuDriver;
use super::driver::modbus_tcp::ModbusTcpDriver;
use super::driver::CommDriver;
use super::engine::CommRunEngine;
use super::error::{
    BridgeCheckError, BridgeCheckErrorKind, CommRunError, CommRunErrorDetails, CommRunErrorKind,
    ImportResultStubError, ImportResultStubErrorKind, ImportUnionError, ImportUnionErrorDetails,
    ImportUnionErrorKind, MergeImportSourcesError, MergeImportSourcesErrorKind, PlcBridgeError,
    PlcBridgeErrorKind, UnifiedPlcImportStubError, UnifiedPlcImportStubErrorKind,
};
use super::export_delivery_xlsx;
use super::export_ir;
use super::export_plc_import_stub;
use super::export_xlsx::export_comm_address_xlsx;
use super::import_union_xlsx;
use super::merge_unified_import;
use super::model::{
    CommConfigV1, CommExportDiagnostics, CommProjectDataV1, CommProjectUiStateV1, CommProjectV1,
    CommWarning, ConnectionProfile, PointsV1, ProfilesV1, RunStats, SampleResult,
    SCHEMA_VERSION_V1,
};
use super::path_resolver;
use super::plan::{build_read_plan, PlanOptions, ReadPlan};
use super::storage;
use super::union_spec_v1;
use super::usecase::evidence_pack;
use super::usecase::run_validation;
use crate::comm::adapters::storage::projects;

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
    pub error: Option<CommRunError>,
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
    pub error: Option<CommRunError>,
}

/// Run stop：结构化可观测返回（用于 UI 稳定展示；不依赖 reject）。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunStopObsResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CommRunError>,
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
    pub error: Option<PlcBridgeError>,
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
    pub error: Option<BridgeCheckError>,
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
    pub error: Option<ImportResultStubError>,
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
    pub error: Option<MergeImportSourcesError>,
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
    pub error: Option<UnifiedPlcImportStubError>,
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
    pub error: Option<ImportUnionError>,
}

#[derive(Default, Clone)]
struct CommMemoryScope {
    profiles: Option<ProfilesV1>,
    points: Option<PointsV1>,
    plan: Option<ReadPlan>,
}

#[derive(Default)]
struct CommMemoryStore {
    scopes: HashMap<String, CommMemoryScope>,
}

impl CommMemoryStore {
    fn scope_mut(&mut self, key: &str) -> &mut CommMemoryScope {
        self.scopes.entry(key.to_string()).or_default()
    }

    fn scope(&self, key: &str) -> Option<&CommMemoryScope> {
        self.scopes.get(key)
    }
}

pub struct CommState {
    memory: Mutex<CommMemoryStore>,
    engine: CommRunEngine,
    tcp_driver: Arc<ModbusTcpDriver>,
    rtu_driver: Arc<ModbusRtuDriver>,
}

impl CommState {
    pub fn new() -> Self {
        Self {
            memory: Mutex::new(CommMemoryStore::default()),
            engine: CommRunEngine::new(),
            tcp_driver: Arc::new(ModbusTcpDriver::new()),
            rtu_driver: Arc::new(ModbusRtuDriver::new()),
        }
    }
}

#[tauri::command]
pub fn comm_ping() -> CommPingResponse {
    CommPingResponse { ok: true }
}

#[tauri::command]
pub async fn comm_project_create(
    app: AppHandle,
    request: CommProjectCreateRequest,
) -> Result<CommProjectV1, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        projects::create_project(&app_data_dir, request.name, request.device, request.notes)
    })
    .await
    .map_err(|e| format!("comm_project_create join error: {e}"))?
}

#[tauri::command]
pub async fn comm_projects_list(
    app: AppHandle,
    request: Option<CommProjectsListRequest>,
) -> Result<CommProjectsListResponse, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let include_deleted = request.and_then(|r| r.include_deleted).unwrap_or(false);

    tauri::async_runtime::spawn_blocking(move || {
        projects::list_projects(&app_data_dir, include_deleted)
    })
    .await
    .map_err(|e| format!("comm_projects_list join error: {e}"))?
    .map(|projects| CommProjectsListResponse { projects })
}

#[tauri::command]
pub async fn comm_project_get(
    app: AppHandle,
    project_id: String,
) -> Result<Option<CommProjectV1>, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || projects::load_project(&app_data_dir, &project_id))
        .await
        .map_err(|e| format!("comm_project_get join error: {e}"))?
}

#[tauri::command]
pub async fn comm_project_load_v1(
    app: AppHandle,
    project_id: String,
) -> Result<CommProjectDataV1, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        projects::load_project_data(&app_data_dir, &project_id)
    })
    .await
    .map_err(|e| format!("comm_project_load_v1 join error: {e}"))?
}

#[tauri::command]
pub async fn comm_project_save_v1(
    app: AppHandle,
    payload: CommProjectDataV1,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        projects::save_project_data(&app_data_dir, &payload)
    })
    .await
    .map_err(|e| format!("comm_project_save_v1 join error: {e}"))?
}

#[tauri::command]
pub async fn comm_project_ui_state_patch_v1(
    app: AppHandle,
    project_id: String,
    patch: CommProjectUiStateV1,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        let mut project = projects::load_project_data(&app_data_dir, &project_id)?;

        let mut ui = project.ui_state.unwrap_or_default();
        if patch.active_channel_name.is_some() {
            ui.active_channel_name = patch.active_channel_name;
        }
        if patch.points_batch_template.is_some() {
            ui.points_batch_template = patch.points_batch_template;
        }
        project.ui_state = Some(ui);

        projects::save_project_data(&app_data_dir, &project)?;
        Ok(())
    })
    .await
    .map_err(|e| format!("comm_project_ui_state_patch_v1 join error: {e}"))?
}

#[tauri::command]
pub async fn comm_project_copy(
    app: AppHandle,
    request: CommProjectCopyRequest,
) -> Result<CommProjectV1, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        projects::copy_project(&app_data_dir, &request.project_id, request.name)
    })
    .await
    .map_err(|e| format!("comm_project_copy join error: {e}"))?
}

#[tauri::command]
pub async fn comm_project_delete(
    app: AppHandle,
    project_id: String,
) -> Result<CommProjectV1, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        projects::soft_delete_project(&app_data_dir, &project_id)
    })
    .await
    .map_err(|e| format!("comm_project_delete join error: {e}"))?
}

#[tauri::command]
pub fn comm_config_load(
    app: AppHandle,
    project_id: Option<String>,
) -> Result<CommConfigV1, String> {
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let default_dir = storage::default_output_dir(&base_dir)
        .to_string_lossy()
        .to_string();

    match storage::load_config(&base_dir).map_err(|e| e.to_string())? {
        Some(mut cfg) => {
            if cfg.output_dir.trim().is_empty() {
                cfg.output_dir = default_dir;
            }
            Ok(cfg)
        }
        None => Ok(CommConfigV1 {
            schema_version: SCHEMA_VERSION_V1,
            output_dir: default_dir,
        }),
    }
}

#[tauri::command]
pub fn comm_config_save(
    app: AppHandle,
    payload: CommConfigV1,
    project_id: Option<String>,
) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let default_dir = storage::default_output_dir(&base_dir)
        .to_string_lossy()
        .to_string();

    let mut cfg = payload;
    if cfg.output_dir.trim().is_empty() {
        cfg.output_dir = default_dir;
    }

    storage::save_config(&base_dir, &cfg).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn comm_profiles_save(
    app: AppHandle,
    state: State<'_, CommState>,
    payload: ProfilesV1,
    project_id: Option<String>,
) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let scope = scope_key(project_id.as_deref());
    if let Some(project_id) = project_id.as_deref() {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let mut project = projects::load_project_data(&app_data_dir, project_id)?;
        project.connections = Some(payload.clone());
        projects::save_project_data(&app_data_dir, &project)?;
    } else {
        let base_dir = comm_base_dir(&app, None)?;
        storage::save_profiles(&base_dir, &payload).map_err(|e| e.to_string())?;
    }
    state.memory.lock().scope_mut(&scope).profiles = Some(payload);
    Ok(())
}

#[tauri::command]
pub fn comm_profiles_load(
    app: AppHandle,
    state: State<'_, CommState>,
    project_id: Option<String>,
) -> Result<ProfilesV1, String> {
    let scope = scope_key(project_id.as_deref());
    if let Some(project_id) = project_id.as_deref() {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let project = projects::load_project_data(&app_data_dir, project_id)?;
        let profiles = project.connections.unwrap_or(ProfilesV1 {
            schema_version: SCHEMA_VERSION_V1,
            profiles: vec![],
        });
        state.memory.lock().scope_mut(&scope).profiles = Some(profiles.clone());
        return Ok(profiles);
    }

    let base_dir = comm_base_dir(&app, None)?;
    let loaded = storage::load_profiles(&base_dir).map_err(|e| e.to_string())?;
    if let Some(v) = loaded {
        state.memory.lock().scope_mut(&scope).profiles = Some(v.clone());
        return Ok(v);
    }

    Ok(state
        .memory
        .lock()
        .scope(&scope)
        .and_then(|v| v.profiles.clone())
        .unwrap_or(ProfilesV1 {
            schema_version: SCHEMA_VERSION_V1,
            profiles: vec![],
        }))
}

#[tauri::command]
pub fn comm_points_save(
    app: AppHandle,
    state: State<'_, CommState>,
    payload: PointsV1,
    project_id: Option<String>,
) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let scope = scope_key(project_id.as_deref());
    if let Some(project_id) = project_id.as_deref() {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let mut project = projects::load_project_data(&app_data_dir, project_id)?;
        project.points = Some(payload.clone());
        projects::save_project_data(&app_data_dir, &project)?;
    } else {
        let base_dir = comm_base_dir(&app, None)?;
        storage::save_points(&base_dir, &payload).map_err(|e| e.to_string())?;
    }
    state.memory.lock().scope_mut(&scope).points = Some(payload);
    Ok(())
}

#[tauri::command]
pub fn comm_points_load(
    app: AppHandle,
    state: State<'_, CommState>,
    project_id: Option<String>,
) -> Result<PointsV1, String> {
    let scope = scope_key(project_id.as_deref());
    if let Some(project_id) = project_id.as_deref() {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let project = projects::load_project_data(&app_data_dir, project_id)?;
        let points = project.points.unwrap_or(PointsV1 {
            schema_version: SCHEMA_VERSION_V1,
            points: vec![],
        });
        state.memory.lock().scope_mut(&scope).points = Some(points.clone());
        return Ok(points);
    }

    let base_dir = comm_base_dir(&app, None)?;
    let loaded = storage::load_points(&base_dir).map_err(|e| e.to_string())?;
    if let Some(v) = loaded {
        state.memory.lock().scope_mut(&scope).points = Some(v.clone());
        return Ok(v);
    }

    Ok(state
        .memory
        .lock()
        .scope(&scope)
        .and_then(|v| v.points.clone())
        .unwrap_or(PointsV1 {
            schema_version: SCHEMA_VERSION_V1,
            points: vec![],
        }))
}

#[tauri::command]
pub fn comm_plan_build(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommPlanBuildRequest,
    project_id: Option<String>,
) -> Result<PlanV1, String> {
    let scope = scope_key(project_id.as_deref());
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let profiles = resolve_profiles(&base_dir, &state, &scope, request.profiles)?;
    let points = resolve_points(&base_dir, &state, &scope, request.points)?;

    let options = request.options.unwrap_or_default();
    let plan =
        build_read_plan(&profiles.profiles, &points.points, options).map_err(|e| e.to_string())?;

    state.memory.lock().scope_mut(&scope).plan = Some(plan.clone());
    storage::save_plan(&base_dir, &plan).map_err(|e| e.to_string())?;
    Ok(PlanV1 {
        schema_version: SCHEMA_VERSION_V1,
        plan,
    })
}

#[tauri::command]
pub async fn comm_run_start(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommRunStartRequest,
    project_id: Option<String>,
) -> Result<CommRunStartResponse, String> {
    let run_id = comm_run_start_inner(app, state, request, project_id)
        .await
        .map_err(|e| e.message)?;
    Ok(CommRunStartResponse { run_id })
}

#[tauri::command]
pub async fn comm_run_start_obs(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommRunStartRequest,
    project_id: Option<String>,
) -> Result<CommRunStartObsResponse, CommRunError> {
    let resp = match comm_run_start_inner(app, state, request, project_id.clone()).await {
        Ok(run_id) => CommRunStartObsResponse {
            ok: true,
            run_id: Some(run_id),
            error: None,
        },
        Err(err) => CommRunStartObsResponse {
            ok: false,
            run_id: None,
            error: Some(err),
        },
    };
    Ok(resp)
}

async fn comm_run_start_inner(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommRunStartRequest,
    project_id: Option<String>,
) -> Result<Uuid, CommRunError> {
    let scope = scope_key(project_id.as_deref());
    let base_dir = comm_base_dir(&app, project_id.as_deref())
        .map_err(|message| comm_run_error_from_message(message, None, project_id.clone()))?;
    let profiles = resolve_profiles(&base_dir, &state, &scope, request.profiles)
        .map_err(|message| comm_run_error_from_message(message, None, project_id.clone()))?;
    let points = resolve_points(&base_dir, &state, &scope, request.points)
        .map_err(|message| comm_run_error_from_message(message, None, project_id.clone()))?;

    if profiles.profiles.is_empty() {
        return Err(CommRunError {
            kind: CommRunErrorKind::ConfigError,
            message: "profiles is empty".to_string(),
            details: Some(CommRunErrorDetails {
                project_id,
                ..Default::default()
            }),
        });
    }

    if points.points.is_empty() {
        return Err(CommRunError {
            kind: CommRunErrorKind::ConfigError,
            message: "points is empty".to_string(),
            details: Some(CommRunErrorDetails {
                project_id,
                ..Default::default()
            }),
        });
    }

    let missing_fields = run_validation::validate_run_inputs(&profiles.profiles, &points.points);
    if !missing_fields.is_empty() {
        return Err(CommRunError {
            kind: CommRunErrorKind::ConfigError,
            message: "invalid points/profiles configuration".to_string(),
            details: Some(CommRunErrorDetails {
                project_id,
                missing_fields: Some(missing_fields),
                ..Default::default()
            }),
        });
    }

    let plan = match request.plan {
        Some(p) => p,
        None => {
            if let Some(saved) = state
                .memory
                .lock()
                .scope(&scope)
                .and_then(|v| v.plan.clone())
            {
                saved
            } else if let Some(saved) = storage::load_plan(&base_dir)
                .map_err(|e| comm_run_error_from_message(e.to_string(), None, project_id.clone()))?
            {
                state.memory.lock().scope_mut(&scope).plan = Some(saved.plan.clone());
                saved.plan
            } else {
                build_read_plan(&profiles.profiles, &points.points, PlanOptions::default())
                    .map_err(|e| {
                        comm_run_error_from_message(e.to_string(), None, project_id.clone())
                    })?
            }
        }
    };

    let driver_kind = match request.driver {
        Some(v) => v,
        None => infer_driver_kind_from_profiles(&profiles.profiles)
            .map_err(|message| comm_run_error_from_message(message, None, project_id.clone()))?,
    };
    if profiles_has_mismatched_protocol(&profiles.profiles, driver_kind.clone()) {
        return Err(CommRunError {
            kind: CommRunErrorKind::ConfigError,
            message: format!("driver={driver_kind:?} does not match profiles.protocolType"),
            details: Some(CommRunErrorDetails {
                project_id,
                ..Default::default()
            }),
        });
    }
    let driver: Arc<dyn CommDriver> = match driver_kind {
        CommDriverKind::Tcp => Arc::clone(&state.tcp_driver) as Arc<dyn CommDriver>,
        CommDriverKind::Rtu485 => Arc::clone(&state.rtu_driver) as Arc<dyn CommDriver>,
    };

    let poll_interval_ms = profiles_min_poll_interval_ms(&profiles.profiles).unwrap_or(1000);
    let run_id = state.engine.start_run(
        driver,
        profiles.profiles.clone(),
        points.points.clone(),
        plan,
        poll_interval_ms,
    );

    Ok(run_id)
}

#[tauri::command]
pub fn comm_run_latest(
    state: State<'_, CommState>,
    run_id: Uuid,
) -> Result<CommRunLatestResponse, String> {
    comm_run_latest_inner(state, run_id)
}

#[tauri::command]
pub fn comm_run_latest_obs(state: State<'_, CommState>, run_id: Uuid) -> CommRunLatestObsResponse {
    match comm_run_latest_inner(state, run_id) {
        Ok(value) => CommRunLatestObsResponse {
            ok: true,
            value: Some(value),
            error: None,
        },
        Err(message) => CommRunLatestObsResponse {
            ok: false,
            value: None,
            error: Some(comm_run_error_from_message(message, Some(run_id), None)),
        },
    }
}

fn comm_run_latest_inner(
    state: State<'_, CommState>,
    run_id: Uuid,
) -> Result<CommRunLatestResponse, String> {
    let Some((results, stats, updated_at_utc, run_warnings)) = state.engine.latest(run_id) else {
        return Err("run not found".to_string());
    };

    Ok(CommRunLatestResponse {
        results,
        stats,
        updated_at_utc,
        run_warnings: if run_warnings.is_empty() {
            None
        } else {
            Some(run_warnings)
        },
    })
}

#[tauri::command]
pub async fn comm_run_stop(
    app: AppHandle,
    state: State<'_, CommState>,
    run_id: Uuid,
    project_id: Option<String>,
) -> Result<(), String> {
    comm_run_stop_inner(app, state, run_id, project_id).await
}

#[tauri::command]
pub async fn comm_run_stop_obs(
    app: AppHandle,
    state: State<'_, CommState>,
    run_id: Uuid,
    project_id: Option<String>,
) -> Result<CommRunStopObsResponse, CommRunError> {
    let resp = match comm_run_stop_inner(app, state, run_id, project_id.clone()).await {
        Ok(()) => CommRunStopObsResponse {
            ok: true,
            error: None,
        },
        Err(message) => CommRunStopObsResponse {
            ok: false,
            error: Some(comm_run_error_from_message(
                message,
                Some(run_id),
                project_id,
            )),
        },
    };
    Ok(resp)
}

async fn comm_run_stop_inner(
    app: AppHandle,
    state: State<'_, CommState>,
    run_id: Uuid,
    project_id: Option<String>,
) -> Result<(), String> {
    let snapshot = state.engine.latest(run_id);
    let stopped = state.engine.stop_run(run_id).await;

    if !stopped {
        return Err("run not found".to_string());
    }

    if let Some((results, stats, _updated_at_utc, _run_warnings)) = snapshot {
        let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        let is_project = project_id.is_some();

        tauri::async_runtime::spawn_blocking(move || {
            if is_project {
                let _ = storage::save_run_last_results(&base_dir, run_id, &results, &stats);
                let _ = storage::save_last_results(&base_dir, &results, &stats);
            } else {
                let _ = storage::save_last_results(&base_dir, &results, &stats);
            }
        });
    }

    Ok(())
}

fn comm_run_error_kind_from_message(message: &str) -> CommRunErrorKind {
    let msg = message.to_ascii_lowercase();
    if msg.contains("run not found") {
        return CommRunErrorKind::RunNotFound;
    }
    if msg.contains("profiles")
        || msg.contains("points")
        || msg.contains("plan")
        || msg.contains("schemaversion")
        || msg.contains("projectid")
        || msg.contains("unsupported")
        || msg.contains("not provided")
    {
        return CommRunErrorKind::ConfigError;
    }
    CommRunErrorKind::InternalError
}

fn comm_run_error_from_message(
    message: String,
    run_id: Option<Uuid>,
    project_id: Option<String>,
) -> CommRunError {
    CommRunError {
        kind: comm_run_error_kind_from_message(&message),
        message,
        details: Some(CommRunErrorDetails {
            run_id: run_id.map(|v| v.to_string()),
            project_id,
            ..Default::default()
        }),
    }
}

#[tauri::command]
pub async fn comm_export_xlsx(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportXlsxRequest,
    project_id: Option<String>,
) -> Result<CommExportXlsxResponse, String> {
    let scope = scope_key(project_id.as_deref());
    let profiles_dir = comm_base_dir(&app, project_id.as_deref())?;
    let profiles = resolve_profiles(&profiles_dir, &state, &scope, request.profiles)?;
    let points = resolve_points(&profiles_dir, &state, &scope, request.points)?;

    let base_dir = profiles_dir;
    let out_path_text = request.out_path;
    let profiles_vec = profiles.profiles.clone();
    let points_vec = points.points.clone();
    let is_project = project_id.is_some();

    tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let out_path = if out_path_text.trim().is_empty() {
            if is_project {
                let ts = path_resolver::ts_label(now);
                projects::project_exports_dir(&base_dir).join(format!("通讯地址表.{ts}.xlsx"))
            } else {
                let output_dir = path_resolver::resolve_output_dir(&base_dir);
                path_resolver::default_delivery_xlsx_path(&output_dir, now)
            }
        } else {
            std::path::PathBuf::from(&out_path_text)
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let outcome = export_comm_address_xlsx(&out_path, &profiles_vec, &points_vec)
            .map_err(|e| e.to_string())?;

        let tcp = outcome.headers.tcp_sheet.clone();
        let rtu = outcome.headers.rtu485_sheet.clone();
        let params = outcome.headers.params_sheet.clone();
        let warnings = if outcome.warnings.is_empty() {
            None
        } else {
            Some(outcome.warnings)
        };

        Ok::<_, String>(CommExportXlsxResponse {
            out_path: out_path.to_string_lossy().to_string(),
            headers: CommExportXlsxHeaders {
                tcp_sheet: outcome.headers.tcp_sheet,
                rtu485_sheet: outcome.headers.rtu485_sheet,
                params_sheet: outcome.headers.params_sheet,
                tcp,
                rtu,
                params,
            },
            warnings,
            diagnostics: Some(outcome.diagnostics),
        })
    })
    .await
    .map_err(|e| format!("comm_export_xlsx join error: {e}"))?
}

#[tauri::command]
pub async fn comm_export_delivery_xlsx(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportDeliveryXlsxRequest,
    project_id: Option<String>,
) -> Result<CommExportDeliveryXlsxResponse, String> {
    let CommExportDeliveryXlsxRequest {
        out_path: out_path_text,
        include_results,
        results_source,
        results,
        stats,
        profiles: request_profiles,
        points: request_points,
    } = request;

    let scope = scope_key(project_id.as_deref());
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let profiles = resolve_profiles(&base_dir, &state, &scope, request_profiles)?;
    let points = resolve_points(&base_dir, &state, &scope, request_points)?;

    let include_results = include_results.unwrap_or(false);
    let results_source = results_source.unwrap_or(DeliveryResultsSource::Appdata);

    let profiles_vec = profiles.profiles.clone();
    let points_vec = points.points.clone();
    let is_project = project_id.is_some();

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let out_path = if out_path_text.trim().is_empty() {
            if is_project {
                let ts = path_resolver::ts_label(now);
                projects::project_exports_dir(&base_dir).join(format!("通讯地址表.{ts}.xlsx"))
            } else {
                let output_dir = path_resolver::resolve_output_dir(&base_dir);
                path_resolver::default_delivery_xlsx_path(&output_dir, now)
            }
        } else {
            std::path::PathBuf::from(&out_path_text)
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Results sheet 缺失策略（拍板，TASK-24）：
        // - includeResults=false：不生成 Results sheet（resultsStatus=skipped）
        // - includeResults=true & resultsSource=appdata：读取 AppData/comm/last_results.v1.json；不存在则 resultsStatus=missing
        // - includeResults=true & resultsSource=runLatest：由前端先调用 comm_run_latest 并把 results 作为参数传入；缺失/空则 resultsStatus=missing
        let mut results_status = DeliveryResultsStatus::Skipped;
        let mut results_message: Option<String> = None;
        let mut results_opt: Option<Vec<SampleResult>> = None;
        let mut stats_opt: Option<RunStats> = None;

        if include_results {
            match results_source {
                DeliveryResultsSource::Appdata => match storage::load_last_results(&base_dir) {
                    Ok(Some(v)) => {
                        results_status = DeliveryResultsStatus::Written;
                        results_message = Some("resultsSource=appdata: loaded last_results.v1.json".to_string());
                        results_opt = Some(v.results);
                        stats_opt = Some(v.stats);
                    }
                    Ok(None) => {
                        results_status = DeliveryResultsStatus::Missing;
                        results_message = Some(
                            "resultsSource=appdata: last_results.v1.json not found; Results sheet skipped"
                                .to_string(),
                        );
                    }
                    Err(e) => {
                        results_status = DeliveryResultsStatus::Missing;
                        results_message = Some(format!(
                            "resultsSource=appdata: failed to load last_results.v1.json; Results sheet skipped ({e})"
                        ));
                    }
                },
                DeliveryResultsSource::RunLatest => {
                    if let Some(r) = results {
                        if r.is_empty() {
                            results_status = DeliveryResultsStatus::Missing;
                            results_message = Some(
                                "resultsSource=runLatest: results payload is empty; Results sheet skipped"
                                    .to_string(),
                            );
                        } else {
                            results_status = DeliveryResultsStatus::Written;
                            results_message =
                                Some("resultsSource=runLatest: results provided by frontend".to_string());
                            results_opt = Some(r);
                            stats_opt = stats;
                        }
                    } else {
                        results_status = DeliveryResultsStatus::Missing;
                        results_message = Some(
                            "resultsSource=runLatest: results payload missing; Results sheet skipped"
                                .to_string(),
                        );
                    }
                }
            }
        }

        let include_results_effective =
            include_results && results_status == DeliveryResultsStatus::Written && results_opt.is_some();

        export_delivery_xlsx::export_delivery_xlsx(
            &out_path,
            &profiles_vec,
            &points_vec,
            include_results_effective,
            results_opt.as_deref(),
            stats_opt.as_ref(),
        )
        .map(|outcome| (out_path.to_string_lossy().to_string(), outcome, results_status, results_message))
    })
    .await
    .map_err(|e| format!("export_delivery_xlsx join error: {e}"))?
    .map_err(|e| e.to_string())?;

    let (out_path_actual_text, outcome, results_status, results_message) = outcome;
    let warnings = if outcome.warnings.is_empty() {
        None
    } else {
        Some(outcome.warnings)
    };

    Ok(CommExportDeliveryXlsxResponse {
        out_path: out_path_actual_text,
        headers: CommExportDeliveryXlsxHeaders {
            tcp: outcome.headers.tcp,
            rtu: outcome.headers.rtu,
            params: outcome.headers.params,
        },
        results_status: Some(results_status),
        results_message,
        warnings,
        diagnostics: Some(outcome.diagnostics),
    })
}

#[tauri::command]
pub async fn comm_export_ir_v1(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportIrV1Request,
    project_id: Option<String>,
) -> Result<CommExportIrV1Response, String> {
    let scope = scope_key(project_id.as_deref());
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let profiles = resolve_profiles(&base_dir, &state, &scope, request.profiles)?;
    let points = resolve_points(&base_dir, &state, &scope, request.points)?;

    let union_xlsx_path = request.union_xlsx_path;
    let decisions = request.decisions;
    let conflict_report = request.conflict_report;

    let results_source = request.results_source.unwrap_or_else(|| {
        if request.latest_results.is_some() {
            export_ir::CommIrResultsSource::RunLatest
        } else {
            export_ir::CommIrResultsSource::Appdata
        }
    });

    let latest_results = request.latest_results;
    let stats = request.stats;

    tauri::async_runtime::spawn_blocking(move || {
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_dir = path_resolver::ir_dir(&output_dir);

        let (results, stats_opt) = match results_source {
            export_ir::CommIrResultsSource::RunLatest => {
                (latest_results.unwrap_or_default(), stats)
            }
            export_ir::CommIrResultsSource::Appdata => {
                match storage::load_last_results(&base_dir) {
                    Ok(Some(v)) => (v.results, Some(v.stats)),
                    Ok(None) => (Vec::new(), None),
                    Err(_) => (Vec::new(), None),
                }
            }
        };

        let outcome = export_ir::export_comm_ir_v1(
            &out_dir,
            &points,
            &profiles,
            union_xlsx_path,
            results_source,
            &results,
            stats_opt.as_ref(),
            decisions.as_ref(),
            conflict_report.as_ref(),
        )?;

        Ok::<_, String>(CommExportIrV1Response {
            ir_path: outcome.ir_path.to_string_lossy().to_string(),
            summary: outcome.summary,
        })
    })
    .await
    .map_err(|e| format!("comm_export_ir_v1 join error: {e}"))?
}

#[tauri::command]
pub async fn comm_bridge_to_plc_import_v1(
    app: AppHandle,
    request: CommBridgeToPlcImportV1Request,
    project_id: Option<String>,
) -> CommBridgeToPlcImportV1Response {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommBridgeToPlcImportV1Response {
                out_path: "".to_string(),
                summary: None,
                ok: Some(false),
                error: Some(PlcBridgeError {
                    kind: PlcBridgeErrorKind::CommIrReadError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let ir_path_text = request.ir_path.trim().to_string();
    if ir_path_text.is_empty() {
        return CommBridgeToPlcImportV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(PlcBridgeError {
                kind: PlcBridgeErrorKind::CommIrReadError,
                message: "irPath is empty".to_string(),
                details: None,
            }),
        };
    }

    let out_path_override = request.out_path.unwrap_or_default();

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_path = if out_path_override.trim().is_empty() {
            path_resolver::default_plc_bridge_path(&output_dir, now)
        } else {
            std::path::PathBuf::from(&out_path_override)
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PlcBridgeError {
                kind: PlcBridgeErrorKind::PlcBridgeWriteError,
                message: e.to_string(),
                details: None,
            })?;
        }

        let ir_path = std::path::PathBuf::from(&ir_path_text);
        let outcome = bridge_plc_import::export_plc_import_bridge_v1(&ir_path, &out_path)?;

        Ok::<_, PlcBridgeError>(CommBridgeToPlcImportV1Response {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            summary: Some(outcome.summary),
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommBridgeToPlcImportV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommBridgeToPlcImportV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(PlcBridgeError {
                kind: PlcBridgeErrorKind::PlcBridgeWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_bridge_consume_check(
    app: AppHandle,
    request: CommBridgeConsumeCheckRequest,
    project_id: Option<String>,
) -> CommBridgeConsumeCheckResponse {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommBridgeConsumeCheckResponse {
                out_path: "".to_string(),
                summary: None,
                ok: Some(false),
                error: Some(BridgeCheckError {
                    kind: BridgeCheckErrorKind::PlcBridgeReadError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let bridge_path_text = request.bridge_path.trim().to_string();
    if bridge_path_text.is_empty() {
        return CommBridgeConsumeCheckResponse {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(BridgeCheckError {
                kind: BridgeCheckErrorKind::PlcBridgeReadError,
                message: "bridgePath is empty".to_string(),
                details: None,
            }),
        };
    }

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_dir = path_resolver::bridge_check_dir(&output_dir, now);

        let bridge_path = std::path::PathBuf::from(&bridge_path_text);
        let outcome = bridge_plc_import::consume_bridge_and_write_summary(&bridge_path, &out_dir)?;

        Ok::<_, BridgeCheckError>(CommBridgeConsumeCheckResponse {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            summary: Some(outcome.summary),
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommBridgeConsumeCheckResponse {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommBridgeConsumeCheckResponse {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(BridgeCheckError {
                kind: BridgeCheckErrorKind::BridgeSummaryWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_bridge_export_importresult_stub_v1(
    app: AppHandle,
    request: CommBridgeExportImportResultStubV1Request,
    project_id: Option<String>,
) -> CommBridgeExportImportResultStubV1Response {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommBridgeExportImportResultStubV1Response {
                out_path: "".to_string(),
                summary: None,
                ok: Some(false),
                error: Some(ImportResultStubError {
                    kind: ImportResultStubErrorKind::PlcBridgeReadError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let bridge_path_text = request.bridge_path.trim().to_string();
    if bridge_path_text.is_empty() {
        return CommBridgeExportImportResultStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(ImportResultStubError {
                kind: ImportResultStubErrorKind::PlcBridgeReadError,
                message: "bridgePath is empty".to_string(),
                details: None,
            }),
        };
    }

    let out_path_override = request.out_path.unwrap_or_default();

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_path = if out_path_override.trim().is_empty() {
            path_resolver::default_importresult_stub_path(&output_dir, now)
        } else {
            std::path::PathBuf::from(&out_path_override)
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ImportResultStubError {
                kind: ImportResultStubErrorKind::ImportResultStubWriteError,
                message: e.to_string(),
                details: None,
            })?;
        }

        let bridge_path = std::path::PathBuf::from(&bridge_path_text);
        let outcome =
            bridge_importresult_stub::export_import_result_stub_v1(&bridge_path, &out_path)?;

        Ok::<_, ImportResultStubError>(CommBridgeExportImportResultStubV1Response {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            summary: Some(outcome.summary),
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommBridgeExportImportResultStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommBridgeExportImportResultStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(ImportResultStubError {
                kind: ImportResultStubErrorKind::ImportResultStubWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_merge_import_sources_v1(
    app: AppHandle,
    request: CommMergeImportSourcesV1Request,
    project_id: Option<String>,
) -> CommMergeImportSourcesV1Response {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommMergeImportSourcesV1Response {
                out_path: "".to_string(),
                report_path: None,
                summary: None,
                warnings: Vec::new(),
                ok: Some(false),
                error: Some(MergeImportSourcesError {
                    kind: MergeImportSourcesErrorKind::MergeWriteError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let union_path_text = request.union_xlsx_path.trim().to_string();
    if union_path_text.is_empty() {
        return CommMergeImportSourcesV1Response {
            out_path: "".to_string(),
            report_path: None,
            summary: None,
            warnings: Vec::new(),
            ok: Some(false),
            error: Some(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::UnionXlsxReadError,
                message: "unionXlsxPath is empty".to_string(),
                details: None,
            }),
        };
    }

    let stub_path_text = request.import_result_stub_path.trim().to_string();
    if stub_path_text.is_empty() {
        return CommMergeImportSourcesV1Response {
            out_path: "".to_string(),
            report_path: None,
            summary: None,
            warnings: Vec::new(),
            ok: Some(false),
            error: Some(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::ImportResultStubReadError,
                message: "importResultStubPath is empty".to_string(),
                details: None,
            }),
        };
    }

    let out_path_override = request.out_path.unwrap_or_default();

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);

        let out_path = if out_path_override.trim().is_empty() {
            path_resolver::default_unified_import_path(&output_dir, now)
        } else {
            std::path::PathBuf::from(&out_path_override)
        };
        let report_path = path_resolver::default_merge_report_path(&output_dir, now);

        let union_path = std::path::PathBuf::from(&union_path_text);
        let stub_path = std::path::PathBuf::from(&stub_path_text);

        let outcome = merge_unified_import::merge_import_sources_v1(
            &union_path,
            &stub_path,
            &out_path,
            &report_path,
        )?;

        Ok::<_, MergeImportSourcesError>(CommMergeImportSourcesV1Response {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            report_path: Some(outcome.report_path.to_string_lossy().to_string()),
            summary: Some(outcome.summary),
            warnings: outcome.warnings,
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommMergeImportSourcesV1Response {
            out_path: "".to_string(),
            report_path: None,
            summary: None,
            warnings: Vec::new(),
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommMergeImportSourcesV1Response {
            out_path: "".to_string(),
            report_path: None,
            summary: None,
            warnings: Vec::new(),
            ok: Some(false),
            error: Some(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::MergeWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_unified_export_plc_import_stub_v1(
    app: AppHandle,
    request: CommUnifiedExportPlcImportStubV1Request,
    project_id: Option<String>,
) -> CommUnifiedExportPlcImportStubV1Response {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommUnifiedExportPlcImportStubV1Response {
                out_path: "".to_string(),
                summary: None,
                ok: Some(false),
                error: Some(UnifiedPlcImportStubError {
                    kind: UnifiedPlcImportStubErrorKind::PlcImportStubWriteError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let unified_path_text = request.unified_import_path.trim().to_string();
    if unified_path_text.is_empty() {
        return CommUnifiedExportPlcImportStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(UnifiedPlcImportStubError {
                kind: UnifiedPlcImportStubErrorKind::UnifiedImportReadError,
                message: "unifiedImportPath is empty".to_string(),
                details: None,
            }),
        };
    }

    let out_path_override = request.out_path.unwrap_or_default();

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);

        let out_path = if out_path_override.trim().is_empty() {
            path_resolver::default_plc_import_stub_path(&output_dir, now)
        } else {
            std::path::PathBuf::from(&out_path_override)
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| UnifiedPlcImportStubError {
                kind: UnifiedPlcImportStubErrorKind::PlcImportStubWriteError,
                message: e.to_string(),
                details: None,
            })?;
        }

        let unified_path = std::path::PathBuf::from(&unified_path_text);
        let outcome = export_plc_import_stub::export_plc_import_stub_v1(&unified_path, &out_path)?;

        Ok::<_, UnifiedPlcImportStubError>(CommUnifiedExportPlcImportStubV1Response {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            summary: Some(outcome.summary),
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommUnifiedExportPlcImportStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommUnifiedExportPlcImportStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(UnifiedPlcImportStubError {
                kind: UnifiedPlcImportStubErrorKind::PlcImportStubWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_evidence_pack_create(
    app: AppHandle,
    request: CommEvidencePackRequest,
    project_id: Option<String>,
) -> Result<CommEvidencePackResponse, String> {
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let output_dir = if project_id.is_some() {
        base_dir.clone()
    } else {
        path_resolver::resolve_output_dir(&base_dir)
    };
    let app_name = app.config().identifier.clone();
    let app_version = app.package_info().version.to_string();
    let git_commit = option_env!("GIT_COMMIT").unwrap_or("unknown").to_string();

    tauri::async_runtime::spawn_blocking(move || {
        evidence_pack::create_evidence_pack(
            &output_dir,
            &request,
            &app_name,
            &app_version,
            &git_commit,
        )
    })
    .await
    .map_err(|e| format!("comm_evidence_pack_create join error: {e}"))?
}

#[tauri::command]
pub async fn comm_evidence_verify_v1(path: String) -> CommEvidenceVerifyV1Response {
    let path_buf = std::path::PathBuf::from(path);

    match tauri::async_runtime::spawn_blocking(move || {
        evidence_pack::verify_evidence_pack_v1(&path_buf)
    })
    .await
    {
        Ok(resp) => resp,
        Err(e) => CommEvidenceVerifyV1Response {
            ok: false,
            checks: Vec::new(),
            errors: vec![EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::ZipReadError,
                message: format!("comm_evidence_verify_v1 spawn_blocking join error: {e}"),
                details: Some(EvidenceVerifyErrorDetails {
                    message: Some("spawn_blocking join error".to_string()),
                    ..Default::default()
                }),
            }],
        },
    }
}

#[tauri::command]
pub async fn comm_import_union_xlsx(
    path: String,
    options: Option<import_union_xlsx::ImportUnionOptions>,
) -> CommImportUnionXlsxResponse {
    let path_buf = std::path::PathBuf::from(path);
    let options = options.unwrap_or_default();
    let strict = options.strict.unwrap_or(false);
    let address_base_used = options.address_base.unwrap_or_default();
    let used_sheet = options
        .sheet_name
        .clone()
        .unwrap_or_else(|| union_spec_v1::DEFAULT_SHEET_V1.to_string());

    let fallback_diagnostics = import_union_xlsx::ImportUnionDiagnostics {
        detected_sheets: Vec::new(),
        detected_columns: Vec::new(),
        used_sheet: used_sheet.clone(),
        strict,
        address_base_used,
        rows_scanned: 0,
        spec_version: Some(union_spec_v1::SPEC_VERSION_V1.to_string()),
        required_columns: Some(
            union_spec_v1::REQUIRED_COLUMNS_V1
                .iter()
                .map(|v| v.to_string())
                .collect(),
        ),
        allowed_protocols: Some(
            union_spec_v1::ALLOWED_PROTOCOLS_V1
                .iter()
                .map(|v| v.to_string())
                .collect(),
        ),
        allowed_datatypes: Some(
            union_spec_v1::ALLOWED_DATATYPES_V1
                .iter()
                .map(|v| v.to_string())
                .collect(),
        ),
        allowed_byte_orders: Some(
            union_spec_v1::ALLOWED_BYTEORDERS_V1
                .iter()
                .map(|v| v.to_string())
                .collect(),
        ),
    };

    match tauri::async_runtime::spawn_blocking(move || {
        import_union_xlsx::import_union_xlsx_with_options(&path_buf, Some(options))
    })
    .await
    {
        Ok(Ok(outcome)) => CommImportUnionXlsxResponse {
            ok: Some(true),
            error: None,
            points: outcome.points,
            profiles: outcome.profiles,
            warnings: outcome.warnings,
            diagnostics: Some(outcome.diagnostics),
        },
        Ok(Err(err)) => CommImportUnionXlsxResponse {
            ok: Some(false),
            error: Some(err.to_import_error()),
            points: PointsV1 {
                schema_version: SCHEMA_VERSION_V1,
                points: Vec::new(),
            },
            profiles: ProfilesV1 {
                schema_version: SCHEMA_VERSION_V1,
                profiles: Vec::new(),
            },
            warnings: Vec::new(),
            diagnostics: Some(err.diagnostics().cloned().unwrap_or(fallback_diagnostics)),
        },
        Err(join_err) => CommImportUnionXlsxResponse {
            ok: Some(false),
            error: Some(ImportUnionError {
                kind: ImportUnionErrorKind::UnionXlsxReadError,
                message: format!("import_union_xlsx spawn_blocking join error: {join_err}"),
                details: Some(ImportUnionErrorDetails {
                    sheet_name: Some(used_sheet),
                    address_base_used: Some(address_base_used),
                    ..Default::default()
                }),
            }),
            points: PointsV1 {
                schema_version: SCHEMA_VERSION_V1,
                points: Vec::new(),
            },
            profiles: ProfilesV1 {
                schema_version: SCHEMA_VERSION_V1,
                profiles: Vec::new(),
            },
            warnings: Vec::new(),
            diagnostics: Some(fallback_diagnostics),
        },
    }
}

fn resolve_profiles(
    base_dir: &std::path::Path,
    state: &State<'_, CommState>,
    scope: &str,
    payload: Option<ProfilesV1>,
) -> Result<ProfilesV1, String> {
    if let Some(payload) = payload {
        if payload.schema_version != SCHEMA_VERSION_V1 {
            return Err(format!(
                "unsupported schemaVersion: {}",
                payload.schema_version
            ));
        }
        return Ok(payload);
    }

    if let Some(v) = storage::load_profiles(&base_dir).map_err(|e| e.to_string())? {
        state.memory.lock().scope_mut(scope).profiles = Some(v.clone());
        return Ok(v);
    }

    state
        .memory
        .lock()
        .scope(scope)
        .and_then(|v| v.profiles.clone())
        .ok_or_else(|| "profiles not provided and not saved".to_string())
}

fn resolve_points(
    base_dir: &std::path::Path,
    state: &State<'_, CommState>,
    scope: &str,
    payload: Option<PointsV1>,
) -> Result<PointsV1, String> {
    if let Some(payload) = payload {
        if payload.schema_version != SCHEMA_VERSION_V1 {
            return Err(format!(
                "unsupported schemaVersion: {}",
                payload.schema_version
            ));
        }
        return Ok(payload);
    }

    if let Some(v) = storage::load_points(&base_dir).map_err(|e| e.to_string())? {
        state.memory.lock().scope_mut(scope).points = Some(v.clone());
        return Ok(v);
    }

    state
        .memory
        .lock()
        .scope(scope)
        .and_then(|v| v.points.clone())
        .ok_or_else(|| "points not provided and not saved".to_string())
}

fn profiles_min_poll_interval_ms(profiles: &[ConnectionProfile]) -> Option<u32> {
    profiles
        .iter()
        .map(|p| match p {
            ConnectionProfile::Tcp {
                poll_interval_ms, ..
            } => *poll_interval_ms,
            ConnectionProfile::Rtu485 {
                poll_interval_ms, ..
            } => *poll_interval_ms,
        })
        .min()
}

fn profiles_has_mismatched_protocol(
    profiles: &[ConnectionProfile],
    driver_kind: CommDriverKind,
) -> bool {
    match driver_kind {
        CommDriverKind::Tcp => profiles
            .iter()
            .any(|p| matches!(p, ConnectionProfile::Rtu485 { .. })),
        CommDriverKind::Rtu485 => profiles
            .iter()
            .any(|p| matches!(p, ConnectionProfile::Tcp { .. })),
    }
}

fn infer_driver_kind_from_profiles(
    profiles: &[ConnectionProfile],
) -> Result<CommDriverKind, String> {
    let mut has_tcp = false;
    let mut has_rtu = false;
    for p in profiles {
        match p {
            ConnectionProfile::Tcp { .. } => has_tcp = true,
            ConnectionProfile::Rtu485 { .. } => has_rtu = true,
        }
    }

    match (has_tcp, has_rtu) {
        (true, false) => Ok(CommDriverKind::Tcp),
        (false, true) => Ok(CommDriverKind::Rtu485),
        (false, false) => Err("profiles is empty".to_string()),
        (true, true) => Err(
            "mixed TCP and 485 profiles are not supported in a single run; please run separately"
                .to_string(),
        ),
    }
}

fn scope_key(project_id: Option<&str>) -> String {
    project_id.unwrap_or("legacy").to_string()
}

fn comm_base_dir(app: &AppHandle, project_id: Option<&str>) -> Result<std::path::PathBuf, String> {
    if let Some(v) = project_id {
        projects::validate_project_id(v)?;
    }

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(match project_id {
        Some(project_id) => projects::project_comm_dir(&app_data_dir, project_id),
        None => storage::comm_dir(app_data_dir),
    })
}

// Legacy evidence-pack implementation (previously behind `#[cfg(any())]`) has been removed.
// The active implementation lives in `crate::comm::usecase::evidence_pack`.
