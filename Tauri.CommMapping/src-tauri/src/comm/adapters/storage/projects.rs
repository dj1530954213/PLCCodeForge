use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use crate::comm::core::model::{
    CommDeviceV1, CommProjectDataV1, CommProjectV1, ConnectionProfile, PointsV1, ProfilesV1,
    RegisterArea, SCHEMA_VERSION_V1,
};

use super::storage::{self, StorageError};

pub const PROJECTS_DIR_NAME: &str = "projects";
pub const PROJECT_FILE_NAME: &str = "project.v1.json";
pub const EXPORTS_DIR_NAME: &str = "exports";
pub const EVIDENCE_DIR_NAME: &str = "evidence";
pub const RUNS_DIR_NAME: &str = "runs";

pub fn projects_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(PROJECTS_DIR_NAME)
}

pub fn project_dir(app_data_dir: &Path, project_id: &str) -> PathBuf {
    projects_dir(app_data_dir).join(project_id)
}

pub fn project_comm_dir(app_data_dir: &Path, project_id: &str) -> PathBuf {
    project_dir(app_data_dir, project_id).join(storage::STORAGE_DIR_NAME)
}

pub fn project_meta_path(comm_dir: &Path) -> PathBuf {
    comm_dir.join(PROJECT_FILE_NAME)
}

pub fn project_exports_dir(comm_dir: &Path) -> PathBuf {
    comm_dir.join(EXPORTS_DIR_NAME)
}

pub fn project_evidence_dir(comm_dir: &Path) -> PathBuf {
    comm_dir.join(EVIDENCE_DIR_NAME)
}

pub fn project_runs_dir(comm_dir: &Path) -> PathBuf {
    comm_dir.join(RUNS_DIR_NAME)
}

fn sanitize_workbook_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        "Device".to_string()
    } else {
        trimmed
    }
}

fn default_profile() -> ConnectionProfile {
    ConnectionProfile::Tcp {
        channel_name: "tcp-1".to_string(),
        device_id: 1,
        read_area: RegisterArea::Holding,
        start_address: 0,
        length: 1,
        ip: "127.0.0.1".to_string(),
        port: 502,
        timeout_ms: 1000,
        retry_count: 0,
        poll_interval_ms: 500,
    }
}

fn legacy_device_id(project_id: &str) -> String {
    format!("legacy-{project_id}")
}

fn build_device_from_legacy(
    project: &CommProjectV1,
    connections: &ProfilesV1,
    points: &PointsV1,
) -> CommDeviceV1 {
    let device_name = project
        .device
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| project.name.clone());
    let workbook_name = sanitize_workbook_name(&device_name);
    let profile = connections
        .profiles
        .first()
        .cloned()
        .unwrap_or_else(default_profile);

    CommDeviceV1 {
        device_id: legacy_device_id(&project.project_id),
        device_name,
        workbook_name,
        profile,
        points: points.clone(),
        ui_state: None,
    }
}

pub fn validate_project_id(project_id: &str) -> Result<(), String> {
    let id = project_id.trim();
    if id.is_empty() {
        return Err("projectId is empty".to_string());
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("projectId contains invalid path characters".to_string());
    }
    Ok(())
}

