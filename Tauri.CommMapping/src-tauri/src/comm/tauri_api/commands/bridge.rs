use tauri::AppHandle;

use crate::comm::bridge_importresult_stub;
use crate::comm::bridge_plc_import;
use crate::comm::export_plc_import_stub;
use crate::comm::merge_unified_import;
use crate::comm::path_resolver;
use crate::comm::tauri_api::common::comm_base_dir;
use crate::comm::tauri_api::{
    CommBridgeConsumeCheckRequest, CommBridgeConsumeCheckResponse,
    CommBridgeExportImportResultStubV1Request, CommBridgeExportImportResultStubV1Response,
    CommBridgeToPlcImportV1Request, CommBridgeToPlcImportV1Response,
    CommMergeImportSourcesV1Request, CommMergeImportSourcesV1Response,
    CommUnifiedExportPlcImportStubV1Request, CommUnifiedExportPlcImportStubV1Response,
};
use crate::comm::error::{
    BridgeCheckError, BridgeCheckErrorKind, ImportResultStubError, ImportResultStubErrorKind,
    MergeImportSourcesError, MergeImportSourcesErrorKind, PlcBridgeError, PlcBridgeErrorKind,
    UnifiedPlcImportStubError, UnifiedPlcImportStubErrorKind,
};

#[tauri::command]
pub async fn comm_bridge_to_plc_import_v1(
    app: AppHandle,
    request: CommBridgeToPlcImportV1Request,
    project_id: Option<String>,
) -> CommBridgeToPlcImportV1Response {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommBridgeToPlcImportV1Response {
                out_path: "".to_string(),
                summary: None,
                ok: Some(false),
                error: Some(PlcBridgeError {
                    kind: PlcBridgeErrorKind::CommIrReadError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let ir_path_text = request.ir_path.trim().to_string();
    if ir_path_text.is_empty() {
        return CommBridgeToPlcImportV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(PlcBridgeError {
                kind: PlcBridgeErrorKind::CommIrReadError,
                message: "irPath is empty".to_string(),
                details: None,
            }),
        };
    }

    let out_path_override = request.out_path.unwrap_or_default();

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_path = if out_path_override.trim().is_empty() {
            path_resolver::default_plc_bridge_path(&output_dir, now)
        } else {
            std::path::PathBuf::from(&out_path_override)
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PlcBridgeError {
                kind: PlcBridgeErrorKind::PlcBridgeWriteError,
                message: e.to_string(),
                details: None,
            })?;
        }

        let ir_path = std::path::PathBuf::from(&ir_path_text);
        let outcome = bridge_plc_import::export_plc_import_bridge_v1(&ir_path, &out_path)?;

        Ok::<_, PlcBridgeError>(CommBridgeToPlcImportV1Response {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            summary: Some(outcome.summary),
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommBridgeToPlcImportV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommBridgeToPlcImportV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(PlcBridgeError {
                kind: PlcBridgeErrorKind::PlcBridgeWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_bridge_consume_check(
    app: AppHandle,
    request: CommBridgeConsumeCheckRequest,
    project_id: Option<String>,
) -> CommBridgeConsumeCheckResponse {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommBridgeConsumeCheckResponse {
                out_path: "".to_string(),
                summary: None,
                ok: Some(false),
                error: Some(BridgeCheckError {
                    kind: BridgeCheckErrorKind::PlcBridgeReadError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let bridge_path_text = request.bridge_path.trim().to_string();
    if bridge_path_text.is_empty() {
        return CommBridgeConsumeCheckResponse {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(BridgeCheckError {
                kind: BridgeCheckErrorKind::PlcBridgeReadError,
                message: "bridgePath is empty".to_string(),
                details: None,
            }),
        };
    }

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_dir = path_resolver::bridge_check_dir(&output_dir, now);

        let bridge_path = std::path::PathBuf::from(&bridge_path_text);
        let outcome = bridge_plc_import::consume_bridge_and_write_summary(&bridge_path, &out_dir)?;

        Ok::<_, BridgeCheckError>(CommBridgeConsumeCheckResponse {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            summary: Some(outcome.summary),
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommBridgeConsumeCheckResponse {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommBridgeConsumeCheckResponse {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(BridgeCheckError {
                kind: BridgeCheckErrorKind::BridgeSummaryWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_bridge_export_importresult_stub_v1(
    app: AppHandle,
    request: CommBridgeExportImportResultStubV1Request,
    project_id: Option<String>,
) -> CommBridgeExportImportResultStubV1Response {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommBridgeExportImportResultStubV1Response {
                out_path: "".to_string(),
                summary: None,
                ok: Some(false),
                error: Some(ImportResultStubError {
                    kind: ImportResultStubErrorKind::PlcBridgeReadError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let bridge_path_text = request.bridge_path.trim().to_string();
    if bridge_path_text.is_empty() {
        return CommBridgeExportImportResultStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(ImportResultStubError {
                kind: ImportResultStubErrorKind::PlcBridgeReadError,
                message: "bridgePath is empty".to_string(),
                details: None,
            }),
        };
    }

    let out_path_override = request.out_path.unwrap_or_default();

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);
        let out_path = if out_path_override.trim().is_empty() {
            path_resolver::default_importresult_stub_path(&output_dir, now)
        } else {
            std::path::PathBuf::from(&out_path_override)
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ImportResultStubError {
                kind: ImportResultStubErrorKind::ImportResultStubWriteError,
                message: e.to_string(),
                details: None,
            })?;
        }

        let bridge_path = std::path::PathBuf::from(&bridge_path_text);
        let outcome =
            bridge_importresult_stub::export_import_result_stub_v1(&bridge_path, &out_path)?;

        Ok::<_, ImportResultStubError>(CommBridgeExportImportResultStubV1Response {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            summary: Some(outcome.summary),
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommBridgeExportImportResultStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommBridgeExportImportResultStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(ImportResultStubError {
                kind: ImportResultStubErrorKind::ImportResultStubWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_merge_import_sources_v1(
    app: AppHandle,
    request: CommMergeImportSourcesV1Request,
    project_id: Option<String>,
) -> CommMergeImportSourcesV1Response {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommMergeImportSourcesV1Response {
                out_path: "".to_string(),
                report_path: None,
                summary: None,
                warnings: Vec::new(),
                ok: Some(false),
                error: Some(MergeImportSourcesError {
                    kind: MergeImportSourcesErrorKind::MergeWriteError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let union_path_text = request.union_xlsx_path.trim().to_string();
    if union_path_text.is_empty() {
        return CommMergeImportSourcesV1Response {
            out_path: "".to_string(),
            report_path: None,
            summary: None,
            warnings: Vec::new(),
            ok: Some(false),
            error: Some(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::UnionXlsxReadError,
                message: "unionXlsxPath is empty".to_string(),
                details: None,
            }),
        };
    }

    let stub_path_text = request.import_result_stub_path.trim().to_string();
    if stub_path_text.is_empty() {
        return CommMergeImportSourcesV1Response {
            out_path: "".to_string(),
            report_path: None,
            summary: None,
            warnings: Vec::new(),
            ok: Some(false),
            error: Some(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::ImportResultStubReadError,
                message: "importResultStubPath is empty".to_string(),
                details: None,
            }),
        };
    }

    let out_path_override = request.out_path.unwrap_or_default();

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);

        let out_path = if out_path_override.trim().is_empty() {
            path_resolver::default_unified_import_path(&output_dir, now)
        } else {
            std::path::PathBuf::from(&out_path_override)
        };
        let report_path = path_resolver::default_merge_report_path(&output_dir, now);

        let union_path = std::path::PathBuf::from(&union_path_text);
        let stub_path = std::path::PathBuf::from(&stub_path_text);

        let outcome = merge_unified_import::merge_import_sources_v1(
            &union_path,
            &stub_path,
            &out_path,
            &report_path,
        )?;

        Ok::<_, MergeImportSourcesError>(CommMergeImportSourcesV1Response {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            report_path: Some(outcome.report_path.to_string_lossy().to_string()),
            summary: Some(outcome.summary),
            warnings: outcome.warnings,
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommMergeImportSourcesV1Response {
            out_path: "".to_string(),
            report_path: None,
            summary: None,
            warnings: Vec::new(),
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommMergeImportSourcesV1Response {
            out_path: "".to_string(),
            report_path: None,
            summary: None,
            warnings: Vec::new(),
            ok: Some(false),
            error: Some(MergeImportSourcesError {
                kind: MergeImportSourcesErrorKind::MergeWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn comm_unified_export_plc_import_stub_v1(
    app: AppHandle,
    request: CommUnifiedExportPlcImportStubV1Request,
    project_id: Option<String>,
) -> CommUnifiedExportPlcImportStubV1Response {
    let base_dir = match comm_base_dir(&app, project_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return CommUnifiedExportPlcImportStubV1Response {
                out_path: "".to_string(),
                summary: None,
                ok: Some(false),
                error: Some(UnifiedPlcImportStubError {
                    kind: UnifiedPlcImportStubErrorKind::PlcImportStubWriteError,
                    message: e,
                    details: None,
                }),
            }
        }
    };

    let unified_path_text = request.unified_import_path.trim().to_string();
    if unified_path_text.is_empty() {
        return CommUnifiedExportPlcImportStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(UnifiedPlcImportStubError {
                kind: UnifiedPlcImportStubErrorKind::UnifiedImportReadError,
                message: "unifiedImportPath is empty".to_string(),
                details: None,
            }),
        };
    }

    let out_path_override = request.out_path.unwrap_or_default();

    match tauri::async_runtime::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let output_dir = path_resolver::resolve_output_dir(&base_dir);

        let out_path = if out_path_override.trim().is_empty() {
            path_resolver::default_plc_import_stub_path(&output_dir, now)
        } else {
            std::path::PathBuf::from(&out_path_override)
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| UnifiedPlcImportStubError {
                kind: UnifiedPlcImportStubErrorKind::PlcImportStubWriteError,
                message: e.to_string(),
                details: None,
            })?;
        }

        let unified_path = std::path::PathBuf::from(&unified_path_text);
        let outcome = export_plc_import_stub::export_plc_import_stub_v1(&unified_path, &out_path)?;

        Ok::<_, UnifiedPlcImportStubError>(CommUnifiedExportPlcImportStubV1Response {
            out_path: outcome.out_path.to_string_lossy().to_string(),
            summary: Some(outcome.summary),
            ok: Some(true),
            error: None,
        })
    })
    .await
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => CommUnifiedExportPlcImportStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(e),
        },
        Err(e) => CommUnifiedExportPlcImportStubV1Response {
            out_path: "".to_string(),
            summary: None,
            ok: Some(false),
            error: Some(UnifiedPlcImportStubError {
                kind: UnifiedPlcImportStubErrorKind::PlcImportStubWriteError,
                message: format!("spawn_blocking join error: {e}"),
                details: None,
            }),
        },
    }
}
