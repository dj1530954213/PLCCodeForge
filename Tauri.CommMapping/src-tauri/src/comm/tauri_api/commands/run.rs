use std::sync::Arc;

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::comm::driver::CommDriver;
use crate::comm::error::{CommRunError, CommRunErrorDetails, CommRunErrorKind};
use crate::comm::model::ConnectionProfile;
use crate::comm::plan::{build_read_plan, PlanOptions};
use crate::comm::storage;
use crate::comm::tauri_api::common::{
    comm_base_dir, load_project_data_if_needed, profiles_min_poll_interval_ms, resolve_points,
    resolve_profiles, scope_key,
};
use crate::comm::tauri_api::{
    CommDriverKind, CommRunLatestObsResponse, CommRunLatestResponse, CommRunStartObsResponse,
    CommRunStartRequest, CommRunStartResponse, CommRunStopObsResponse, CommState,
};
use crate::comm::usecase::run_validation;

#[tauri::command]
pub async fn comm_run_start(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommRunStartRequest,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<CommRunStartResponse, String> {
    let run_id = comm_run_start_inner(app, state, request, project_id, device_id)
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
    device_id: Option<String>,
) -> Result<CommRunStartObsResponse, CommRunError> {
    let resp = match comm_run_start_inner(app, state, request, project_id.clone(), device_id.clone()).await {
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
    device_id: Option<String>,
) -> Result<Uuid, CommRunError> {
    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    let base_dir = comm_base_dir(&app, project_id.as_deref())
        .map_err(|message| comm_run_error_from_message(message, None, project_id.clone(), device_id.clone()))?;
    let project_data = load_project_data_if_needed(&app, project_id.as_deref())
        .map_err(|message| comm_run_error_from_message(message, None, project_id.clone(), device_id.clone()))?;
    let profiles = resolve_profiles(
        &base_dir,
        &state,
        &scope,
        request.profiles,
        project_data.as_ref(),
        device_id.as_deref(),
    )
    .map_err(|message| comm_run_error_from_message(message, None, project_id.clone(), device_id.clone()))?;
    let points = resolve_points(
        &base_dir,
        &state,
        &scope,
        request.points,
        project_data.as_ref(),
        device_id.as_deref(),
    )
    .map_err(|message| comm_run_error_from_message(message, None, project_id.clone(), device_id.clone()))?;

    if profiles.profiles.is_empty() {
        return Err(CommRunError {
            kind: CommRunErrorKind::ConfigError,
            message: "连接配置为空".to_string(),
            details: Some(CommRunErrorDetails {
                project_id,
                device_id: device_id.clone(),
                ..Default::default()
            }),
        });
    }

    if points.points.is_empty() {
        return Err(CommRunError {
            kind: CommRunErrorKind::ConfigError,
            message: "点位列表为空".to_string(),
            details: Some(CommRunErrorDetails {
                project_id,
                device_id: device_id.clone(),
                ..Default::default()
            }),
        });
    }

    let mut missing_fields = run_validation::validate_run_inputs(&profiles.profiles, &points.points);
    if let Some(project) = project_data.as_ref() {
        if let Some(devices) = project.devices.as_ref() {
            missing_fields.extend(run_validation::validate_global_hmi_uniqueness(devices));
        } else {
            missing_fields.extend(run_validation::validate_hmi_uniqueness_points(&points.points));
        }
    } else {
        missing_fields.extend(run_validation::validate_hmi_uniqueness_points(&points.points));
    }
    if !missing_fields.is_empty() {
        return Err(CommRunError {
            kind: CommRunErrorKind::ConfigError,
            message: "点位或连接配置无效".to_string(),
            details: Some(CommRunErrorDetails {
                project_id,
                device_id: device_id.clone(),
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
                .map_err(|e| {
                    comm_run_error_from_message(
                        e.to_string(),
                        None,
                        project_id.clone(),
                        device_id.clone(),
                    )
                })?
            {
                state.memory.lock().scope_mut(&scope).plan = Some(saved.plan.clone());
                saved.plan
            } else {
                build_read_plan(&profiles.profiles, &points.points, PlanOptions::default())
                    .map_err(|e| {
                        comm_run_error_from_message(
                            e.to_string(),
                            None,
                            project_id.clone(),
                            device_id.clone(),
                        )
                    })?
            }
        }
    };

    let driver_kind = match request.driver {
        Some(v) => v,
        None => infer_driver_kind_from_profiles(&profiles.profiles)
            .map_err(|message| {
                comm_run_error_from_message(message, None, project_id.clone(), device_id.clone())
            })?,
    };
    if profiles_has_mismatched_protocol(&profiles.profiles, driver_kind.clone()) {
        return Err(CommRunError {
            kind: CommRunErrorKind::ConfigError,
            message: format!("driver={driver_kind:?} does not match profiles.protocolType"),
            details: Some(CommRunErrorDetails {
                project_id,
                device_id: device_id.clone(),
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
            error: Some(comm_run_error_from_message(
                message,
                Some(run_id),
                None,
                None,
            )),
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
                None,
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
    device_id: Option<String>,
) -> CommRunError {
    CommRunError {
        kind: comm_run_error_kind_from_message(&message),
        message,
        details: Some(CommRunErrorDetails {
            run_id: run_id.map(|v| v.to_string()),
            project_id,
            device_id,
            ..Default::default()
        }),
    }
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
        (false, false) => Err("连接配置为空".to_string()),
        (true, true) => Err(
            "TCP 与 485 混用不支持同一次运行，请拆分为两次运行".to_string(),
        ),
    }
}
