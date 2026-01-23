use tauri::{AppHandle, State};

use crate::comm::tauri_api::services::export as export_service;
use crate::comm::tauri_api::{
    CommExportDeliveryXlsxRequest, CommExportDeliveryXlsxResponse, CommExportIrV1Request,
    CommExportIrV1Response, CommExportXlsxRequest, CommExportXlsxResponse, CommState,
};

#[tauri::command]
pub async fn comm_export_xlsx(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportXlsxRequest,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<CommExportXlsxResponse, String> {
    export_service::export_xlsx(app, state, request, project_id, device_id).await
}

#[tauri::command]
pub async fn comm_export_delivery_xlsx(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportDeliveryXlsxRequest,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<CommExportDeliveryXlsxResponse, String> {
    export_service::export_delivery_xlsx(app, state, request, project_id, device_id).await
}

#[tauri::command]
pub async fn comm_export_ir_v1(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommExportIrV1Request,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<CommExportIrV1Response, String> {
    export_service::export_ir(app, state, request, project_id, device_id).await
}
