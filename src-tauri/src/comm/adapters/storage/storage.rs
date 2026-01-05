use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::comm::core::model::{
    CommConfigV1, PointsV1, ProfilesV1, RunStats, SampleResult, SCHEMA_VERSION_V1,
};
use crate::comm::core::plan::ReadPlan;

pub const STORAGE_DIR_NAME: &str = "comm";
pub const PROFILES_FILE_NAME: &str = "profiles.v1.json";
pub const POINTS_FILE_NAME: &str = "points.v1.json";
pub const PLAN_FILE_NAME: &str = "plan.v1.json";
pub const LAST_RESULTS_FILE_NAME: &str = "last_results.v1.json";
pub const CONFIG_FILE_NAME: &str = "config.v1.json";
pub const RUNS_DIR_NAME: &str = "runs";

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unsupported schemaVersion: {0}")]
    UnsupportedSchemaVersion(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanV1 {
    pub schema_version: u32,
    pub plan: ReadPlan,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LastResultsV1 {
    pub schema_version: u32,
    pub results: Vec<SampleResult>,
    pub stats: RunStats,
}

pub fn comm_dir(app_data_dir: PathBuf) -> PathBuf {
    app_data_dir.join(STORAGE_DIR_NAME)
}

pub fn default_output_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("deliveries")
}

pub fn save_config(base_dir: &Path, payload: &CommConfigV1) -> Result<(), StorageError> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(StorageError::UnsupportedSchemaVersion(
            payload.schema_version,
        ));
    }
    write_json_atomic(base_dir.join(CONFIG_FILE_NAME), payload)
}

pub fn load_config(base_dir: &Path) -> Result<Option<CommConfigV1>, StorageError> {
    read_json_optional::<CommConfigV1>(base_dir.join(CONFIG_FILE_NAME)).and_then(|opt| {
        if let Some(v) = &opt {
            if v.schema_version != SCHEMA_VERSION_V1 {
                return Err(StorageError::UnsupportedSchemaVersion(v.schema_version));
            }
        }
        Ok(opt)
    })
}

pub fn save_profiles(base_dir: &Path, payload: &ProfilesV1) -> Result<(), StorageError> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(StorageError::UnsupportedSchemaVersion(
            payload.schema_version,
        ));
    }
    write_json_atomic(base_dir.join(PROFILES_FILE_NAME), payload)
}

pub fn load_profiles(base_dir: &Path) -> Result<Option<ProfilesV1>, StorageError> {
    read_json_optional::<ProfilesV1>(base_dir.join(PROFILES_FILE_NAME)).and_then(|opt| {
        if let Some(v) = &opt {
            if v.schema_version != SCHEMA_VERSION_V1 {
                return Err(StorageError::UnsupportedSchemaVersion(v.schema_version));
            }
        }
        Ok(opt)
    })
}

pub fn save_points(base_dir: &Path, payload: &PointsV1) -> Result<(), StorageError> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(StorageError::UnsupportedSchemaVersion(
            payload.schema_version,
        ));
    }
    write_json_atomic(base_dir.join(POINTS_FILE_NAME), payload)
}

pub fn load_points(base_dir: &Path) -> Result<Option<PointsV1>, StorageError> {
    read_json_optional::<PointsV1>(base_dir.join(POINTS_FILE_NAME)).and_then(|opt| {
        if let Some(v) = &opt {
            if v.schema_version != SCHEMA_VERSION_V1 {
                return Err(StorageError::UnsupportedSchemaVersion(v.schema_version));
            }
        }
        Ok(opt)
    })
}

pub fn save_plan(base_dir: &Path, plan: &ReadPlan) -> Result<(), StorageError> {
    let payload = PlanV1 {
        schema_version: SCHEMA_VERSION_V1,
        plan: plan.clone(),
    };
    write_json_atomic(base_dir.join(PLAN_FILE_NAME), &payload)
}

pub fn load_plan(base_dir: &Path) -> Result<Option<PlanV1>, StorageError> {
    read_json_optional::<PlanV1>(base_dir.join(PLAN_FILE_NAME)).and_then(|opt| {
        if let Some(v) = &opt {
            if v.schema_version != SCHEMA_VERSION_V1 {
                return Err(StorageError::UnsupportedSchemaVersion(v.schema_version));
            }
        }
        Ok(opt)
    })
}

pub fn save_last_results(
    base_dir: &Path,
    results: &[SampleResult],
    stats: &RunStats,
) -> Result<(), StorageError> {
    let payload = LastResultsV1 {
        schema_version: SCHEMA_VERSION_V1,
        results: results.to_vec(),
        stats: stats.clone(),
    };
    write_json_atomic(base_dir.join(LAST_RESULTS_FILE_NAME), &payload)
}

pub fn load_last_results(base_dir: &Path) -> Result<Option<LastResultsV1>, StorageError> {
    read_json_optional::<LastResultsV1>(base_dir.join(LAST_RESULTS_FILE_NAME)).and_then(|opt| {
        if let Some(v) = &opt {
            if v.schema_version != SCHEMA_VERSION_V1 {
                return Err(StorageError::UnsupportedSchemaVersion(v.schema_version));
            }
        }
        Ok(opt)
    })
}

