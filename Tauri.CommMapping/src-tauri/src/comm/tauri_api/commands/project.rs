use tauri::AppHandle;

use crate::comm::model::{CommProjectDataV1, CommProjectUiStateV1, CommProjectV1};
use crate::comm::tauri_api::services::project as project_service;
use crate::comm::tauri_api::types::{
    CommProjectCopyRequest, CommProjectCreateRequest, CommProjectsListRequest,
    CommProjectsListResponse,
};

#[tauri::command]
pub async fn comm_project_create(
    app: AppHandle,
    request: CommProjectCreateRequest,
) -> Result<CommProjectV1, String> {
    project_service::create_project(app, request).await
}

#[tauri::command]
pub async fn comm_projects_list(
    app: AppHandle,
    request: Option<CommProjectsListRequest>,
) -> Result<CommProjectsListResponse, String> {
    project_service::list_projects(app, request).await
}

#[tauri::command]
pub async fn comm_project_get(
    app: AppHandle,
    project_id: String,
) -> Result<Option<CommProjectV1>, String> {
    project_service::get_project(app, project_id).await
}

#[tauri::command]
pub async fn comm_project_load_v1(
    app: AppHandle,
    project_id: String,
) -> Result<CommProjectDataV1, String> {
    project_service::load_project_data(app, project_id).await
}

#[tauri::command]
pub async fn comm_project_save_v1(
    app: AppHandle,
    payload: CommProjectDataV1,
) -> Result<(), String> {
    project_service::save_project_data(app, payload).await
}

#[tauri::command]
pub async fn comm_project_ui_state_patch_v1(
    app: AppHandle,
    project_id: String,
    patch: CommProjectUiStateV1,
) -> Result<(), String> {
    project_service::patch_project_ui_state(app, project_id, patch).await
}

#[tauri::command]
pub async fn comm_project_copy(
    app: AppHandle,
    request: CommProjectCopyRequest,
) -> Result<CommProjectV1, String> {
    project_service::copy_project(app, request).await
}

#[tauri::command]
pub async fn comm_project_delete(
    app: AppHandle,
    project_id: String,
) -> Result<CommProjectV1, String> {
    project_service::delete_project(app, project_id).await
}
