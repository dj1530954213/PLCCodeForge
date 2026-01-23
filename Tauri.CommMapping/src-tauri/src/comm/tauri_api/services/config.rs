use tauri::{AppHandle, Manager, State};

use crate::comm::adapters::storage::projects;
use crate::comm::model::{CommConfigV1, PointsV1, ProfilesV1, SCHEMA_VERSION_V1};
use crate::comm::plan::build_read_plan;
use crate::comm::storage;
use crate::comm::tauri_api::common::{
    comm_base_dir, find_device_mut, load_project_data_if_needed, resolve_points,
    resolve_profiles, resolve_project_device, scope_key,
};
use crate::comm::tauri_api::{CommPlanBuildRequest, CommState, PlanV1};

pub(crate) fn load_config(
    app: AppHandle,
    project_id: Option<String>,
) -> Result<CommConfigV1, String> {
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let default_dir = storage::default_output_dir(&base_dir)
        .to_string_lossy()
        .to_string();

    match storage::load_config(&base_dir).map_err(|e| e.to_string())? {
        Some(mut cfg) => {
            if cfg.output_dir.trim().is_empty() {
                cfg.output_dir = default_dir;
            }
            Ok(cfg)
        }
        None => Ok(CommConfigV1 {
            schema_version: SCHEMA_VERSION_V1,
            output_dir: default_dir,
        }),
    }
}

pub(crate) fn save_config(
    app: AppHandle,
    payload: CommConfigV1,
    project_id: Option<String>,
) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let default_dir = storage::default_output_dir(&base_dir)
        .to_string_lossy()
        .to_string();

    let mut cfg = payload;
    if cfg.output_dir.trim().is_empty() {
        cfg.output_dir = default_dir;
    }

    storage::save_config(&base_dir, &cfg).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn save_profiles(
    app: AppHandle,
    state: State<'_, CommState>,
    payload: ProfilesV1,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    if let Some(project_id) = project_id.as_deref() {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let mut project = projects::load_project_data(&app_data_dir, project_id)?;
        if let Some(device_id) = device_id.as_deref() {
            let Some(first_profile) = payload.profiles.first().cloned() else {
                return Err("连接配置为空".to_string());
            };
            let device = find_device_mut(&mut project, device_id)?;
            device.profile = first_profile;
            if project
                .devices
                .as_ref()
                .and_then(|list| list.first())
                .map(|v| v.device_id.as_str() == device_id)
                .unwrap_or(false)
            {
                project.connections = Some(payload.clone());
            }
        } else {
            project.connections = Some(payload.clone());
            if let Some(first_device) = project
                .devices
                .as_mut()
                .and_then(|list| list.first_mut())
            {
                if let Some(first_profile) = payload.profiles.first().cloned() {
                    first_device.profile = first_profile;
                }
            }
        }
        projects::save_project_data(&app_data_dir, &project)?;
    } else {
        let base_dir = comm_base_dir(&app, None)?;
        storage::save_profiles(&base_dir, &payload).map_err(|e| e.to_string())?;
    }
    state.memory.lock().scope_mut(&scope).profiles = Some(payload);
    Ok(())
}

pub(crate) fn load_profiles(
    app: AppHandle,
    state: State<'_, CommState>,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<ProfilesV1, String> {
    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    if let Some(project_id) = project_id.as_deref() {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let project = projects::load_project_data(&app_data_dir, project_id)?;
        let profiles = match resolve_project_device(&project, device_id.as_deref())? {
            Some(device) => ProfilesV1 {
                schema_version: SCHEMA_VERSION_V1,
                profiles: vec![device.profile.clone()],
            },
            None => project.connections.unwrap_or(ProfilesV1 {
                schema_version: SCHEMA_VERSION_V1,
                profiles: vec![],
            }),
        };
        state.memory.lock().scope_mut(&scope).profiles = Some(profiles.clone());
        return Ok(profiles);
    }

    let base_dir = comm_base_dir(&app, None)?;
    let loaded = storage::load_profiles(&base_dir).map_err(|e| e.to_string())?;
    if let Some(v) = loaded {
        state.memory.lock().scope_mut(&scope).profiles = Some(v.clone());
        return Ok(v);
    }

    Ok(state
        .memory
        .lock()
        .scope(&scope)
        .and_then(|v| v.profiles.clone())
        .unwrap_or(ProfilesV1 {
            schema_version: SCHEMA_VERSION_V1,
            profiles: vec![],
        }))
}