pub fn create_project(
    app_data_dir: &Path,
    name: String,
    device: Option<String>,
    notes: Option<String>,
) -> Result<CommProjectV1, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("project name is empty".to_string());
    }

    let project_id = Uuid::new_v4().to_string();
    validate_project_id(&project_id)?;

    let comm_dir = project_comm_dir(app_data_dir, &project_id);
    ensure_project_dirs(&comm_dir).map_err(|e| e.to_string())?;

    let project = CommProjectV1 {
        schema_version: SCHEMA_VERSION_V1,
        project_id: project_id.clone(),
        name,
        device,
        created_at_utc: Utc::now(),
        notes,
        deleted_at_utc: None,
    };

    // Pre-create profiles/points to make "open project" deterministic.
    let default_profiles = ProfilesV1 {
        schema_version: SCHEMA_VERSION_V1,
        profiles: vec![],
    };
    storage::save_profiles(&comm_dir, &default_profiles)
        .map_err(|e| format!("save profiles failed: {e}"))?;

    let empty_points = PointsV1 {
        schema_version: SCHEMA_VERSION_V1,
        points: Vec::new(),
    };
    storage::save_points(&comm_dir, &empty_points)
        .map_err(|e| format!("save points failed: {e}"))?;

    // Single-source-of-truth project file (v1): meta + connections + points (+ optional uiState).
    let project_data = CommProjectDataV1 {
        schema_version: project.schema_version,
        project_id: project.project_id.clone(),
        name: project.name.clone(),
        device: project.device.clone(),
        created_at_utc: project.created_at_utc,
        notes: project.notes.clone(),
        deleted_at_utc: project.deleted_at_utc,
        devices: Some(vec![]),
        connections: Some(default_profiles),
        points: Some(empty_points),
        ui_state: None,
    };
    write_json_atomic(&project_meta_path(&comm_dir), &project_data).map_err(|e| e.to_string())?;

    Ok(project)
}

pub fn load_project_data(
    app_data_dir: &Path,
    project_id: &str,
) -> Result<CommProjectDataV1, String> {
    validate_project_id(project_id)?;
    let comm_dir = project_comm_dir(app_data_dir, project_id);
    let path = project_meta_path(&comm_dir);
    let Some(value) = read_json_optional::<serde_json::Value>(&path).map_err(|e| e.to_string())?
    else {
        return Err(format!("project not found: {project_id}"));
    };

    let meta: CommProjectV1 = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if meta.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            meta.schema_version
        ));
    }
    if meta.deleted_at_utc.is_some() {
        return Err("project deleted".to_string());
    }

    // Prefer project.v1.json embedded fields; fallback to legacy split files when missing.
    let mut connections = if value.get("connections").is_some() {
        serde_json::from_value::<ProfilesV1>(value.get("connections").cloned().unwrap_or_default())
            .map_err(|e| e.to_string())?
    } else {
        storage::load_profiles(&comm_dir)
            .map_err(|e| e.to_string())?
            .unwrap_or(ProfilesV1 {
                schema_version: SCHEMA_VERSION_V1,
                profiles: Vec::new(),
            })
    };

    let mut points = if value.get("points").is_some() {
        serde_json::from_value::<PointsV1>(value.get("points").cloned().unwrap_or_default())
            .map_err(|e| e.to_string())?
    } else {
        storage::load_points(&comm_dir)
            .map_err(|e| e.to_string())?
            .unwrap_or(PointsV1 {
                schema_version: SCHEMA_VERSION_V1,
                points: Vec::new(),
            })
    };

    let devices = value
        .get("devices")
        .and_then(|v| serde_json::from_value::<Vec<CommDeviceV1>>(v.clone()).ok())
        .or_else(|| Some(vec![build_device_from_legacy(&meta, &connections, &points)]));

    if let Some(device) = devices.as_ref().and_then(|list| list.first()) {
        if connections.profiles.is_empty() {
            connections = ProfilesV1 {
                schema_version: SCHEMA_VERSION_V1,
                profiles: vec![device.profile.clone()],
            };
        }
        if points.points.is_empty() {
            points = device.points.clone();
        }
    }

    let ui_state = value
        .get("uiState")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    Ok(CommProjectDataV1 {
        schema_version: meta.schema_version,
        project_id: meta.project_id,
        name: meta.name,
        device: meta.device,
        created_at_utc: meta.created_at_utc,
        notes: meta.notes,
        deleted_at_utc: meta.deleted_at_utc,
        devices,
        connections: Some(connections),
        points: Some(points),
        ui_state,
    })
}

