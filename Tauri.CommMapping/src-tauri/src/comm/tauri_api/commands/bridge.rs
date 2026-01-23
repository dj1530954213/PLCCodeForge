use tauri::AppHandle;

use crate::comm::tauri_api::services::bridge as bridge_service;
use crate::comm::tauri_api::{
    CommBridgeConsumeCheckRequest, CommBridgeConsumeCheckResponse,
    CommBridgeExportImportResultStubV1Request, CommBridgeExportImportResultStubV1Response,
    CommBridgeToPlcImportV1Request, CommBridgeToPlcImportV1Response,
    CommMergeImportSourcesV1Request, CommMergeImportSourcesV1Response,
    CommUnifiedExportPlcImportStubV1Request, CommUnifiedExportPlcImportStubV1Response,
};

#[tauri::command]
pub async fn comm_bridge_to_plc_import_v1(
    app: AppHandle,
    request: CommBridgeToPlcImportV1Request,
    project_id: Option<String>,
) -> CommBridgeToPlcImportV1Response {
    bridge_service::bridge_to_plc_import(app, request, project_id).await
}

#[tauri::command]
pub async fn comm_bridge_consume_check(
    app: AppHandle,
    request: CommBridgeConsumeCheckRequest,
    project_id: Option<String>,
) -> CommBridgeConsumeCheckResponse {
    bridge_service::bridge_consume_check(app, request, project_id).await
}

#[tauri::command]
pub async fn comm_bridge_export_importresult_stub_v1(
    app: AppHandle,
    request: CommBridgeExportImportResultStubV1Request,
    project_id: Option<String>,
) -> CommBridgeExportImportResultStubV1Response {
    bridge_service::bridge_export_importresult_stub(app, request, project_id).await
}

#[tauri::command]
pub async fn comm_merge_import_sources_v1(
    app: AppHandle,
    request: CommMergeImportSourcesV1Request,
    project_id: Option<String>,
) -> CommMergeImportSourcesV1Response {
    bridge_service::merge_import_sources(app, request, project_id).await
}

#[tauri::command]
pub async fn comm_unified_export_plc_import_stub_v1(
    app: AppHandle,
    request: CommUnifiedExportPlcImportStubV1Request,
    project_id: Option<String>,
) -> CommUnifiedExportPlcImportStubV1Response {
    bridge_service::unified_export_plc_import_stub(app, request, project_id).await
}
