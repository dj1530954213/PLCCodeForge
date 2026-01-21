use tauri::{AppHandle, Manager, State};

use crate::comm::adapters::storage::projects;
use crate::comm::adapters::storage::storage;
use crate::comm::core::model::{
    CommDeviceV1, CommProjectDataV1, ConnectionProfile, PointsV1, ProfilesV1, SCHEMA_VERSION_V1,
};
use crate::comm::tauri_api::state::CommState;

pub(crate) fn scope_key(project_id: Option<&str>, device_id: Option<&str>) -> String {
    match (project_id, device_id) {
        (Some(project_id), Some(device_id)) => format!("{project_id}:{device_id}"),
        (Some(project_id), None) => project_id.to_string(),
        (None, _) => "legacy".to_string(),
    }
}

pub(crate) fn comm_base_dir(
    app: &AppHandle,
    project_id: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    if let Some(v) = project_id {
        projects::validate_project_id(v)?;
    }

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(match project_id {
        Some(project_id) => projects::project_comm_dir(&app_data_dir, project_id),
        None => storage::comm_dir(app_data_dir),
    })
}

pub(crate) fn load_project_data_if_needed(
    app: &AppHandle,
    project_id: Option<&str>,
) -> Result<Option<CommProjectDataV1>, String> {
    let Some(project_id) = project_id else {
        return Ok(None);
    };
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    projects::load_project_data(&app_data_dir, project_id).map(Some)
}

pub(crate) fn profiles_from_device(device: &CommDeviceV1) -> ProfilesV1 {
    ProfilesV1 {
        schema_version: SCHEMA_VERSION_V1,
        profiles: vec![device.profile.clone()],
    }
}

pub(crate) fn resolve_project_device<'a>(
    project: &'a CommProjectDataV1,
    device_id: Option<&str>,
) -> Result<Option<&'a CommDeviceV1>, String> {
    let Some(devices) = project.devices.as_ref() else {
        if device_id.is_some() {
            return Err("deviceId provided but project.devices is missing".to_string());
        }
        return Ok(None);
    };
    if devices.is_empty() {
        if device_id.is_some() {
            return Err("project.devices is empty".to_string());
        }
        return Ok(None);
    }

    if let Some(device_id) = device_id {
        return devices
            .iter()
            .find(|d| d.device_id == device_id)
            .map(Some)
            .ok_or_else(|| format!("device not found: {device_id}"));
    }

    if let Some(active_id) = project
        .ui_state
        .as_ref()
        .and_then(|ui| ui.active_device_id.as_deref())
    {
        if let Some(device) = devices.iter().find(|d| d.device_id == active_id) {
            return Ok(Some(device));
        }
    }

    Ok(devices.first())
}

pub(crate) fn find_device_mut<'a>(
    project: &'a mut CommProjectDataV1,
    device_id: &str,
) -> Result<&'a mut CommDeviceV1, String> {
    let Some(devices) = project.devices.as_mut() else {
        return Err("project.devices is empty".to_string());
    };
    devices
        .iter_mut()
        .find(|d| d.device_id == device_id)
        .ok_or_else(|| format!("device not found: {device_id}"))
}

pub(crate) fn resolve_profiles(
    base_dir: &std::path::Path,
    state: &State<'_, CommState>,
    scope: &str,
    payload: Option<ProfilesV1>,
    project_data: Option<&CommProjectDataV1>,
    device_id: Option<&str>,
) -> Result<ProfilesV1, String> {
    if let Some(payload) = payload {
        if payload.schema_version != SCHEMA_VERSION_V1 {
            return Err(format!(
                "unsupported schemaVersion: {}",
                payload.schema_version
            ));
        }
        return Ok(payload);
    }

    if let Some(project) = project_data {
        if let Some(device) = resolve_project_device(project, device_id)? {
            let profiles = profiles_from_device(device);
            state.memory.lock().scope_mut(scope).profiles = Some(profiles.clone());
            return Ok(profiles);
        }

        if let Some(profiles) = project.connections.clone() {
            if profiles.schema_version != SCHEMA_VERSION_V1 {
                return Err(format!(
                    "unsupported connections.schemaVersion: {}",
                    profiles.schema_version
                ));
            }
            state.memory.lock().scope_mut(scope).profiles = Some(profiles.clone());
            return Ok(profiles);
        }
    }

    if let Some(v) = storage::load_profiles(&base_dir).map_err(|e| e.to_string())? {
        state.memory.lock().scope_mut(scope).profiles = Some(v.clone());
        return Ok(v);
    }

    state
        .memory
        .lock()
        .scope(scope)
        .and_then(|v| v.profiles.clone())
        .ok_or_else(|| "profiles not provided and not saved".to_string())
}

pub(crate) fn resolve_points(
    base_dir: &std::path::Path,
    state: &State<'_, CommState>,
    scope: &str,
    payload: Option<PointsV1>,
    project_data: Option<&CommProjectDataV1>,
    device_id: Option<&str>,
) -> Result<PointsV1, String> {
    if let Some(payload) = payload {
        if payload.schema_version != SCHEMA_VERSION_V1 {
            return Err(format!(
                "unsupported schemaVersion: {}",
                payload.schema_version
            ));
        }
        return Ok(payload);
    }

    if let Some(project) = project_data {
        if let Some(device) = resolve_project_device(project, device_id)? {
            let points = device.points.clone();
            if points.schema_version != SCHEMA_VERSION_V1 {
                return Err(format!(
                    "unsupported device points.schemaVersion: {}",
                    points.schema_version
                ));
            }
            state.memory.lock().scope_mut(scope).points = Some(points.clone());
            return Ok(points);
        }

        if let Some(points) = project.points.clone() {
            if points.schema_version != SCHEMA_VERSION_V1 {
                return Err(format!(
                    "unsupported points.schemaVersion: {}",
                    points.schema_version
                ));
            }
            state.memory.lock().scope_mut(scope).points = Some(points.clone());
            return Ok(points);
        }
    }

    if let Some(v) = storage::load_points(&base_dir).map_err(|e| e.to_string())? {
        state.memory.lock().scope_mut(scope).points = Some(v.clone());
        return Ok(v);
    }

    state
        .memory
        .lock()
        .scope(scope)
        .and_then(|v| v.points.clone())
        .ok_or_else(|| "points not provided and not saved".to_string())
}

pub(crate) fn profiles_min_poll_interval_ms(profiles: &[ConnectionProfile]) -> Option<u32> {
    profiles
        .iter()
        .map(|p| match p {
            ConnectionProfile::Tcp {
                poll_interval_ms, ..
            } => *poll_interval_ms,
            ConnectionProfile::Rtu485 {
                poll_interval_ms, ..
            } => *poll_interval_ms,
        })
        .min()
}