pub fn save_project_data(app_data_dir: &Path, payload: &CommProjectDataV1) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }
    validate_project_id(&payload.project_id)?;

    let mut normalized = payload.clone();

    if let Some(devices) = normalized.devices.as_ref() {
        for device in devices {
            if device.device_id.trim().is_empty() {
                return Err("deviceId is empty".to_string());
            }
            if device.device_name.trim().is_empty() {
                return Err("deviceName is empty".to_string());
            }
            if device.workbook_name.trim().is_empty() {
                return Err("workbookName is empty".to_string());
            }
            if device.points.schema_version != SCHEMA_VERSION_V1 {
                return Err(format!(
                    "unsupported device points.schemaVersion: {}",
                    device.points.schema_version
                ));
            }
        }
    }

    if normalized.connections.is_none() || normalized.points.is_none() {
        if let Some(device) = normalized
            .devices
            .as_ref()
            .and_then(|list| list.first())
        {
            normalized.connections = Some(ProfilesV1 {
                schema_version: SCHEMA_VERSION_V1,
                profiles: vec![device.profile.clone()],
            });
            normalized.points = Some(device.points.clone());
        }
    }

    let Some(connections) = normalized.connections.as_ref() else {
        return Err("connections is required".to_string());
    };
    if connections.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported connections.schemaVersion: {}",
            connections.schema_version
        ));
    }

    let Some(points) = normalized.points.as_ref() else {
        return Err("points is required".to_string());
    };
    if points.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported points.schemaVersion: {}",
            points.schema_version
        ));
    }

    let comm_dir = project_comm_dir(app_data_dir, &payload.project_id);
    ensure_project_dirs(&comm_dir).map_err(|e| e.to_string())?;
    write_json_atomic(&project_meta_path(&comm_dir), &normalized).map_err(|e| e.to_string())?;

    // Keep legacy split files in sync to preserve existing commands/tools.
    storage::save_profiles(&comm_dir, connections).map_err(|e| e.to_string())?;
    storage::save_points(&comm_dir, points).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_project(
    app_data_dir: &Path,
    project_id: &str,
) -> Result<Option<CommProjectV1>, String> {
    validate_project_id(project_id)?;
    let comm_dir = project_comm_dir(app_data_dir, project_id);
    let path = project_meta_path(&comm_dir);
    read_json_optional::<CommProjectV1>(&path)
        .map_err(|e| e.to_string())?
        .map(|project| {
            if project.schema_version != SCHEMA_VERSION_V1 {
                return Err(format!(
                    "unsupported schemaVersion: {}",
                    project.schema_version
                ));
            }
            Ok(project)
        })
        .transpose()
}

pub fn list_projects(
    app_data_dir: &Path,
    include_deleted: bool,
) -> Result<Vec<CommProjectV1>, String> {
    let root = projects_dir(app_data_dir);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut projects: Vec<CommProjectV1> = Vec::new();
    let entries = std::fs::read_dir(&root).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let project_id = match path.file_name().and_then(|v| v.to_str()) {
            Some(v) => v.to_string(),
            None => continue,
        };
        if validate_project_id(&project_id).is_err() {
            continue;
        }

        let comm_dir = path.join(storage::STORAGE_DIR_NAME);
        let meta_path = project_meta_path(&comm_dir);
        let project = match read_json_optional::<CommProjectV1>(&meta_path) {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(_) => continue,
        };
        if project.schema_version != SCHEMA_VERSION_V1 {
            continue;
        }
        if !include_deleted && project.deleted_at_utc.is_some() {
            continue;
        }
        projects.push(project);
    }

    projects.sort_by(|a, b| b.created_at_utc.cmp(&a.created_at_utc));
    Ok(projects)
}

pub fn soft_delete_project(app_data_dir: &Path, project_id: &str) -> Result<CommProjectV1, String> {
    let mut data = load_project_data(app_data_dir, project_id)?;
    if data.deleted_at_utc.is_none() {
        data.deleted_at_utc = Some(Utc::now());
        save_project_data(app_data_dir, &data)?;
    }
    Ok(CommProjectV1 {
        schema_version: data.schema_version,
        project_id: data.project_id,
        name: data.name,
        device: data.device,
        created_at_utc: data.created_at_utc,
        notes: data.notes,
        deleted_at_utc: data.deleted_at_utc,
    })
}

