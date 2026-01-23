use tauri::AppHandle;

use crate::comm::import_union_xlsx;
use crate::comm::tauri_api::services::evidence as evidence_service;
use crate::comm::tauri_api::{
    CommEvidencePackRequest, CommEvidencePackResponse, CommEvidenceVerifyV1Response,
    CommImportUnionXlsxResponse,
};

#[tauri::command]
pub async fn comm_evidence_pack_create(
    app: AppHandle,
    request: CommEvidencePackRequest,
    project_id: Option<String>,
) -> Result<CommEvidencePackResponse, String> {
    evidence_service::evidence_pack_create(app, request, project_id).await
}

#[tauri::command]
pub async fn comm_evidence_verify_v1(path: String) -> CommEvidenceVerifyV1Response {
    evidence_service::evidence_verify(path).await
}

#[tauri::command]
pub async fn comm_import_union_xlsx(
    path: String,
    options: Option<import_union_xlsx::ImportUnionOptions>,
) -> CommImportUnionXlsxResponse {
    evidence_service::import_union_xlsx(path, options).await
}
