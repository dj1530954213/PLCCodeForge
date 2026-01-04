//! Tauri command 层（冻结契约入口）。
//!
//! 注意：一旦 DTO/命令在这里对外暴露，即视为稳定契约（后续只允许新增可选字段）。
//!
//! 硬约束（来自 Docs/通讯数据采集验证/执行要求.md）：
//! - `comm_run_start` 只能 spawn 后台任务；不得在 command 内循环采集
//! - `comm_run_latest` 只读缓存，不触发采集
//! - `comm_run_stop` 必须在 1s 内生效（MVP）
//! - DTO 契约冻结：只允许新增可选字段，不得改名/删字段/改语义

use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use super::bridge_importresult_stub;
use super::bridge_plc_import;
use super::driver::mock::MockDriver;
use super::driver::modbus_rtu::ModbusRtuDriver;
use super::driver::modbus_tcp::ModbusTcpDriver;
use super::driver::CommDriver;
use super::engine::CommRunEngine;
use super::error::{
    BridgeCheckError, BridgeCheckErrorKind, ImportResultStubError, ImportResultStubErrorKind,
    ImportUnionError, ImportUnionErrorDetails, ImportUnionErrorKind, MergeImportSourcesError,
    MergeImportSourcesErrorKind, PlcBridgeError, PlcBridgeErrorKind, UnifiedPlcImportStubError,
    UnifiedPlcImportStubErrorKind,
};
use super::export_delivery_xlsx;
use super::export_ir;
use super::export_plc_import_stub;
use super::export_xlsx::export_comm_address_xlsx;
use super::import_union_xlsx;
use super::merge_unified_import;
use super::model::{
    CommConfigV1, CommExportDiagnostics, CommWarning, ConnectionProfile, PointsV1, ProfilesV1,
    RunStats, SampleResult, SCHEMA_VERSION_V1,
};
use super::path_resolver;
use super::plan::{build_read_plan, PlanOptions, ReadPlan};
use super::storage;
use super::union_spec_v1;
use super::usecase::evidence_pack;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommPingResponse {
    pub ok: bool,
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
    Mock,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommRunLatestResponse {
    pub results: Vec<SampleResult>,
    pub stats: RunStats,
    pub updated_at_utc: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_warnings: Option<Vec<CommWarning>>,
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

#[derive(Default)]
struct CommMemoryStore {
    profiles: Option<ProfilesV1>,
    points: Option<PointsV1>,
    plan: Option<ReadPlan>,
}

pub struct CommState {
    memory: Mutex<CommMemoryStore>,
    engine: CommRunEngine,
    mock_driver: Arc<MockDriver>,
    tcp_driver: Arc<ModbusTcpDriver>,
    rtu_driver: Arc<ModbusRtuDriver>,
}

impl CommState {
    pub fn new() -> Self {
        Self {
            memory: Mutex::new(CommMemoryStore::default()),
            engine: CommRunEngine::new(),
            mock_driver: Arc::new(MockDriver::new()),
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
pub fn comm_config_load(app: AppHandle) -> Result<CommConfigV1, String> {
    let base_dir = comm_base_dir(&app)?;
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
pub fn comm_config_save(app: AppHandle, payload: CommConfigV1) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let base_dir = comm_base_dir(&app)?;
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
) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let base_dir = comm_base_dir(&app)?;
    storage::save_profiles(&base_dir, &payload).map_err(|e| e.to_string())?;
    state.memory.lock().profiles = Some(payload);
    Ok(())
}

#[tauri::command]
pub fn comm_profiles_load(
    app: AppHandle,
    state: State<'_, CommState>,
) -> Result<ProfilesV1, String> {
    let base_dir = comm_base_dir(&app)?;
    let loaded = storage::load_profiles(&base_dir).map_err(|e| e.to_string())?;
    if let Some(v) = loaded {
        state.memory.lock().profiles = Some(v.clone());
        return Ok(v);
    }

    Ok(state.memory.lock().profiles.clone().unwrap_or(ProfilesV1 {
        schema_version: SCHEMA_VERSION_V1,
        profiles: vec![],
    }))
}

#[tauri::command]
pub fn comm_points_save(
    app: AppHandle,
    state: State<'_, CommState>,
    payload: PointsV1,
) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let base_dir = comm_base_dir(&app)?;
    storage::save_points(&base_dir, &payload).map_err(|e| e.to_string())?;
    state.memory.lock().points = Some(payload);
    Ok(())
}

#[tauri::command]
pub fn comm_points_load(app: AppHandle, state: State<'_, CommState>) -> Result<PointsV1, String> {
    let base_dir = comm_base_dir(&app)?;
    let loaded = storage::load_points(&base_dir).map_err(|e| e.to_string())?;
    if let Some(v) = loaded {
        state.memory.lock().points = Some(v.clone());
        return Ok(v);
    }

    Ok(state.memory.lock().points.clone().unwrap_or(PointsV1 {
        schema_version: SCHEMA_VERSION_V1,
        points: vec![],
    }))
}

#[tauri::command]
pub fn comm_plan_build(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommPlanBuildRequest,
) -> Result<PlanV1, String> {
    let base_dir = comm_base_dir(&app)?;
    let profiles = resolve_profiles(&app, &state, request.profiles)?;
    let points = resolve_points(&app, &state, request.points)?;

    let options = request.options.unwrap_or_default();
    let plan =
        build_read_plan(&profiles.profiles, &points.points, options).map_err(|e| e.to_string())?;

    state.memory.lock().plan = Some(plan.clone());
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
) -> Result<CommRunStartResponse, String> {
    let base_dir = comm_base_dir(&app)?;
    let profiles = resolve_profiles(&app, &state, request.profiles)?;
    let points = resolve_points(&app, &state, request.points)?;

    let plan = match request.plan {
        Some(p) => p,
        None => {
            if let Some(saved) = state.memory.lock().plan.clone() {
                saved
            } else {
                if let Some(saved) = storage::load_plan(&base_dir).map_err(|e| e.to_string())? {
                    state.memory.lock().plan = Some(saved.plan.clone());
                    saved.plan
                } else {
                    build_read_plan(&profiles.profiles, &points.points, PlanOptions::default())
                        .map_err(|e| e.to_string())?
                }
            }
        }
    };

    let driver_kind = request.driver.unwrap_or(CommDriverKind::Mock);
    let driver: Arc<dyn CommDriver> = match driver_kind {
        CommDriverKind::Mock => Arc::clone(&state.mock_driver) as Arc<dyn CommDriver>,
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

    Ok(CommRunStartResponse { run_id })
}

#[tauri::command]
pub fn comm_run_latest(
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
) -> Result<(), String> {
    let snapshot = state.engine.latest(run_id);
    let stopped = state.engine.stop_run(run_id).await;

    if !stopped {
        return Err("run not found".to_string());
    }

    if let Some((results, stats, _updated_at_utc, _run_warnings)) = snapshot {
        let base_dir = match comm_base_dir(&app) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        tauri::async_runtime::spawn_blocking(move || {
            let _ = storage::save_last_results(&base_dir, &results, &stats);
        });
    }

    Ok(())
}

#[tauri::command]
pub async fn comm_export_xlsx(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportXlsxRequest,
) -> Result<CommExportXlsxResponse, String> {
    let profiles = resolve_profiles(&app, &state, request.profiles)?;
    let points = resolve_points(&app, &state, request.points)?;

    let base_dir = comm_base_dir(&app)?;
    let out_path_text = request.out_path;
    let profiles_vec = profiles.profiles.clone();
    let points_vec = points.points.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_path = if out_path_text.trim().is_empty() {
            path_resolver::default_delivery_xlsx_path(&output_dir, now)
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

    let profiles = resolve_profiles(&app, &state, request_profiles)?;
    let points = resolve_points(&app, &state, request_points)?;

    let base_dir = comm_base_dir(&app)?;
    let include_results = include_results.unwrap_or(false);
    let results_source = results_source.unwrap_or(DeliveryResultsSource::Appdata);

    let profiles_vec = profiles.profiles.clone();
    let points_vec = points.points.clone();

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_path = if out_path_text.trim().is_empty() {
            path_resolver::default_delivery_xlsx_path(&output_dir, now)
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
) -> Result<CommExportIrV1Response, String> {
    let base_dir = comm_base_dir(&app)?;
    let profiles = resolve_profiles(&app, &state, request.profiles)?;
    let points = resolve_points(&app, &state, request.points)?;

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
) -> CommBridgeToPlcImportV1Response {
    let base_dir = match comm_base_dir(&app) {
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
) -> CommBridgeConsumeCheckResponse {
    let base_dir = match comm_base_dir(&app) {
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
) -> CommBridgeExportImportResultStubV1Response {
    let base_dir = match comm_base_dir(&app) {
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
) -> CommMergeImportSourcesV1Response {
    let base_dir = match comm_base_dir(&app) {
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
) -> CommUnifiedExportPlcImportStubV1Response {
    let base_dir = match comm_base_dir(&app) {
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
) -> Result<CommEvidencePackResponse, String> {
    let base_dir = comm_base_dir(&app)?;
    let app_name = app.config().identifier.clone();
    let app_version = app.package_info().version.to_string();
    let git_commit = option_env!("GIT_COMMIT").unwrap_or("unknown").to_string();

    tauri::async_runtime::spawn_blocking(move || {
        evidence_pack::create_evidence_pack(
            &base_dir,
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
    app: &AppHandle,
    state: &State<'_, CommState>,
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

    let base_dir = comm_base_dir(app)?;
    if let Some(v) = storage::load_profiles(&base_dir).map_err(|e| e.to_string())? {
        state.memory.lock().profiles = Some(v.clone());
        return Ok(v);
    }

    state
        .memory
        .lock()
        .profiles
        .clone()
        .ok_or_else(|| "profiles not provided and not saved".to_string())
}

fn resolve_points(
    app: &AppHandle,
    state: &State<'_, CommState>,
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

    let base_dir = comm_base_dir(app)?;
    if let Some(v) = storage::load_points(&base_dir).map_err(|e| e.to_string())? {
        state.memory.lock().points = Some(v.clone());
        return Ok(v);
    }

    state
        .memory
        .lock()
        .points
        .clone()
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

fn comm_base_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(storage::comm_dir(app_data_dir))
}

// Legacy evidence-pack implementation (previously behind `#[cfg(any())]`) has been removed.
// The active implementation lives in `crate::comm::usecase::evidence_pack`.
