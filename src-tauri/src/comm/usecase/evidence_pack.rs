//! Evidence pack 用例：将一次 demo/run 的关键产物打包为可回传的 evidence 目录/zip，并提供回归校验。
//!
//! 说明：
//! - 这是用例层逻辑（会进行文件 IO、zip 打包），command 侧需通过 `spawn_blocking` 调用，避免阻塞 UI 线程。
//! - DTO/字段语义冻结（仅允许未来新增可选字段）。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::comm::adapters::storage::path_resolver;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommEvidencePackRequest {
    pub pipeline_log: JsonValue,
    pub export_response: JsonValue,
    #[serde(default)]
    pub conflict_report: Option<JsonValue>,
    /// 可选：由前端提供的运行配置/统计快照（后端会写入 manifest.json）。
    #[serde(default)]
    pub meta: Option<JsonValue>,
    #[serde(default)]
    pub exported_xlsx_path: Option<String>,
    /// 可选：CommIR v1 的输出路径（用于写入 manifest.outputs.irPath/irDigest）。
    #[serde(default)]
    pub ir_path: Option<String>,
    /// 可选：PlcImportBridge v1 的输出路径（用于写入 manifest.outputs.plcBridgePath/plcBridgeDigest）。
    #[serde(default)]
    pub plc_bridge_path: Option<String>,
    /// 可选：ImportResultStub v1 的输出路径（用于写入 manifest.outputs.importResultStubPath/importResultStubDigest）。
    #[serde(default)]
    pub import_result_stub_path: Option<String>,
    /// 可选：UnifiedImport v1 的输出路径（用于写入 manifest.outputs.unifiedImportPath/unifiedImportDigest）。
    #[serde(default)]
    pub unified_import_path: Option<String>,
    /// 可选：merge_report v1 的输出路径（用于写入 manifest.outputs.mergeReportPath/mergeReportDigest）。
    #[serde(default)]
    pub merge_report_path: Option<String>,
    /// 可选：plc_import_stub v1 的输出路径（用于写入 manifest.outputs.plcImportStubPath/plcImportStubDigest）。
    #[serde(default)]
    pub plc_import_stub_path: Option<String>,
    /// 可选：联合 xlsx 输入路径（用于写入 manifest.inputs.unionXlsxDigest 等，便于可追溯）。
    #[serde(default)]
    pub union_xlsx_path: Option<String>,
    /// 可选：解析使用到的列名清单（用于写入 manifest.inputs.parsedColumnsUsed）。
    #[serde(default)]
    pub parsed_columns_used: Option<Vec<String>>,
    #[serde(default)]
    pub copy_xlsx: Option<bool>,
    #[serde(default)]
    pub copy_ir: Option<bool>,
    #[serde(default)]
    pub copy_plc_bridge: Option<bool>,
    #[serde(default)]
    pub copy_import_result_stub: Option<bool>,
    #[serde(default)]
    pub copy_unified_import: Option<bool>,
    #[serde(default)]
    pub copy_merge_report: Option<bool>,
    #[serde(default)]
    pub copy_plc_import_stub: Option<bool>,
    #[serde(default)]
    pub zip: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommEvidencePackResponse {
    pub evidence_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip_path: Option<String>,
    pub manifest: JsonValue,
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceVerifyErrorKind {
    #[serde(rename = "PathNotFound")]
    PathNotFound,
    #[serde(rename = "ZipReadError")]
    ZipReadError,
    #[serde(rename = "ManifestMissing")]
    ManifestMissing,
    #[serde(rename = "ManifestParseError")]
    ManifestParseError,
    #[serde(rename = "EvidenceSummaryMissing")]
    EvidenceSummaryMissing,
    #[serde(rename = "EvidenceSummaryParseError")]
    EvidenceSummaryParseError,
    #[serde(rename = "FileMissing")]
    FileMissing,
    #[serde(rename = "DigestMismatch")]
    DigestMismatch,
    #[serde(rename = "SchemaMismatch")]
    SchemaMismatch,
    #[serde(rename = "PointsOrderMismatch")]
    PointsOrderMismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceVerifyErrorDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceVerifyError {
    pub kind: EvidenceVerifyErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<EvidenceVerifyErrorDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceVerifyCheck {
    pub name: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommEvidenceVerifyV1Response {
    pub ok: bool,
    pub checks: Vec<EvidenceVerifyCheck>,
    pub errors: Vec<EvidenceVerifyError>,
}

fn write_json_atomic(path: &std::path::Path, value: &serde_json::Value) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    std::fs::rename(tmp_path, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn build_zip(zip_path: &std::path::Path, files: &[std::path::PathBuf]) -> Result<(), String> {
    use std::io::Write;

    let file = std::fs::File::create(zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for path in files {
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn headers_digest_sha256(export_response: &JsonValue) -> String {
    use sha2::{Digest, Sha256};

    let headers = export_response.get("headers");
    let tcp = headers
        .and_then(|h| h.get("tcp"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let rtu = headers
        .and_then(|h| h.get("rtu"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let params = headers
        .and_then(|h| h.get("params"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if tcp.is_empty() && rtu.is_empty() && params.is_empty() {
        return "sha256:unknown".to_string();
    }

    let canonical = json!({
        "tcp": tcp,
        "rtu": rtu,
        "params": params,
    })
    .to_string();

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    format!("sha256:{hex}")
}

fn file_digest_sha256(path: &std::path::Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    Ok(format!("sha256:{hex}"))
}

fn sha256_digest_prefixed_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    format!("sha256:{hex}")
}

trait EvidenceAccessor {
    fn exists(&mut self, file_name: &str) -> bool;
    fn read_bytes(&mut self, file_name: &str) -> Result<Vec<u8>, String>;
}

struct DirEvidenceAccessor {
    root: std::path::PathBuf,
}

impl EvidenceAccessor for DirEvidenceAccessor {
    fn exists(&mut self, file_name: &str) -> bool {
        self.root.join(file_name).exists()
    }

    fn read_bytes(&mut self, file_name: &str) -> Result<Vec<u8>, String> {
        std::fs::read(self.root.join(file_name)).map_err(|e| e.to_string())
    }
}

struct ZipEvidenceAccessor {
    zip: zip::ZipArchive<std::fs::File>,
}

impl EvidenceAccessor for ZipEvidenceAccessor {
    fn exists(&mut self, file_name: &str) -> bool {
        self.zip.by_name(file_name).is_ok()
    }

    fn read_bytes(&mut self, file_name: &str) -> Result<Vec<u8>, String> {
        use std::io::Read;

        let mut f = self.zip.by_name(file_name).map_err(|e| e.to_string())?;
        let mut bytes: Vec<u8> = Vec::new();
        f.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }
}

fn evidence_file_name_from_path_value(v: Option<&JsonValue>) -> Option<String> {
    let s = v?.as_str()?;
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    std::path::Path::new(s)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
}

fn should_verify_digest(expected: &str) -> bool {
    expected.starts_with("sha256:") && expected != "sha256:unknown"
}

pub(crate) fn create_evidence_pack(
    base_dir: &std::path::Path,
    request: &CommEvidencePackRequest,
    app_name: &str,
    app_version: &str,
    git_commit: &str,
) -> Result<CommEvidencePackResponse, String> {
    let copy_xlsx = request.copy_xlsx.unwrap_or(true);
    let copy_ir = request.copy_ir.unwrap_or(true);
    let copy_plc_bridge = request.copy_plc_bridge.unwrap_or(true);
    let copy_import_result_stub = request.copy_import_result_stub.unwrap_or(true);
    let copy_unified_import = request.copy_unified_import.unwrap_or(true);
    let copy_merge_report = request.copy_merge_report.unwrap_or(true);
    let copy_plc_import_stub = request.copy_plc_import_stub.unwrap_or(true);
    let zip_enabled = request.zip.unwrap_or(true);

    let now = chrono::Utc::now();
    let output_dir = path_resolver::resolve_output_dir(base_dir);
    let evidence_dir = path_resolver::evidence_dir(&output_dir, now);
    std::fs::create_dir_all(&evidence_dir).map_err(|e| e.to_string())?;

    let mut warnings: Vec<String> = Vec::new();
    let mut files: Vec<std::path::PathBuf> = Vec::new();

    let pipeline_log_path = evidence_dir.join("pipeline_log.json");
    write_json_atomic(&pipeline_log_path, &request.pipeline_log)
        .map_err(|e| format!("write pipeline_log.json failed: {e}"))?;
    files.push(pipeline_log_path);

    let export_resp_path = evidence_dir.join("export_response.json");
    write_json_atomic(&export_resp_path, &request.export_response)
        .map_err(|e| format!("write export_response.json failed: {e}"))?;
    files.push(export_resp_path);

    if let Some(conflict) = &request.conflict_report {
        let conflict_path = evidence_dir.join("conflict_report.json");
        write_json_atomic(&conflict_path, conflict)
            .map_err(|e| format!("write conflict_report.json failed: {e}"))?;
        files.push(conflict_path);
    }

    // ------- manifest.json（可追溯签收单） -------
    let meta = request.meta.clone().unwrap_or(JsonValue::Null);
    let manifest_created_at = now.to_rfc3339();

    let meta_run = meta.get("run").cloned().unwrap_or(JsonValue::Null);
    let meta_counts = meta.get("counts").cloned().unwrap_or(JsonValue::Null);
    let meta_connection_snapshot = meta.get("connectionSnapshot").cloned();

    let run_driver = meta_run
        .get("driver")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let run_include_results = meta_run
        .get("includeResults")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let run_results_source = meta_run
        .get("resultsSource")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let run_duration_ms = meta_run
        .get("durationMs")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let count_profiles = meta_counts
        .get("profiles")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let count_points = meta_counts
        .get("points")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let count_results = meta_counts
        .get("results")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let count_conflicts = meta_counts
        .get("conflicts")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let decisions_obj = meta_counts.get("decisions").cloned().unwrap_or(json!({}));
    let decisions_reused_keyv2 = decisions_obj
        .get("reusedKeyV2")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let decisions_reused_keyv2_no_device = decisions_obj
        .get("reusedKeyV2NoDevice")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let decisions_reused_keyv1 = decisions_obj
        .get("reusedKeyV1")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let decisions_created_new = decisions_obj
        .get("createdNew")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let headers_digest = headers_digest_sha256(&request.export_response);

    // union xlsx 输入可追溯（可选）：用于现场确认“是哪一份联合表/哪些列被读取”。
    let union_xlsx_path_original = request.union_xlsx_path.clone().unwrap_or_default();
    let parsed_columns_used = request.parsed_columns_used.clone();
    let mut union_xlsx_digest = "sha256:unknown".to_string();
    if request.union_xlsx_path.is_some() {
        if union_xlsx_path_original.trim().is_empty() {
            warnings.push("unionXlsxPath is empty; unionXlsxDigest=sha256:unknown".to_string());
        } else {
            let src_path = std::path::PathBuf::from(&union_xlsx_path_original);
            if src_path.exists() {
                match file_digest_sha256(&src_path) {
                    Ok(d) => union_xlsx_digest = d,
                    Err(e) => warnings.push(format!("unionXlsx digest failed: {e}")),
                }
            } else {
                warnings.push(format!("unionXlsx not found: {union_xlsx_path_original}"));
            }
        }
    }

    // itFlags（ENV 快照，若未设置则为 null）
    let it_flags = json!({
        "COMM_IT_ENABLE": std::env::var("COMM_IT_ENABLE").ok(),
        "COMM_IT_TCP_HOST": std::env::var("COMM_IT_TCP_HOST").ok(),
        "COMM_IT_TCP_PORT": std::env::var("COMM_IT_TCP_PORT").ok(),
        "COMM_IT_TCP_UNITID": std::env::var("COMM_IT_TCP_UNITID").ok(),
        "COMM_IT_RTU_PORT": std::env::var("COMM_IT_RTU_PORT").ok(),
        "COMM_IT_RTU_BAUD": std::env::var("COMM_IT_RTU_BAUD").ok(),
        "COMM_IT_RTU_PARITY": std::env::var("COMM_IT_RTU_PARITY").ok(),
        "COMM_IT_RTU_DATABITS": std::env::var("COMM_IT_RTU_DATABITS").ok(),
        "COMM_IT_RTU_STOPBITS": std::env::var("COMM_IT_RTU_STOPBITS").ok(),
        "COMM_IT_RTU_SLAVEID": std::env::var("COMM_IT_RTU_SLAVEID").ok(),
    });

    let xlsx_path_original = request
        .exported_xlsx_path
        .clone()
        .or_else(|| {
            request
                .export_response
                .get("outPath")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
        })
        .unwrap_or_default();

    if copy_xlsx {
        if xlsx_path_original.trim().is_empty() {
            warnings.push("exportedXlsxPath missing; xlsx not copied".to_string());
        } else {
            let src_path = std::path::PathBuf::from(&xlsx_path_original);
            if src_path.exists() {
                if src_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("xlsx"))
                    != Some(true)
                {
                    warnings.push(format!(
                        "exportedXlsxPath is not .xlsx; skipped copy: {xlsx_path_original}"
                    ));
                } else {
                    let file_name = src_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "通讯地址表.xlsx".to_string());
                    let dst_path = evidence_dir.join(file_name);
                    match std::fs::copy(&src_path, &dst_path) {
                        Ok(_) => files.push(dst_path),
                        Err(e) => warnings.push(format!("copy xlsx failed: {e}")),
                    }
                }
            } else {
                warnings.push(format!("xlsx not found: {xlsx_path_original}"));
            }
        }
    }

    let ir_path_original = request.ir_path.clone().unwrap_or_default();
    let mut ir_digest = "sha256:unknown".to_string();
    let mut copied_ir_path: Option<String> = None;

    if !ir_path_original.trim().is_empty() {
        let src_path = std::path::PathBuf::from(&ir_path_original);
        if src_path.exists() {
            match file_digest_sha256(&src_path) {
                Ok(d) => ir_digest = d,
                Err(e) => warnings.push(format!("ir digest failed: {e}")),
            }

            if copy_ir {
                let file_name = src_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "comm_ir.v1.json".to_string());
                let dst_path = evidence_dir.join(file_name);
                match std::fs::copy(&src_path, &dst_path) {
                    Ok(_) => {
                        copied_ir_path = Some(dst_path.to_string_lossy().to_string());
                        files.push(dst_path);
                    }
                    Err(e) => warnings.push(format!("copy ir failed: {e}")),
                }
            }
        } else {
            warnings.push(format!("ir not found: {ir_path_original}"));
        }
    } else {
        warnings.push("irPath missing; irDigest=sha256:unknown".to_string());
    }

    let plc_bridge_path_original = request.plc_bridge_path.clone().unwrap_or_default();
    let mut plc_bridge_digest = "sha256:unknown".to_string();
    let mut copied_plc_bridge_path: Option<String> = None;

    if request.plc_bridge_path.is_some() {
        if plc_bridge_path_original.trim().is_empty() {
            warnings.push("plcBridgePath is empty; plcBridgeDigest=sha256:unknown".to_string());
        } else {
            let src_path = std::path::PathBuf::from(&plc_bridge_path_original);
            if src_path.exists() {
                match file_digest_sha256(&src_path) {
                    Ok(d) => plc_bridge_digest = d,
                    Err(e) => warnings.push(format!("plcBridge digest failed: {e}")),
                }

                if copy_plc_bridge {
                    let file_name = src_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "plc_import_bridge.v1.json".to_string());
                    let dst_path = evidence_dir.join(file_name);
                    match std::fs::copy(&src_path, &dst_path) {
                        Ok(_) => {
                            copied_plc_bridge_path = Some(dst_path.to_string_lossy().to_string());
                            files.push(dst_path);
                        }
                        Err(e) => warnings.push(format!("copy plcBridge failed: {e}")),
                    }
                }
            } else {
                warnings.push(format!("plcBridge not found: {plc_bridge_path_original}"));
            }
        }
    }

    let import_result_stub_path_original =
        request.import_result_stub_path.clone().unwrap_or_default();
    let mut import_result_stub_digest = "sha256:unknown".to_string();
    let mut copied_import_result_stub_path: Option<String> = None;

    if request.import_result_stub_path.is_some() {
        if import_result_stub_path_original.trim().is_empty() {
            warnings.push(
                "importResultStubPath is empty; importResultStubDigest=sha256:unknown".to_string(),
            );
        } else {
            let src_path = std::path::PathBuf::from(&import_result_stub_path_original);
            if src_path.exists() {
                match file_digest_sha256(&src_path) {
                    Ok(d) => import_result_stub_digest = d,
                    Err(e) => warnings.push(format!("importResultStub digest failed: {e}")),
                }

                if copy_import_result_stub {
                    let file_name = src_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "import_result_stub.v1.json".to_string());
                    let dst_path = evidence_dir.join(file_name);
                    match std::fs::copy(&src_path, &dst_path) {
                        Ok(_) => {
                            copied_import_result_stub_path =
                                Some(dst_path.to_string_lossy().to_string());
                            files.push(dst_path);
                        }
                        Err(e) => warnings.push(format!("copy importResultStub failed: {e}")),
                    }
                }
            } else {
                warnings.push(format!(
                    "importResultStub not found: {import_result_stub_path_original}"
                ));
            }
        }
    }

    let unified_import_path_original = request.unified_import_path.clone().unwrap_or_default();
    let mut unified_import_digest = "sha256:unknown".to_string();
    let mut copied_unified_import_path: Option<String> = None;

    if request.unified_import_path.is_some() {
        if unified_import_path_original.trim().is_empty() {
            warnings
                .push("unifiedImportPath is empty; unifiedImportDigest=sha256:unknown".to_string());
        } else {
            let src_path = std::path::PathBuf::from(&unified_import_path_original);
            if src_path.exists() {
                match file_digest_sha256(&src_path) {
                    Ok(d) => unified_import_digest = d,
                    Err(e) => warnings.push(format!("unifiedImport digest failed: {e}")),
                }

                if copy_unified_import {
                    let file_name = src_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unified_import.v1.json".to_string());
                    let dst_path = evidence_dir.join(file_name);
                    match std::fs::copy(&src_path, &dst_path) {
                        Ok(_) => {
                            copied_unified_import_path =
                                Some(dst_path.to_string_lossy().to_string());
                            files.push(dst_path);
                        }
                        Err(e) => warnings.push(format!("copy unifiedImport failed: {e}")),
                    }
                }
            } else {
                warnings.push(format!(
                    "unifiedImport not found: {unified_import_path_original}"
                ));
            }
        }
    }

    let merge_report_path_original = request.merge_report_path.clone().unwrap_or_default();
    let mut merge_report_digest = "sha256:unknown".to_string();
    let mut copied_merge_report_path: Option<String> = None;

    if request.merge_report_path.is_some() {
        if merge_report_path_original.trim().is_empty() {
            warnings.push("mergeReportPath is empty; mergeReportDigest=sha256:unknown".to_string());
        } else {
            let src_path = std::path::PathBuf::from(&merge_report_path_original);
            if src_path.exists() {
                match file_digest_sha256(&src_path) {
                    Ok(d) => merge_report_digest = d,
                    Err(e) => warnings.push(format!("mergeReport digest failed: {e}")),
                }

                if copy_merge_report {
                    let file_name = src_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "merge_report.v1.json".to_string());
                    let dst_path = evidence_dir.join(file_name);
                    match std::fs::copy(&src_path, &dst_path) {
                        Ok(_) => {
                            copied_merge_report_path = Some(dst_path.to_string_lossy().to_string());
                            files.push(dst_path);
                        }
                        Err(e) => warnings.push(format!("copy mergeReport failed: {e}")),
                    }
                }
            } else {
                warnings.push(format!(
                    "mergeReport not found: {merge_report_path_original}"
                ));
            }
        }
    }

    let plc_import_stub_path_original = request.plc_import_stub_path.clone().unwrap_or_default();
    let mut plc_import_stub_digest = "sha256:unknown".to_string();
    let mut copied_plc_import_stub_path: Option<String> = None;

    if request.plc_import_stub_path.is_some() {
        if plc_import_stub_path_original.trim().is_empty() {
            warnings
                .push("plcImportStubPath is empty; plcImportStubDigest=sha256:unknown".to_string());
        } else {
            let src_path = std::path::PathBuf::from(&plc_import_stub_path_original);
            if src_path.exists() {
                match file_digest_sha256(&src_path) {
                    Ok(d) => plc_import_stub_digest = d,
                    Err(e) => warnings.push(format!("plcImportStub digest failed: {e}")),
                }

                if copy_plc_import_stub {
                    let file_name = src_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "plc_import.v1.json".to_string());
                    let dst_path = evidence_dir.join(file_name);
                    match std::fs::copy(&src_path, &dst_path) {
                        Ok(_) => {
                            copied_plc_import_stub_path =
                                Some(dst_path.to_string_lossy().to_string());
                            files.push(dst_path);
                        }
                        Err(e) => warnings.push(format!("copy plcImportStub failed: {e}")),
                    }
                }
            } else {
                warnings.push(format!(
                    "plcImportStub not found: {plc_import_stub_path_original}"
                ));
            }
        }
    }

    let copied_xlsx_path = files
        .iter()
        .find(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("xlsx"))
                == Some(true)
        })
        .map(|p| p.to_string_lossy().to_string());

    let zip_path_expected = evidence_dir.join("evidence.zip");
    let zip_path_text = if zip_enabled {
        zip_path_expected.to_string_lossy().to_string()
    } else {
        "".to_string()
    };

    let output_dir_text = output_dir.to_string_lossy().to_string();
    let xlsx_path_rel = if xlsx_path_original.trim().is_empty() {
        None
    } else {
        path_resolver::rel_if_under(&output_dir, std::path::Path::new(&xlsx_path_original))
    };
    let copied_xlsx_path_rel = copied_xlsx_path
        .as_deref()
        .and_then(|p| path_resolver::rel_if_under(&output_dir, std::path::Path::new(p)));
    let evidence_zip_rel = if zip_enabled {
        path_resolver::rel_if_under(&output_dir, &zip_path_expected)
    } else {
        None
    };
    let ir_path_rel = if ir_path_original.trim().is_empty() {
        None
    } else {
        path_resolver::rel_if_under(&output_dir, std::path::Path::new(&ir_path_original))
    };
    let copied_ir_path_rel = copied_ir_path
        .as_deref()
        .and_then(|p| path_resolver::rel_if_under(&output_dir, std::path::Path::new(p)));
    let plc_bridge_path_rel = if plc_bridge_path_original.trim().is_empty() {
        None
    } else {
        path_resolver::rel_if_under(&output_dir, std::path::Path::new(&plc_bridge_path_original))
    };
    let copied_plc_bridge_path_rel = copied_plc_bridge_path
        .as_deref()
        .and_then(|p| path_resolver::rel_if_under(&output_dir, std::path::Path::new(p)));
    let import_result_stub_path_rel = if import_result_stub_path_original.trim().is_empty() {
        None
    } else {
        path_resolver::rel_if_under(
            &output_dir,
            std::path::Path::new(&import_result_stub_path_original),
        )
    };
    let copied_import_result_stub_path_rel = copied_import_result_stub_path
        .as_deref()
        .and_then(|p| path_resolver::rel_if_under(&output_dir, std::path::Path::new(p)));
    let unified_import_path_rel = if unified_import_path_original.trim().is_empty() {
        None
    } else {
        path_resolver::rel_if_under(
            &output_dir,
            std::path::Path::new(&unified_import_path_original),
        )
    };
    let copied_unified_import_path_rel = copied_unified_import_path
        .as_deref()
        .and_then(|p| path_resolver::rel_if_under(&output_dir, std::path::Path::new(p)));
    let merge_report_path_rel = if merge_report_path_original.trim().is_empty() {
        None
    } else {
        path_resolver::rel_if_under(
            &output_dir,
            std::path::Path::new(&merge_report_path_original),
        )
    };
    let copied_merge_report_path_rel = copied_merge_report_path
        .as_deref()
        .and_then(|p| path_resolver::rel_if_under(&output_dir, std::path::Path::new(p)));
    let plc_import_stub_path_rel = if plc_import_stub_path_original.trim().is_empty() {
        None
    } else {
        path_resolver::rel_if_under(
            &output_dir,
            std::path::Path::new(&plc_import_stub_path_original),
        )
    };
    let copied_plc_import_stub_path_rel = copied_plc_import_stub_path
        .as_deref()
        .and_then(|p| path_resolver::rel_if_under(&output_dir, std::path::Path::new(p)));

    let manifest = json!({
        "createdAtUtc": manifest_created_at,
        "app": {
            "appName": app_name,
            "appVersion": app_version,
            "gitCommit": git_commit,
        },
        "inputs": {
            "unionXlsxPath": if union_xlsx_path_original.trim().is_empty() { JsonValue::Null } else { JsonValue::String(union_xlsx_path_original.clone()) },
            "unionXlsxDigest": union_xlsx_digest,
            "parsedColumnsUsed": parsed_columns_used,
        },
        "schema": {
            "profilesSchemaVersion": 1,
            "pointsSchemaVersion": 1,
            "resultsSchemaVersion": 1,
        },
        "run": {
            "driver": run_driver,
            "includeResults": run_include_results,
            "resultsSource": run_results_source,
            "durationMs": run_duration_ms,
        },
        "counts": {
            "profiles": count_profiles,
            "points": count_points,
            "results": count_results,
            "decisions": {
                "reused:keyV2": decisions_reused_keyv2,
                "reused:keyV2NoDevice": decisions_reused_keyv2_no_device,
                "reused:keyV1": decisions_reused_keyv1,
                "created:new": decisions_created_new,
            },
            "conflicts": count_conflicts,
        },
        "outputs": {
            "outputDir": output_dir_text,
            "xlsxPath": xlsx_path_original,
            "xlsxPathRel": xlsx_path_rel,
            "evidenceZipPath": zip_path_text,
            "evidenceZipPathRel": evidence_zip_rel,
            "copiedXlsxPath": copied_xlsx_path,
            "copiedXlsxPathRel": copied_xlsx_path_rel,
            "irPath": ir_path_original,
            "irPathRel": ir_path_rel,
            "irDigest": ir_digest,
            "copiedIrPath": copied_ir_path,
            "copiedIrPathRel": copied_ir_path_rel,
            "plcBridgePath": plc_bridge_path_original,
            "plcBridgePathRel": plc_bridge_path_rel,
            "plcBridgeDigest": plc_bridge_digest,
            "copiedPlcBridgePath": copied_plc_bridge_path,
            "copiedPlcBridgePathRel": copied_plc_bridge_path_rel,
            "importResultStubPath": import_result_stub_path_original,
            "importResultStubPathRel": import_result_stub_path_rel,
            "importResultStubDigest": import_result_stub_digest,
            "copiedImportResultStubPath": copied_import_result_stub_path,
            "copiedImportResultStubPathRel": copied_import_result_stub_path_rel,
            "unifiedImportPath": unified_import_path_original,
            "unifiedImportPathRel": unified_import_path_rel,
            "unifiedImportDigest": unified_import_digest,
            "copiedUnifiedImportPath": copied_unified_import_path,
            "copiedUnifiedImportPathRel": copied_unified_import_path_rel,
            "mergeReportPath": merge_report_path_original,
            "mergeReportPathRel": merge_report_path_rel,
            "mergeReportDigest": merge_report_digest,
            "copiedMergeReportPath": copied_merge_report_path,
            "copiedMergeReportPathRel": copied_merge_report_path_rel,
            "plcImportStubPath": plc_import_stub_path_original,
            "plcImportStubPathRel": plc_import_stub_path_rel,
            "plcImportStubDigest": plc_import_stub_digest,
            "copiedPlcImportStubPath": copied_plc_import_stub_path,
            "copiedPlcImportStubPathRel": copied_plc_import_stub_path_rel,
            "headersDigest": headers_digest,
        },
        "connectionSnapshot": meta_connection_snapshot,
        "itFlags": it_flags,
    });

    let manifest_path = evidence_dir.join("manifest.json");
    write_json_atomic(&manifest_path, &manifest)
        .map_err(|e| format!("write manifest.json failed: {e}"))?;
    files.push(manifest_path);

    // ------- evidence_summary.v1.json（回归/签收摘要） -------
    let mut summary_points: u32 = 0;
    let mut summary_matched: u32 = 0;
    let mut summary_unmatched: u32 = 0;
    let mut summary_comm_covered: u32 = 0;
    let mut summary_verified: u32 = 0;

    // 从 unified_import / plc_import_stub 中提取统计（若缺失则为 0；不 panic）。
    let unified_for_counts = if !unified_import_path_original.trim().is_empty()
        && std::path::Path::new(&unified_import_path_original).exists()
    {
        Some(std::path::PathBuf::from(&unified_import_path_original))
    } else {
        copied_unified_import_path
            .as_deref()
            .map(std::path::PathBuf::from)
            .filter(|p| p.exists())
    };
    if let Some(p) = unified_for_counts {
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<JsonValue>(&text) {
                summary_points = v
                    .get("points")
                    .and_then(|x| x.as_array())
                    .map(|a| a.len() as u32)
                    .unwrap_or(0);
                summary_matched = v
                    .get("statistics")
                    .and_then(|s| s.get("matched"))
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as u32;
                summary_unmatched = v
                    .get("statistics")
                    .and_then(|s| s.get("unmatchedStub"))
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as u32;
            }
        }
    }

    let plc_stub_for_counts = if !plc_import_stub_path_original.trim().is_empty()
        && std::path::Path::new(&plc_import_stub_path_original).exists()
    {
        Some(std::path::PathBuf::from(&plc_import_stub_path_original))
    } else {
        copied_plc_import_stub_path
            .as_deref()
            .map(std::path::PathBuf::from)
            .filter(|p| p.exists())
    };
    if let Some(p) = plc_stub_for_counts {
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<JsonValue>(&text) {
                summary_comm_covered = v
                    .get("statistics")
                    .and_then(|s| s.get("commCovered"))
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as u32;
                summary_verified = v
                    .get("statistics")
                    .and_then(|s| s.get("verified"))
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as u32;
            }
        }
    }

    let evidence_summary = json!({
        "schemaVersion": 1,
        "specVersion": "v1",
        "createdAtUtc": manifest_created_at,
        "app": {
            "appName": app_name,
            "appVersion": app_version,
            "gitCommit": git_commit,
        },
        "inputs": {
            "unionXlsxPath": if union_xlsx_path_original.trim().is_empty() { JsonValue::Null } else { JsonValue::String(union_xlsx_path_original.clone()) },
            "unionXlsxDigest": union_xlsx_digest,
            "parsedColumnsUsed": parsed_columns_used,
        },
        "digests": {
            "headersDigest": headers_digest,
            "irDigest": ir_digest,
            "plcBridgeDigest": plc_bridge_digest,
            "importResultStubDigest": import_result_stub_digest,
            "unifiedImportDigest": unified_import_digest,
            "mergeReportDigest": merge_report_digest,
            "plcImportStubDigest": plc_import_stub_digest,
        },
        "counts": {
            "points": summary_points,
            "matched": summary_matched,
            "unmatched": summary_unmatched,
            "commCovered": summary_comm_covered,
            "verified": summary_verified,
        },
        "warnings": {
            "packWarnings": warnings.clone(),
        }
    });

    let evidence_summary_path = evidence_dir.join("evidence_summary.v1.json");
    write_json_atomic(&evidence_summary_path, &evidence_summary)
        .map_err(|e| format!("write evidence_summary.v1.json failed: {e}"))?;
    files.push(evidence_summary_path);

    // 生成 evidence.zip（必须包含 manifest.json）
    let zip_path = if zip_enabled {
        if let Err(e) = build_zip(&zip_path_expected, &files) {
            warnings.push(format!("zip failed: {e}"));
            None
        } else {
            Some(zip_path_expected)
        }
    } else {
        None
    };

    let mut file_names: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect();
    if let Some(p) = &zip_path {
        if let Some(n) = p.file_name().map(|n| n.to_string_lossy().to_string()) {
            file_names.push(n);
        }
    }
    file_names.sort();

    Ok(CommEvidencePackResponse {
        evidence_dir: evidence_dir.to_string_lossy().to_string(),
        zip_path: zip_path.map(|p| p.to_string_lossy().to_string()),
        manifest,
        files: file_names,
        warnings: if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        },
    })
}

pub(crate) fn verify_evidence_pack_v1(path: &std::path::Path) -> CommEvidenceVerifyV1Response {
    if !path.exists() {
        return CommEvidenceVerifyV1Response {
            ok: false,
            checks: Vec::new(),
            errors: vec![EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::PathNotFound,
                message: format!("path not found: {}", path.to_string_lossy()),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some(path.to_string_lossy().to_string()),
                    ..Default::default()
                }),
            }],
        };
    }

    if path.is_dir() {
        let accessor = DirEvidenceAccessor {
            root: path.to_path_buf(),
        };
        return verify_evidence_pack_with_accessor(accessor);
    }

    let file = match std::fs::File::open(path) {
        Ok(v) => v,
        Err(e) => {
            return CommEvidenceVerifyV1Response {
                ok: false,
                checks: Vec::new(),
                errors: vec![EvidenceVerifyError {
                    kind: EvidenceVerifyErrorKind::ZipReadError,
                    message: format!("open failed: {e}"),
                    details: Some(EvidenceVerifyErrorDetails {
                        file_name: Some(path.to_string_lossy().to_string()),
                        ..Default::default()
                    }),
                }],
            }
        }
    };

    let zip = match zip::ZipArchive::new(file) {
        Ok(v) => v,
        Err(e) => {
            return CommEvidenceVerifyV1Response {
                ok: false,
                checks: Vec::new(),
                errors: vec![EvidenceVerifyError {
                    kind: EvidenceVerifyErrorKind::ZipReadError,
                    message: format!("zip read failed: {e}"),
                    details: Some(EvidenceVerifyErrorDetails {
                        file_name: Some(path.to_string_lossy().to_string()),
                        ..Default::default()
                    }),
                }],
            }
        }
    };

    verify_evidence_pack_with_accessor(ZipEvidenceAccessor { zip })
}