pub fn copy_project(
    app_data_dir: &Path,
    source_project_id: &str,
    new_name: Option<String>,
) -> Result<CommProjectV1, String> {
    let Some(source) = load_project(app_data_dir, source_project_id)? else {
        return Err(format!("project not found: {source_project_id}"));
    };

    let source_data = load_project_data(app_data_dir, source_project_id)?;

    let name = new_name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("{} (copy)", source.name));

    let project = create_project(
        app_data_dir,
        name,
        source.device.clone(),
        source.notes.clone(),
    )?;

    let src_comm_dir = project_comm_dir(app_data_dir, source_project_id);
    let dst_comm_dir = project_comm_dir(app_data_dir, &project.project_id);

    // Copy only config/points/profiles/plan to keep "copy project" small and predictable.
    copy_if_exists(
        src_comm_dir.join(storage::CONFIG_FILE_NAME),
        dst_comm_dir.join(storage::CONFIG_FILE_NAME),
    )?;
    copy_if_exists(
        src_comm_dir.join(storage::PROFILES_FILE_NAME),
        dst_comm_dir.join(storage::PROFILES_FILE_NAME),
    )?;
    copy_if_exists(
        src_comm_dir.join(storage::POINTS_FILE_NAME),
        dst_comm_dir.join(storage::POINTS_FILE_NAME),
    )?;
    copy_if_exists(
        src_comm_dir.join(storage::PLAN_FILE_NAME),
        dst_comm_dir.join(storage::PLAN_FILE_NAME),
    )?;

    let mut project_data = source_data.clone();
    project_data.project_id = project.project_id.clone();
    project_data.name = project.name.clone();
    project_data.device = project.device.clone();
    project_data.created_at_utc = project.created_at_utc;
    project_data.notes = project.notes.clone();
    project_data.deleted_at_utc = project.deleted_at_utc;
    if let Some(devices) = project_data.devices.as_mut() {
        for device in devices {
            device.device_id = Uuid::new_v4().to_string();
            device.workbook_name = sanitize_workbook_name(&device.device_name);
        }
    }
    write_json_atomic(&project_meta_path(&dst_comm_dir), &project_data)
        .map_err(|e| e.to_string())?;

    Ok(project)
}

fn copy_if_exists(src: PathBuf, dst: PathBuf) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }
    let Some(parent) = dst.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    std::fs::copy(src, dst).map_err(|e| e.to_string())?;
    Ok(())
}

fn ensure_project_dirs(comm_dir: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(comm_dir)?;
    std::fs::create_dir_all(project_exports_dir(comm_dir))?;
    std::fs::create_dir_all(project_evidence_dir(comm_dir))?;
    std::fs::create_dir_all(project_runs_dir(comm_dir))?;
    Ok(())
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent)?;

    let json = serde_json::to_string_pretty(value)?;
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, json)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(tmp_path, path)?;
    Ok(())
}

fn read_json_optional<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, StorageError> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&text)?))
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub fn format_utc(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump_tree(root: &Path, indent: usize) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.path());
        for e in entries {
            let p = e.path();
            let name = p.file_name().and_then(|v| v.to_str()).unwrap_or("<?>");
            println!("{}{}", " ".repeat(indent), name);
            if p.is_dir() {
                dump_tree(&p, indent + 2);
            }
        }
    }

    #[test]
    fn create_and_list_projects() {
        let app_data_dir =
            std::env::temp_dir().join(format!("plc-codeforge-projects-{}", Uuid::new_v4()));
        let p = create_project(
            &app_data_dir,
            "P1".to_string(),
            Some("Dev1".to_string()),
            None,
        )
        .unwrap();
        let list = list_projects(&app_data_dir, false).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].project_id, p.project_id);

        let comm_dir = project_comm_dir(&app_data_dir, &p.project_id);
        println!("project test appDataDir: {}", app_data_dir.display());
        println!("projectId: {}", p.project_id);
        println!("--- tree(appDataDir) ---");
        dump_tree(&app_data_dir, 0);
        assert!(project_meta_path(&comm_dir).exists());
        assert!(comm_dir.join(storage::PROFILES_FILE_NAME).exists());
        assert!(comm_dir.join(storage::POINTS_FILE_NAME).exists());
        assert!(project_exports_dir(&comm_dir).exists());
        assert!(project_evidence_dir(&comm_dir).exists());
        assert!(project_runs_dir(&comm_dir).exists());

        let _ = std::fs::remove_dir_all(app_data_dir);
    }
}
