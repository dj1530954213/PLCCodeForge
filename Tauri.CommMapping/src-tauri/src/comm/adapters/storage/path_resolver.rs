//! 通讯采集模块：交付路径解析（TASK-32）
//!
//! 目标：
//! - 将 XLSX / IR / evidence 的输出统一到一个可配置的 outputDir
//! - 读取配置：AppData/<app>/comm/config.v1.json（schemaVersion=1）
//! - 若未配置，则默认 outputDir = AppData/<app>/comm/deliveries/

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::storage;

pub fn resolve_output_dir(base_dir: &Path) -> PathBuf {
    let default_dir = storage::default_output_dir(base_dir);

    let cfg = match storage::load_config(base_dir) {
        Ok(v) => v,
        Err(_) => return default_dir,
    };

    let Some(cfg) = cfg else {
        return default_dir;
    };

    let raw = cfg.output_dir.trim();
    if raw.is_empty() {
        return default_dir;
    }

    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        base_dir.join(candidate)
    }
}

pub fn ts_label(now: DateTime<Utc>) -> String {
    format!(
        "{}-{}",
        now.format("%Y%m%dT%H%M%SZ"),
        now.timestamp_millis()
    )
}

pub fn default_delivery_xlsx_path(output_dir: &Path, now: DateTime<Utc>) -> PathBuf {
    let ts = ts_label(now);
    output_dir
        .join("xlsx")
        .join(format!("通讯地址表.{ts}.xlsx"))
}

pub fn default_device_delivery_xlsx_path(
    output_dir: &Path,
    now: DateTime<Utc>,
    device_name: &str,
) -> PathBuf {
    let batch = ts_label(now);
    let device = device_name.trim();
    let device_dir = if device.is_empty() { "Device" } else { device };
    output_dir
        .join(batch)
        .join(device_dir)
        .join("通讯地址表.xlsx")
}

pub fn evidence_dir(output_dir: &Path, now: DateTime<Utc>) -> PathBuf {
    output_dir.join("evidence").join(ts_label(now))
}

pub fn ir_dir(output_dir: &Path) -> PathBuf {
    output_dir.join("ir")
}

pub fn bridge_dir(output_dir: &Path) -> PathBuf {
    output_dir.join("bridge")
}

pub fn default_plc_bridge_path(output_dir: &Path, now: DateTime<Utc>) -> PathBuf {
    let ts = ts_label(now);
    bridge_dir(output_dir).join(format!("plc_import_bridge.v1.{ts}.json"))
}

pub fn bridge_check_dir(output_dir: &Path, now: DateTime<Utc>) -> PathBuf {
    output_dir.join("bridge_check").join(ts_label(now))
}

pub fn bridge_importresult_stub_dir(output_dir: &Path) -> PathBuf {
    output_dir.join("bridge_importresult_stub")
}

pub fn default_importresult_stub_path(output_dir: &Path, now: DateTime<Utc>) -> PathBuf {
    let ts = ts_label(now);
    bridge_importresult_stub_dir(output_dir).join(format!("import_result_stub.v1.{ts}.json"))
}

pub fn unified_import_dir(output_dir: &Path) -> PathBuf {
    output_dir.join("unified_import")
}

pub fn default_unified_import_path(output_dir: &Path, now: DateTime<Utc>) -> PathBuf {
    let ts = ts_label(now);
    unified_import_dir(output_dir).join(format!("unified_import.v1.{ts}.json"))
}

pub fn default_merge_report_path(output_dir: &Path, now: DateTime<Utc>) -> PathBuf {
    let ts = ts_label(now);
    unified_import_dir(output_dir).join(format!("merge_report.v1.{ts}.json"))
}

pub fn plc_import_stub_dir(output_dir: &Path) -> PathBuf {
    output_dir.join("plc_import_stub")
}

pub fn default_plc_import_stub_path(output_dir: &Path, now: DateTime<Utc>) -> PathBuf {
    let ts = ts_label(now);
    plc_import_stub_dir(output_dir).join(format!("plc_import.v1.{ts}.json"))
}

pub fn rel_if_under(output_dir: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(output_dir).ok()?;
    Some(rel.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::model::CommConfigV1;
    use crate::comm::storage;
    use uuid::Uuid;

    #[test]
    fn resolve_output_dir_defaults_to_deliveries_when_missing_config() {
        let base_dir = std::env::temp_dir().join(format!("plc-codeforge-path-{}", Uuid::new_v4()));
        let resolved = resolve_output_dir(&base_dir);
        assert_eq!(resolved, base_dir.join("deliveries"));
    }

    #[test]
    fn resolve_output_dir_uses_config_absolute_or_relative() {
        let base_dir = std::env::temp_dir().join(format!("plc-codeforge-path-{}", Uuid::new_v4()));

        let rel = CommConfigV1 {
            schema_version: 1,
            output_dir: "out".to_string(),
        };
        storage::save_config(&base_dir, &rel).unwrap();
        assert_eq!(resolve_output_dir(&base_dir), base_dir.join("out"));

        let abs_dir = std::env::temp_dir().join(format!("plc-codeforge-abs-{}", Uuid::new_v4()));
        let abs = CommConfigV1 {
            schema_version: 1,
            output_dir: abs_dir.to_string_lossy().to_string(),
        };
        storage::save_config(&base_dir, &abs).unwrap();
        assert_eq!(resolve_output_dir(&base_dir), abs_dir);
    }
}