fn verify_evidence_pack_with_accessor(
    mut accessor: impl EvidenceAccessor,
) -> CommEvidenceVerifyV1Response {
    let mut checks: Vec<EvidenceVerifyCheck> = Vec::new();
    let mut errors: Vec<EvidenceVerifyError> = Vec::new();

    let manifest_bytes = match accessor.read_bytes("manifest.json") {
        Ok(v) => {
            checks.push(EvidenceVerifyCheck {
                name: "manifest.json:present".to_string(),
                ok: true,
                message: "manifest.json found".to_string(),
            });
            v
        }
        Err(e) => {
            checks.push(EvidenceVerifyCheck {
                name: "manifest.json:present".to_string(),
                ok: false,
                message: format!("manifest.json missing: {e}"),
            });
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::ManifestMissing,
                message: format!("manifest.json missing: {e}"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("manifest.json".to_string()),
                    ..Default::default()
                }),
            });
            return CommEvidenceVerifyV1Response {
                ok: false,
                checks,
                errors,
            };
        }
    };

    let manifest_text = match String::from_utf8(manifest_bytes) {
        Ok(v) => v,
        Err(e) => {
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::ManifestParseError,
                message: format!("manifest.json is not valid utf8: {e}"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("manifest.json".to_string()),
                    ..Default::default()
                }),
            });
            return CommEvidenceVerifyV1Response {
                ok: false,
                checks,
                errors,
            };
        }
    };

    let manifest: JsonValue = match serde_json::from_str(&manifest_text) {
        Ok(v) => {
            checks.push(EvidenceVerifyCheck {
                name: "manifest.json:parse".to_string(),
                ok: true,
                message: "manifest.json parsed".to_string(),
            });
            v
        }
        Err(e) => {
            checks.push(EvidenceVerifyCheck {
                name: "manifest.json:parse".to_string(),
                ok: false,
                message: format!("manifest.json parse error: {e}"),
            });
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::ManifestParseError,
                message: format!("manifest.json parse error: {e}"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("manifest.json".to_string()),
                    ..Default::default()
                }),
            });
            return CommEvidenceVerifyV1Response {
                ok: false,
                checks,
                errors,
            };
        }
    };

    let evidence_summary_bytes = match accessor.read_bytes("evidence_summary.v1.json") {
        Ok(v) => {
            checks.push(EvidenceVerifyCheck {
                name: "evidence_summary.v1.json:present".to_string(),
                ok: true,
                message: "evidence_summary.v1.json found".to_string(),
            });
            v
        }
        Err(e) => {
            checks.push(EvidenceVerifyCheck {
                name: "evidence_summary.v1.json:present".to_string(),
                ok: false,
                message: format!("evidence_summary.v1.json missing: {e}"),
            });
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::EvidenceSummaryMissing,
                message: format!("evidence_summary.v1.json missing: {e}"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("evidence_summary.v1.json".to_string()),
                    ..Default::default()
                }),
            });
            return CommEvidenceVerifyV1Response {
                ok: false,
                checks,
                errors,
            };
        }
    };

    let evidence_summary_text = match String::from_utf8(evidence_summary_bytes) {
        Ok(v) => v,
        Err(e) => {
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::EvidenceSummaryParseError,
                message: format!("evidence_summary.v1.json is not valid utf8: {e}"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("evidence_summary.v1.json".to_string()),
                    ..Default::default()
                }),
            });
            return CommEvidenceVerifyV1Response {
                ok: false,
                checks,
                errors,
            };
        }
    };

    let evidence_summary: JsonValue = match serde_json::from_str(&evidence_summary_text) {
        Ok(v) => {
            checks.push(EvidenceVerifyCheck {
                name: "evidence_summary.v1.json:parse".to_string(),
                ok: true,
                message: "evidence_summary.v1.json parsed".to_string(),
            });
            v
        }
        Err(e) => {
            checks.push(EvidenceVerifyCheck {
                name: "evidence_summary.v1.json:parse".to_string(),
                ok: false,
                message: format!("evidence_summary.v1.json parse error: {e}"),
            });
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::EvidenceSummaryParseError,
                message: format!("evidence_summary.v1.json parse error: {e}"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("evidence_summary.v1.json".to_string()),
                    ..Default::default()
                }),
            });
            return CommEvidenceVerifyV1Response {
                ok: false,
                checks,
                errors,
            };
        }
    };

    let summary_schema_ok = evidence_summary
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        == 1
        && evidence_summary
            .get("specVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            == "v1";
    checks.push(EvidenceVerifyCheck {
        name: "evidence_summary.v1.json:schema".to_string(),
        ok: summary_schema_ok,
        message: if summary_schema_ok {
            "schemaVersion/specVersion ok".to_string()
        } else {
            "schemaVersion/specVersion mismatch".to_string()
        },
    });
    if !summary_schema_ok {
        errors.push(EvidenceVerifyError {
            kind: EvidenceVerifyErrorKind::SchemaMismatch,
            message: "evidence_summary.v1.json schemaVersion/specVersion mismatch".to_string(),
            details: Some(EvidenceVerifyErrorDetails {
                file_name: Some("evidence_summary.v1.json".to_string()),
                ..Default::default()
            }),
        });
    }

    for name in ["pipeline_log.json", "export_response.json"] {
        let ok = accessor.exists(name);
        checks.push(EvidenceVerifyCheck {
            name: format!("{name}:present"),
            ok,
            message: if ok {
                format!("{name} found")
            } else {
                format!("{name} missing")
            },
        });
        if !ok {
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::FileMissing,
                message: format!("{name} missing"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some(name.to_string()),
                    ..Default::default()
                }),
            });
        }
    }

    let outputs = match manifest.get("outputs").and_then(|v| v.as_object()) {
        Some(v) => v,
        None => {
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::ManifestParseError,
                message: "manifest.outputs missing or not an object".to_string(),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("manifest.json".to_string()),
                    ..Default::default()
                }),
            });
            return CommEvidenceVerifyV1Response {
                ok: errors.is_empty(),
                checks,
                errors,
            };
        }
    };

    let targets: Vec<(&str, &str, &str, &str)> = vec![
        ("ir", "irDigest", "copiedIrPath", "irPath"),
        (
            "plcBridge",
            "plcBridgeDigest",
            "copiedPlcBridgePath",
            "plcBridgePath",
        ),
        (
            "importResultStub",
            "importResultStubDigest",
            "copiedImportResultStubPath",
            "importResultStubPath",
        ),
        (
            "unifiedImport",
            "unifiedImportDigest",
            "copiedUnifiedImportPath",
            "unifiedImportPath",
        ),
        (
            "mergeReport",
            "mergeReportDigest",
            "copiedMergeReportPath",
            "mergeReportPath",
        ),
        (
            "plcImportStub",
            "plcImportStubDigest",
            "copiedPlcImportStubPath",
            "plcImportStubPath",
        ),
    ];

    let mut unified_json: Option<JsonValue> = None;
    let mut plc_stub_json: Option<JsonValue> = None;

    for (label, digest_key, copied_key, original_key) in targets {
        let expected = outputs
            .get(digest_key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !should_verify_digest(&expected) {
            checks.push(EvidenceVerifyCheck {
                name: format!("{label}:digest"),
                ok: true,
                message: "skipped (digest unknown)".to_string(),
            });
            continue;
        }

        let file_name = evidence_file_name_from_path_value(outputs.get(copied_key))
            .or_else(|| evidence_file_name_from_path_value(outputs.get(original_key)));
        let Some(file_name) = file_name else {
            checks.push(EvidenceVerifyCheck {
                name: format!("{label}:digest"),
                ok: false,
                message: "file name missing in manifest outputs".to_string(),
            });
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::FileMissing,
                message: format!("{label} file name missing in manifest outputs"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some(format!("{copied_key}/{original_key}")),
                    ..Default::default()
                }),
            });
            continue;
        };

        if !accessor.exists(&file_name) {
            checks.push(EvidenceVerifyCheck {
                name: format!("{label}:digest"),
                ok: false,
                message: format!("{file_name} missing"),
            });
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::FileMissing,
                message: format!("{file_name} missing"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some(file_name.clone()),
                    ..Default::default()
                }),
            });
            continue;
        }

        let bytes = match accessor.read_bytes(&file_name) {
            Ok(v) => v,
            Err(e) => {
                checks.push(EvidenceVerifyCheck {
                    name: format!("{label}:digest"),
                    ok: false,
                    message: format!("read failed: {e}"),
                });
                errors.push(EvidenceVerifyError {
                    kind: EvidenceVerifyErrorKind::ZipReadError,
                    message: format!("read {file_name} failed: {e}"),
                    details: Some(EvidenceVerifyErrorDetails {
                        file_name: Some(file_name.clone()),
                        ..Default::default()
                    }),
                });
                continue;
            }
        };

        let actual = sha256_digest_prefixed_bytes(&bytes);
        if actual != expected {
            checks.push(EvidenceVerifyCheck {
                name: format!("{label}:digest"),
                ok: false,
                message: "digest mismatch".to_string(),
            });
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::DigestMismatch,
                message: format!("{label} digest mismatch"),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some(file_name.clone()),
                    expected: Some(expected.clone()),
                    actual: Some(actual.clone()),
                    ..Default::default()
                }),
            });
            continue;
        }

        checks.push(EvidenceVerifyCheck {
            name: format!("{label}:digest"),
            ok: true,
            message: "digest ok".to_string(),
        });

        if label == "unifiedImport"
            || label == "plcImportStub"
            || label == "mergeReport"
            || label == "ir"
        {
            if let Ok(text) = String::from_utf8(bytes) {
                if let Ok(v) = serde_json::from_str::<JsonValue>(&text) {
                    match label {
                        "unifiedImport" => unified_json = Some(v),
                        "plcImportStub" => plc_stub_json = Some(v),
                        _ => {}
                    }
                }
            }
        }
    }

    if let Some(v) = &unified_json {
        let ok = v.get("schemaVersion").and_then(|n| n.as_u64()).unwrap_or(0) == 1
            && v.get("specVersion").and_then(|s| s.as_str()).unwrap_or("") == "v1";
        checks.push(EvidenceVerifyCheck {
            name: "unifiedImport:schema".to_string(),
            ok,
            message: if ok { "schema ok" } else { "schema mismatch" }.to_string(),
        });
        if !ok {
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::SchemaMismatch,
                message: "unifiedImport schemaVersion/specVersion mismatch".to_string(),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("unifiedImport".to_string()),
                    ..Default::default()
                }),
            });
        }
    }

    if let Some(v) = &plc_stub_json {
        let ok = v.get("schemaVersion").and_then(|n| n.as_u64()).unwrap_or(0) == 1
            && v.get("specVersion").and_then(|s| s.as_str()).unwrap_or("") == "v1";
        checks.push(EvidenceVerifyCheck {
            name: "plcImportStub:schema".to_string(),
            ok,
            message: if ok { "schema ok" } else { "schema mismatch" }.to_string(),
        });
        if !ok {
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::SchemaMismatch,
                message: "plcImportStub schemaVersion/specVersion mismatch".to_string(),
                details: Some(EvidenceVerifyErrorDetails {
                    file_name: Some("plcImportStub".to_string()),
                    ..Default::default()
                }),
            });
        }
    }

    if let (Some(unified), Some(stub)) = (&unified_json, &plc_stub_json) {
        let unified_names: Vec<String> = match unified.get("points").and_then(|p| p.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|p| {
                    p.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect(),
            None => Vec::new(),
        };
        let stub_names: Vec<String> = match stub.get("points").and_then(|p| p.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|p| {
                    p.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect(),
            None => Vec::new(),
        };

        let ok = unified_names == stub_names;
        checks.push(EvidenceVerifyCheck {
            name: "pointsOrder:unified_vs_plc_stub".to_string(),
            ok,
            message: if ok {
                "points order ok".to_string()
            } else {
                "points order mismatch".to_string()
            },
        });
        if !ok {
            errors.push(EvidenceVerifyError {
                kind: EvidenceVerifyErrorKind::PointsOrderMismatch,
                message: "points order mismatch between unified_import and plc_import_stub"
                    .to_string(),
                details: Some(EvidenceVerifyErrorDetails {
                    message: Some(format!(
                        "unified points={} plc_stub points={}",
                        unified_names.len(),
                        stub_names.len()
                    )),
                    ..Default::default()
                }),
            });
        }
    }

    CommEvidenceVerifyV1Response {
        ok: errors.is_empty(),
        checks,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::comm::adapters::storage::storage;
    use crate::comm::core::model::CommConfigV1;
    use crate::comm::core::union_spec_v1;
    use crate::comm::error::ImportUnionErrorKind;
    use crate::comm::tauri_api::comm_import_union_xlsx;
    use crate::comm::usecase::import_union_xlsx;
    use rust_xlsxwriter::Workbook;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    fn temp_xlsx_path(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "plccodeforge_tauri_api_{prefix}_{}.xlsx",
            Uuid::new_v4()
        ))
    }

    fn write_xlsx(path: &Path, sheet_name: &str, headers: &[&str], rows: &[Vec<&str>]) {
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_name(sheet_name).unwrap();

        for (col, header) in headers.iter().enumerate() {
            sheet.write_string(0, col as u16, *header).unwrap();
        }

        for (row_idx, row) in rows.iter().enumerate() {
            let excel_row = (row_idx + 1) as u32;
            for (col, value) in row.iter().enumerate() {
                sheet.write_string(excel_row, col as u16, *value).unwrap();
            }
        }

        workbook.save(path).unwrap();
    }

    #[tokio::test]
    async fn import_union_strict_missing_sheet_returns_structured_error_object() {
        let path = temp_xlsx_path("missing_sheet");
        write_xlsx(
            &path,
            "OtherSheet",
            &union_spec_v1::REQUIRED_COLUMNS_V1,
            &[],
        );

        let resp = comm_import_union_xlsx(
            path.to_string_lossy().to_string(),
            Some(import_union_xlsx::ImportUnionOptions {
                strict: Some(true),
                sheet_name: Some(union_spec_v1::DEFAULT_SHEET_V1.to_string()),
                address_base: None,
            }),
        )
        .await;

        assert_eq!(resp.ok, Some(false));
        let err = resp.error.expect("error must exist when ok=false");
        assert_eq!(err.kind, ImportUnionErrorKind::UnionXlsxInvalidSheet);
        assert!(err.message.contains("sheet not found"));

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn import_union_strict_missing_columns_returns_missing_columns_details() {
        let path = temp_xlsx_path("missing_columns");
        let headers = [
            "变量名称（HMI）",
            "数据类型",
            // 缺少：字节序
            "通道名称",
            "协议类型",
            "设备标识",
        ];
        write_xlsx(&path, union_spec_v1::DEFAULT_SHEET_V1, &headers, &[]);

        let resp = comm_import_union_xlsx(
            path.to_string_lossy().to_string(),
            Some(import_union_xlsx::ImportUnionOptions {
                strict: Some(true),
                sheet_name: None,
                address_base: None,
            }),
        )
        .await;

        assert_eq!(resp.ok, Some(false));
        let err = resp.error.expect("error must exist when ok=false");
        assert_eq!(err.kind, ImportUnionErrorKind::UnionXlsxMissingColumns);
        let missing = err
            .details
            .and_then(|d| d.missing_columns)
            .unwrap_or_default();
        assert!(missing.iter().any(|c| c == "字节序"));

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn import_union_strict_invalid_enum_returns_row_column_raw_and_allowed_values() {
        let path = temp_xlsx_path("invalid_enum");
        let headers = union_spec_v1::REQUIRED_COLUMNS_V1;
        let rows = vec![vec!["TEMP_1", "BADTYPE", "ABCD", "tcp-1", "TCP", "1"]];
        write_xlsx(&path, union_spec_v1::DEFAULT_SHEET_V1, &headers, &rows);

        let resp = comm_import_union_xlsx(
            path.to_string_lossy().to_string(),
            Some(import_union_xlsx::ImportUnionOptions {
                strict: Some(true),
                sheet_name: None,
                address_base: None,
            }),
        )
        .await;

        assert_eq!(resp.ok, Some(false));
        let err = resp.error.expect("error must exist when ok=false");
        assert_eq!(err.kind, ImportUnionErrorKind::UnionXlsxInvalidEnum);
        let details = err.details.expect("details must exist for invalid enum");
        assert_eq!(details.row_index, Some(2));
        assert_eq!(details.column_name.as_deref(), Some("数据类型"));
        assert_eq!(details.raw_value.as_deref(), Some("BADTYPE"));
        assert!(details.allowed_values.unwrap_or_default().len() >= 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn evidence_pack_manifest_includes_ir_path_and_digest_and_output_dir() {
        let base_dir =
            std::env::temp_dir().join(format!("plc-codeforge-evidence-{}", Uuid::new_v4()));
        let output_dir = storage::default_output_dir(&base_dir);

        // 写入 config.v1.json（显式指定 outputDir，避免默认逻辑回归）
        storage::save_config(
            &base_dir,
            &CommConfigV1 {
                schema_version: 1,
                output_dir: output_dir.to_string_lossy().to_string(),
            },
        )
        .unwrap();

        let xlsx_path = output_dir.join("xlsx").join("通讯地址表.test.xlsx");
        std::fs::create_dir_all(xlsx_path.parent().unwrap()).unwrap();
        std::fs::write(&xlsx_path, b"dummy xlsx").unwrap();

        let ir_path = output_dir.join("ir").join("comm_ir.v1.test.json");
        std::fs::create_dir_all(ir_path.parent().unwrap()).unwrap();
        std::fs::write(
            &ir_path,
            br#"{"schemaVersion":1,"generatedAtUtc":"2026-01-01T00:00:00Z"}"#,
        )
        .unwrap();

        let plc_bridge_path = output_dir
            .join("bridge")
            .join("plc_import_bridge.v1.test.json");
        std::fs::create_dir_all(plc_bridge_path.parent().unwrap()).unwrap();
        std::fs::write(
            &plc_bridge_path,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[]}"#,
        )
        .unwrap();

        let stub_path = output_dir
            .join("bridge_importresult_stub")
            .join("import_result_stub.v1.test.json");
        std::fs::create_dir_all(stub_path.parent().unwrap()).unwrap();
        std::fs::write(
            &stub_path,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[]}"#,
        )
        .unwrap();

        let unified_import_path = output_dir
            .join("unified_import")
            .join("unified_import.v1.test.json");
        std::fs::create_dir_all(unified_import_path.parent().unwrap()).unwrap();
        std::fs::write(
            &unified_import_path,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[]}"#,
        )
        .unwrap();

        let merge_report_path = output_dir
            .join("unified_import")
            .join("merge_report.v1.test.json");
        std::fs::create_dir_all(merge_report_path.parent().unwrap()).unwrap();
        std::fs::write(
            &merge_report_path,
            br#"{"schemaVersion":1,"specVersion":"v1","matchedCount":0}"#,
        )
        .unwrap();

        let plc_import_stub_path = output_dir
            .join("plc_import_stub")
            .join("plc_import.v1.test.json");
        std::fs::create_dir_all(plc_import_stub_path.parent().unwrap()).unwrap();
        std::fs::write(
            &plc_import_stub_path,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[]}"#,
        )
        .unwrap();

        let request = CommEvidencePackRequest {
            pipeline_log: json!([{ "step": "demo", "status": "ok" }]),
            export_response: json!({
                "outPath": xlsx_path.to_string_lossy().to_string(),
                "headers": { "tcp": ["A"], "rtu": ["B"], "params": ["C"] },
                "resultsStatus": "written"
            }),
            conflict_report: None,
            meta: Some(json!({
                "run": { "driver": "mock", "includeResults": true, "resultsSource": "runLatest", "durationMs": 1234 },
                "counts": { "profiles": 1, "points": 1, "results": 1, "decisions": { "reusedKeyV2": 0, "reusedKeyV2NoDevice": 0, "reusedKeyV1": 0, "createdNew": 1 }, "conflicts": 0 }
            })),
            exported_xlsx_path: Some(xlsx_path.to_string_lossy().to_string()),
            ir_path: Some(ir_path.to_string_lossy().to_string()),
            plc_bridge_path: Some(plc_bridge_path.to_string_lossy().to_string()),
            import_result_stub_path: Some(stub_path.to_string_lossy().to_string()),
            unified_import_path: Some(unified_import_path.to_string_lossy().to_string()),
            merge_report_path: Some(merge_report_path.to_string_lossy().to_string()),
            plc_import_stub_path: Some(plc_import_stub_path.to_string_lossy().to_string()),
            union_xlsx_path: Some(xlsx_path.to_string_lossy().to_string()),
            parsed_columns_used: Some(vec!["变量名称（HMI）".to_string(), "数据类型".to_string()]),
            copy_xlsx: Some(true),
            copy_ir: Some(true),
            copy_plc_bridge: Some(true),
            copy_import_result_stub: Some(true),
            copy_unified_import: Some(true),
            copy_merge_report: Some(true),
            copy_plc_import_stub: Some(true),
            zip: Some(true),
        };

        let resp =
            create_evidence_pack(&base_dir, &request, "com.example.app", "0.1.0", "deadbeef")
                .unwrap();

        assert!(!resp.evidence_dir.is_empty());
        assert!(std::path::Path::new(&resp.evidence_dir).exists());
        assert!(
            resp.zip_path
                .as_deref()
                .map(|p| std::path::Path::new(p).exists())
                == Some(true)
        );

        let outputs = resp
            .manifest
            .get("outputs")
            .expect("manifest.outputs must exist");
        let inputs = resp
            .manifest
            .get("inputs")
            .expect("manifest.inputs must exist");
        let output_dir_text = output_dir.to_string_lossy().to_string();
        let ir_path_text = ir_path.to_string_lossy().to_string();
        let plc_bridge_path_text = plc_bridge_path.to_string_lossy().to_string();
        let stub_path_text = stub_path.to_string_lossy().to_string();
        let unified_import_path_text = unified_import_path.to_string_lossy().to_string();
        let merge_report_path_text = merge_report_path.to_string_lossy().to_string();
        let plc_import_stub_path_text = plc_import_stub_path.to_string_lossy().to_string();
        assert_eq!(
            outputs.get("outputDir").and_then(|v| v.as_str()),
            Some(output_dir_text.as_str())
        );
        assert_eq!(
            outputs.get("irPath").and_then(|v| v.as_str()),
            Some(ir_path_text.as_str())
        );
        assert!(outputs
            .get("irDigest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("sha256:"));
        assert_eq!(
            outputs.get("plcBridgePath").and_then(|v| v.as_str()),
            Some(plc_bridge_path_text.as_str())
        );
        assert!(outputs
            .get("plcBridgeDigest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("sha256:"));
        assert_eq!(
            outputs.get("importResultStubPath").and_then(|v| v.as_str()),
            Some(stub_path_text.as_str())
        );
        assert_eq!(
            outputs.get("unifiedImportPath").and_then(|v| v.as_str()),
            Some(unified_import_path_text.as_str())
        );
        assert!(outputs
            .get("unifiedImportDigest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("sha256:"));
        assert_eq!(
            outputs.get("mergeReportPath").and_then(|v| v.as_str()),
            Some(merge_report_path_text.as_str())
        );
        assert!(outputs
            .get("mergeReportDigest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("sha256:"));
        assert_eq!(
            outputs.get("plcImportStubPath").and_then(|v| v.as_str()),
            Some(plc_import_stub_path_text.as_str())
        );
        assert!(outputs
            .get("plcImportStubDigest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("sha256:"));
        assert!(outputs
            .get("importResultStubDigest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("sha256:"));

        assert!(inputs
            .get("unionXlsxDigest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("sha256:"));
        assert!(inputs.get("parsedColumnsUsed").is_some());

        let evidence_summary_path =
            std::path::Path::new(&resp.evidence_dir).join("evidence_summary.v1.json");
        assert!(evidence_summary_path.exists());
    }

    #[test]
    fn evidence_verify_v1_reports_ok_and_detects_digest_mismatch_after_tamper() {
        let base_dir =
            std::env::temp_dir().join(format!("plc-codeforge-evidence-verify-{}", Uuid::new_v4()));
        let output_dir = storage::default_output_dir(&base_dir);
        storage::save_config(
            &base_dir,
            &CommConfigV1 {
                schema_version: 1,
                output_dir: output_dir.to_string_lossy().to_string(),
            },
        )
        .unwrap();

        let xlsx_path = output_dir.join("xlsx").join("通讯地址表.test.xlsx");
        std::fs::create_dir_all(xlsx_path.parent().unwrap()).unwrap();
        std::fs::write(&xlsx_path, b"dummy xlsx").unwrap();

        let ir_path = output_dir.join("ir").join("comm_ir.v1.test.json");
        std::fs::create_dir_all(ir_path.parent().unwrap()).unwrap();
        std::fs::write(
            &ir_path,
            br#"{"schemaVersion":1,"generatedAtUtc":"2026-01-01T00:00:00Z"}"#,
        )
        .unwrap();

        let plc_bridge_path = output_dir
            .join("bridge")
            .join("plc_import_bridge.v1.test.json");
        std::fs::create_dir_all(plc_bridge_path.parent().unwrap()).unwrap();
        std::fs::write(
            &plc_bridge_path,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[]}"#,
        )
        .unwrap();

        let stub_path = output_dir
            .join("bridge_importresult_stub")
            .join("import_result_stub.v1.test.json");
        std::fs::create_dir_all(stub_path.parent().unwrap()).unwrap();
        std::fs::write(
            &stub_path,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[]}"#,
        )
        .unwrap();

        let unified_import_path = output_dir
            .join("unified_import")
            .join("unified_import.v1.test.json");
        std::fs::create_dir_all(unified_import_path.parent().unwrap()).unwrap();
        std::fs::write(
            &unified_import_path,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[{"name":"P1"}]}"#,
        )
        .unwrap();

        let merge_report_path = output_dir
            .join("unified_import")
            .join("merge_report.v1.test.json");
        std::fs::create_dir_all(merge_report_path.parent().unwrap()).unwrap();
        std::fs::write(
            &merge_report_path,
            br#"{"schemaVersion":1,"specVersion":"v1","matchedCount":1}"#,
        )
        .unwrap();

        let plc_import_stub_path = output_dir
            .join("plc_import_stub")
            .join("plc_import.v1.test.json");
        std::fs::create_dir_all(plc_import_stub_path.parent().unwrap()).unwrap();
        std::fs::write(
            &plc_import_stub_path,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[{"name":"P1"}]}"#,
        )
        .unwrap();

        let request = CommEvidencePackRequest {
            pipeline_log: json!([{ "step": "demo", "status": "ok" }]),
            export_response: json!({
                "outPath": xlsx_path.to_string_lossy().to_string(),
                "headers": { "tcp": ["A"], "rtu": ["B"], "params": ["C"] },
                "resultsStatus": "written"
            }),
            conflict_report: None,
            meta: Some(json!({
                "run": { "driver": "mock", "includeResults": true, "resultsSource": "runLatest", "durationMs": 1234 },
                "counts": { "profiles": 1, "points": 1, "results": 1, "decisions": { "reusedKeyV2": 0, "reusedKeyV2NoDevice": 0, "reusedKeyV1": 0, "createdNew": 1 }, "conflicts": 0 }
            })),
            exported_xlsx_path: Some(xlsx_path.to_string_lossy().to_string()),
            ir_path: Some(ir_path.to_string_lossy().to_string()),
            plc_bridge_path: Some(plc_bridge_path.to_string_lossy().to_string()),
            import_result_stub_path: Some(stub_path.to_string_lossy().to_string()),
            unified_import_path: Some(unified_import_path.to_string_lossy().to_string()),
            merge_report_path: Some(merge_report_path.to_string_lossy().to_string()),
            plc_import_stub_path: Some(plc_import_stub_path.to_string_lossy().to_string()),
            union_xlsx_path: Some(xlsx_path.to_string_lossy().to_string()),
            parsed_columns_used: Some(vec!["变量名称（HMI）".to_string(), "数据类型".to_string()]),
            copy_xlsx: Some(true),
            copy_ir: Some(true),
            copy_plc_bridge: Some(true),
            copy_import_result_stub: Some(true),
            copy_unified_import: Some(true),
            copy_merge_report: Some(true),
            copy_plc_import_stub: Some(true),
            zip: Some(true),
        };

        let resp =
            create_evidence_pack(&base_dir, &request, "com.example.app", "0.1.0", "deadbeef")
                .unwrap();

        let dir_verify = verify_evidence_pack_v1(std::path::Path::new(&resp.evidence_dir));
        assert!(dir_verify.ok);

        let zip_path = resp
            .zip_path
            .clone()
            .expect("zipPath must exist for verify test");
        let zip_verify = verify_evidence_pack_v1(std::path::Path::new(&zip_path));
        assert!(zip_verify.ok);

        // 篡改一个被记录 digest 的文件（unified_import），应当触发 digest mismatch
        let copied_unified =
            std::path::Path::new(&resp.evidence_dir).join("unified_import.v1.test.json");
        std::fs::write(
            &copied_unified,
            br#"{"schemaVersion":1,"specVersion":"v1","points":[{"name":"P1"},{"name":"P2"}]}"#,
        )
        .unwrap();

        let tampered = verify_evidence_pack_v1(std::path::Path::new(&resp.evidence_dir));
        assert!(!tampered.ok);
        assert!(tampered
            .errors
            .iter()
            .any(|e| e.kind == EvidenceVerifyErrorKind::DigestMismatch));

        // 仅用于验收文档：默认 cargo test 不显示；需要时可用 -- --nocapture 查看
        println!(
            "VERIFY_OK_DIR={}",
            serde_json::to_string_pretty(&dir_verify).unwrap()
        );
        println!(
            "VERIFY_OK_ZIP={}",
            serde_json::to_string_pretty(&zip_verify).unwrap()
        );
        println!(
            "VERIFY_TAMPERED={}",
            serde_json::to_string_pretty(&tampered).unwrap()
        );
    }
}
