use tauri::{AppHandle, Manager};

use crate::comm::adapters::storage::projects;
use crate::comm::model::{CommProjectDataV1, CommProjectUiStateV1, CommProjectV1};
use crate::comm::tauri_api::types::{
    CommProjectCopyRequest, CommProjectCreateRequest, CommProjectsListRequest,
    CommProjectsListResponse,
};

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
        if patch.active_device_id.is_some() {
            ui.active_device_id = patch.active_device_id;
        }
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
