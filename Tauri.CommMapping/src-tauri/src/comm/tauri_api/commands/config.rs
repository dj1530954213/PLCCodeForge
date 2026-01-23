use tauri::{AppHandle, State};

use crate::comm::model::{CommConfigV1, PointsV1, ProfilesV1};
use crate::comm::tauri_api::services::config as config_service;
use crate::comm::tauri_api::{CommPlanBuildRequest, CommState, PlanV1};

#[tauri::command]
pub fn comm_config_load(
    app: AppHandle,
    project_id: Option<String>,
) -> Result<CommConfigV1, String> {
    config_service::load_config(app, project_id)
}

#[tauri::command]
pub fn comm_config_save(
    app: AppHandle,
    payload: CommConfigV1,
    project_id: Option<String>,
) -> Result<(), String> {
    config_service::save_config(app, payload, project_id)
}

#[tauri::command]
pub fn comm_profiles_save(
    app: AppHandle,
    state: State<'_, CommState>,
    payload: ProfilesV1,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<(), String> {
    config_service::save_profiles(app, state, payload, project_id, device_id)
}

#[tauri::command]
pub fn comm_profiles_load(
    app: AppHandle,
    state: State<'_, CommState>,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<ProfilesV1, String> {
    config_service::load_profiles(app, state, project_id, device_id)
}

#[tauri::command]
pub fn comm_points_save(
    app: AppHandle,
    state: State<'_, CommState>,
    payload: PointsV1,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<(), String> {
    config_service::save_points(app, state, payload, project_id, device_id)
}

#[tauri::command]
pub fn comm_points_load(
    app: AppHandle,
    state: State<'_, CommState>,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<PointsV1, String> {
    config_service::load_points(app, state, project_id, device_id)
}

#[tauri::command]
pub fn comm_plan_build(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommPlanBuildRequest,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<PlanV1, String> {
    config_service::build_plan(app, state, request, project_id, device_id)
}