pub(crate) fn save_points(
    app: AppHandle,
    state: State<'_, CommState>,
    payload: PointsV1,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<(), String> {
    if payload.schema_version != SCHEMA_VERSION_V1 {
        return Err(format!(
            "unsupported schemaVersion: {}",
            payload.schema_version
        ));
    }

    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    if let Some(project_id) = project_id.as_deref() {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let mut project = projects::load_project_data(&app_data_dir, project_id)?;
        if let Some(device_id) = device_id.as_deref() {
            let device = find_device_mut(&mut project, device_id)?;
            device.points = payload.clone();
            if project
                .devices
                .as_ref()
                .and_then(|list| list.first())
                .map(|v| v.device_id.as_str() == device_id)
                .unwrap_or(false)
            {
                project.points = Some(payload.clone());
            }
        } else {
            project.points = Some(payload.clone());
            if let Some(first_device) = project
                .devices
                .as_mut()
                .and_then(|list| list.first_mut())
            {
                first_device.points = payload.clone();
            }
        }
        projects::save_project_data(&app_data_dir, &project)?;
    } else {
        let base_dir = comm_base_dir(&app, None)?;
        storage::save_points(&base_dir, &payload).map_err(|e| e.to_string())?;
    }
    state.memory.lock().scope_mut(&scope).points = Some(payload);
    Ok(())
}

pub(crate) fn load_points(
    app: AppHandle,
    state: State<'_, CommState>,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<PointsV1, String> {
    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    if let Some(project_id) = project_id.as_deref() {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let project = projects::load_project_data(&app_data_dir, project_id)?;
        let points = match resolve_project_device(&project, device_id.as_deref())? {
            Some(device) => device.points.clone(),
            None => project.points.unwrap_or(PointsV1 {
                schema_version: SCHEMA_VERSION_V1,
                points: vec![],
            }),
        };
        state.memory.lock().scope_mut(&scope).points = Some(points.clone());
        return Ok(points);
    }

    let base_dir = comm_base_dir(&app, None)?;
    let loaded = storage::load_points(&base_dir).map_err(|e| e.to_string())?;
    if let Some(v) = loaded {
        state.memory.lock().scope_mut(&scope).points = Some(v.clone());
        return Ok(v);
    }

    Ok(state
        .memory
        .lock()
        .scope(&scope)
        .and_then(|v| v.points.clone())
        .unwrap_or(PointsV1 {
            schema_version: SCHEMA_VERSION_V1,
            points: vec![],
        }))
}

pub(crate) fn build_plan(
    app: AppHandle,
    state: State<'_, CommState>,
    request: CommPlanBuildRequest,
    project_id: Option<String>,
    device_id: Option<String>,
) -> Result<PlanV1, String> {
    let scope = scope_key(project_id.as_deref(), device_id.as_deref());
    let base_dir = comm_base_dir(&app, project_id.as_deref())?;
    let project_data = load_project_data_if_needed(&app, project_id.as_deref())?;
    let profiles = resolve_profiles(
        &base_dir,
        &state,
        &scope,
        request.profiles,
        project_data.as_ref(),
        device_id.as_deref(),
    )?;
    let points = resolve_points(
        &base_dir,
        &state,
        &scope,
        request.points,
        project_data.as_ref(),
        device_id.as_deref(),
    )?;

    let options = request.options.unwrap_or_default();
    let plan =
        build_read_plan(&profiles.profiles, &points.points, options).map_err(|e| e.to_string())?;

    state.memory.lock().scope_mut(&scope).plan = Some(plan.clone());
    storage::save_plan(&base_dir, &plan).map_err(|e| e.to_string())?;
    Ok(PlanV1 {
        schema_version: SCHEMA_VERSION_V1,
        plan,
    })
}