pub fn save_run_last_results(
    base_dir: &Path,
    run_id: Uuid,
    results: &[SampleResult],
    stats: &RunStats,
) -> Result<(), StorageError> {
    let payload = LastResultsV1 {
        schema_version: SCHEMA_VERSION_V1,
        results: results.to_vec(),
        stats: stats.clone(),
    };

    let run_dir = base_dir.join(RUNS_DIR_NAME).join(run_id.to_string());
    write_json_atomic(run_dir.join(LAST_RESULTS_FILE_NAME), &payload)
}

pub fn load_run_last_results(
    base_dir: &Path,
    run_id: Uuid,
) -> Result<Option<LastResultsV1>, StorageError> {
    let path = base_dir
        .join(RUNS_DIR_NAME)
        .join(run_id.to_string())
        .join(LAST_RESULTS_FILE_NAME);

    read_json_optional::<LastResultsV1>(path).and_then(|opt| {
        if let Some(v) = &opt {
            if v.schema_version != SCHEMA_VERSION_V1 {
                return Err(StorageError::UnsupportedSchemaVersion(v.schema_version));
            }
        }
        Ok(opt)
    })
}

fn ensure_dir(path: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(path)
}

fn write_json_atomic<T: Serialize>(path: PathBuf, value: &T) -> Result<(), StorageError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    ensure_dir(parent)?;

    let json = serde_json::to_string_pretty(value)?;
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, json)?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    std::fs::rename(tmp_path, path)?;
    Ok(())
}

fn read_json_optional<T: DeserializeOwned>(path: PathBuf) -> Result<Option<T>, StorageError> {
    if !path.exists() {
        return Ok(None);
    }

    let text = std::fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&text)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::model::{ByteOrder32, CommPoint, ConnectionProfile, DataType, RegisterArea};
    use uuid::Uuid;

    #[test]
    fn appdata_files_are_written_with_schema_version_1() {
        let base_dir = std::env::temp_dir().join(format!("plc-codeforge-comm-{}", Uuid::new_v4()));
        println!("comm storage test dir: {}", base_dir.display());
        let profiles = ProfilesV1 {
            schema_version: SCHEMA_VERSION_V1,
            profiles: vec![ConnectionProfile::Tcp {
                channel_name: "tcp-1".to_string(),
                device_id: 1,
                read_area: RegisterArea::Holding,
                start_address: 0,
                length: 10,
                ip: "127.0.0.1".to_string(),
                port: 502,
                timeout_ms: 1000,
                retry_count: 0,
                poll_interval_ms: 500,
            }],
        };
        save_profiles(&base_dir, &profiles).unwrap();
        let profiles_text = std::fs::read_to_string(base_dir.join(PROFILES_FILE_NAME)).unwrap();
        println!("profiles.v1.json:\n{profiles_text}");

        let points = PointsV1 {
            schema_version: SCHEMA_VERSION_V1,
            points: vec![CommPoint {
                point_key: Uuid::from_u128(1),
                hmi_name: "P1".to_string(),
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                channel_name: "tcp-1".to_string(),
                address_offset: None,
                scale: 1.0,
            }],
        };
        save_points(&base_dir, &points).unwrap();
        let points_text = std::fs::read_to_string(base_dir.join(POINTS_FILE_NAME)).unwrap();
        println!("points.v1.json:\n{points_text}");

        let plan = ReadPlan { jobs: vec![] };
        save_plan(&base_dir, &plan).unwrap();

        let results = vec![SampleResult {
            point_key: Uuid::from_u128(1),
            value_display: "0".to_string(),
            quality: crate::comm::model::Quality::Ok,
            timestamp: chrono::Utc::now(),
            duration_ms: 1,
            error_message: "".to_string(),
        }];
        let stats = RunStats {
            total: 1,
            ok: 1,
            timeout: 0,
            comm_error: 0,
            decode_error: 0,
            config_error: 0,
        };
        save_last_results(&base_dir, &results, &stats).unwrap();

        assert!(base_dir.join(PROFILES_FILE_NAME).exists());
        assert!(base_dir.join(POINTS_FILE_NAME).exists());
        assert!(base_dir.join(PLAN_FILE_NAME).exists());
        assert!(base_dir.join(LAST_RESULTS_FILE_NAME).exists());

        let loaded_profiles = load_profiles(&base_dir).unwrap().unwrap();
        assert_eq!(loaded_profiles.schema_version, 1);

        let loaded_points = load_points(&base_dir).unwrap().unwrap();
        assert_eq!(loaded_points.schema_version, 1);

        let loaded_plan = load_plan(&base_dir).unwrap().unwrap();
        assert_eq!(loaded_plan.schema_version, 1);

        let loaded_results = load_last_results(&base_dir).unwrap().unwrap();
        assert_eq!(loaded_results.schema_version, 1);
    }
}
