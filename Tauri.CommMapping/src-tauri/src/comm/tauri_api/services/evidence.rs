use tauri::AppHandle;

use crate::comm::error::{ImportUnionError, ImportUnionErrorDetails, ImportUnionErrorKind};
use crate::comm::import_union_xlsx;
use crate::comm::model::{PointsV1, ProfilesV1, SCHEMA_VERSION_V1};
use crate::comm::tauri_api::common::comm_base_dir;
use crate::comm::tauri_api::{
    CommEvidencePackRequest, CommEvidencePackResponse, CommEvidenceVerifyV1Response,
    CommImportUnionXlsxResponse, EvidenceVerifyError, EvidenceVerifyErrorDetails,
    EvidenceVerifyErrorKind,
};
use crate::comm::union_spec_v1;
use crate::comm::usecase::evidence_pack;

pub(crate) async fn evidence_pack_create(
    app: AppHandle,
    request: CommEvidencePackRequest,
    project_id: Option<String>,
) -> Result<CommEvidencePackResponse, String> {
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let output_dir = if project_id.is_some() {
        base_dir.clone()
    } else {
        crate::comm::path_resolver::resolve_output_dir(&base_dir)
    };
    let app_name = app.config().identifier.clone();
    let app_version = app.package_info().version.to_string();
    let git_commit = option_env!("GIT_COMMIT").unwrap_or("unknown").to_string();

    tauri::async_runtime::spawn_blocking(move || {
        evidence_pack::create_evidence_pack(
            &output_dir,
            &request,
            &app_name,
            &app_version,
            &git_commit,
        )
    })
    .await
    .map_err(|e| format!("comm_evidence_pack_create join error: {e}"))?
}

pub(crate) async fn evidence_verify(path: String) -> CommEvidenceVerifyV1Response {
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

pub(crate) async fn import_union_xlsx(
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
