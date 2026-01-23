use chrono::{DateTime, Utc};
use tauri::{AppHandle, State};

use crate::comm::export_delivery_xlsx;
use crate::comm::export_ir;
use crate::comm::export_xlsx::export_comm_address_xlsx;
use crate::comm::model::{RunStats, SampleResult};
use crate::comm::path_resolver;
use crate::comm::storage;
use crate::comm::tauri_api::common::{
    comm_base_dir, load_project_data_if_needed, resolve_points, resolve_profiles,
    resolve_project_device, scope_key,
};
use crate::comm::tauri_api::{
    CommExportDeliveryXlsxHeaders, CommExportDeliveryXlsxRequest, CommExportDeliveryXlsxResponse,
    CommExportIrV1Request, CommExportIrV1Response, CommExportXlsxHeaders, CommExportXlsxRequest,
    CommExportXlsxResponse, CommState, DeliveryResultsSource, DeliveryResultsStatus,
};

pub(crate) async fn export_xlsx(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportXlsxRequest,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<CommExportXlsxResponse, String> {
    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    let profiles_dir = comm_base_dir(&app, project_id.as_deref())?;
    let project_data = load_project_data_if_needed(&app, project_id.as_deref())?;
    let profiles = resolve_profiles(
        &profiles_dir,
        &state,
        &scope,
        request.profiles,
        project_data.as_ref(),
        device_id.as_deref(),
    )?;
    let points = resolve_points(
        &profiles_dir,
        &state,
        &scope,
        request.points,
        project_data.as_ref(),
        device_id.as_deref(),
    )?;
    let device_workbook_name = if let Some(project) = project_data.as_ref() {
        resolve_project_device(project, device_id.as_deref())?
            .map(|device| device.workbook_name.clone())
    } else {
        None
    };

    let base_dir = profiles_dir;
    let out_path_text = request.out_path;
    let profiles_vec = profiles.profiles.clone();
    let points_vec = points.points.clone();
    let is_project = project_id.is_some();
    let device_workbook_name = device_workbook_name.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let mut out_path = if out_path_text.trim().is_empty() {
            resolve_default_delivery_path(
                &base_dir,
                now,
                device_workbook_name.as_deref(),
                is_project,
            )
        } else {
            std::path::PathBuf::from(&out_path_text)
        };
        if out_path_text.trim().is_empty() {
            out_path = ensure_unique_delivery_xlsx_path(out_path);
        }

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

pub(crate) async fn export_delivery_xlsx(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportDeliveryXlsxRequest,
    project_id: Option<String>,
    device_id: Option<String>,
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

    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let project_data = load_project_data_if_needed(&app, project_id.as_deref())?;
    let profiles = resolve_profiles(
        &base_dir,
        &state,
        &scope,
        request_profiles,
        project_data.as_ref(),
        device_id.as_deref(),
    )?;
    let points = resolve_points(
        &base_dir,
        &state,
        &scope,
        request_points,
        project_data.as_ref(),
        device_id.as_deref(),
    )?;

    let include_results = include_results.unwrap_or(false);
    let results_source = results_source.unwrap_or(DeliveryResultsSource::Appdata);

    let profiles_vec = profiles.profiles.clone();
    let points_vec = points.points.clone();
    let is_project = project_id.is_some();
    let device_workbook_name = if let Some(project) = project_data.as_ref() {
        resolve_project_device(project, device_id.as_deref())?
            .map(|device| device.workbook_name.clone())
    } else {
        None
    };

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let mut out_path = if out_path_text.trim().is_empty() {
            resolve_default_delivery_path(
                &base_dir,
                now,
                device_workbook_name.as_deref(),
                is_project,
            )
        } else {
            std::path::PathBuf::from(&out_path_text)
        };
        if out_path_text.trim().is_empty() {
            out_path = ensure_unique_delivery_xlsx_path(out_path);
        }

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

pub(crate) async fn export_ir(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportIrV1Request,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<CommExportIrV1Response, String> {
    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let project_data = load_project_data_if_needed(&app, project_id.as_deref())?;
    let profiles = resolve_profiles(
        &base_dir,
        &state,
        &scope,
        request.profiles,
        project_data.as_ref(),
        device_id.as_deref(),
    )?;
    let points = resolve_points(
        &base_dir,
        &state,
        &scope,
        request.points,
        project_data.as_ref(),
        device_id.as_deref(),
    )?;

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

fn resolve_default_delivery_path(
    base_dir: &std::path::Path,
    now: DateTime<Utc>,
    device_name: Option<&str>,
    is_project: bool,
) -> std::path::PathBuf {
    let output_dir = path_resolver::resolve_output_dir(base_dir);
    if is_project {
        if let Some(name) = device_name {
            return path_resolver::default_device_delivery_xlsx_path(&output_dir, now, name);
        }
    }
    path_resolver::default_delivery_xlsx_path(&output_dir, now)
}

fn ensure_unique_delivery_xlsx_path(path: std::path::PathBuf) -> std::path::PathBuf {
    if !path.exists() {
        return path;
    }
    let ts = path_resolver::ts_label(Utc::now());
    path.with_file_name(format!("通讯地址表.{ts}.xlsx"))
}
