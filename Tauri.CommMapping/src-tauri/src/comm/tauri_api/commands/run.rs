use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::comm::error::CommRunError;
use crate::comm::tauri_api::services::run as run_service;
use crate::comm::tauri_api::{
    CommRunLatestObsResponse, CommRunLatestResponse, CommRunStartObsResponse,
    CommRunStartRequest, CommRunStartResponse, CommRunStopObsResponse, CommState,
};

#[tauri::command]
pub async fn comm_run_start(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommRunStartRequest,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<CommRunStartResponse, String> {
    let run_id = run_service::start_run(app, state, request, project_id, device_id)
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
    let resp = match run_service::start_run(
        app,
        state,
        request,
        project_id.clone(),
        device_id.clone(),
    )
    .await
    {
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

#[tauri::command]
pub fn comm_run_latest(
    state: State<'_, CommState>,
    run_id: Uuid,
) -> Result<CommRunLatestResponse, String> {
    run_service::latest_run(state, run_id)
}

#[tauri::command]
pub fn comm_run_latest_obs(state: State<'_, CommState>, run_id: Uuid) -> CommRunLatestObsResponse {
    match run_service::latest_run(state, run_id) {
        Ok(value) => CommRunLatestObsResponse {
            ok: true,
            value: Some(value),
            error: None,
        },
        Err(message) => CommRunLatestObsResponse {
            ok: false,
            value: None,
            error: Some(run_service::error_from_message(
                message,
                Some(run_id),
                None,
                None,
            )),
        },
    }
}

#[tauri::command]
pub async fn comm_run_stop(
    app: AppHandle,
    state: State<'_, CommState>,
    run_id: Uuid,
    project_id: Option<String>,
) -> Result<(), String> {
    run_service::stop_run(app, state, run_id, project_id).await
}

#[tauri::command]
pub async fn comm_run_stop_obs(
    app: AppHandle,
    state: State<'_, CommState>,
    run_id: Uuid,
    project_id: Option<String>,
) -> Result<CommRunStopObsResponse, CommRunError> {
    let resp = match run_service::stop_run(app, state, run_id, project_id.clone()).await {
        Ok(()) => CommRunStopObsResponse {
            ok: true,
            error: None,
        },
        Err(message) => CommRunStopObsResponse {
            ok: false,
            error: Some(run_service::error_from_message(
                message,
                Some(run_id),
                project_id,
                None,
            )),
        },
    };
    Ok(resp)
}
