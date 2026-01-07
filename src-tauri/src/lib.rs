// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#![recursion_limit = "256"]

pub mod comm;

use comm::tauri_api::{
    comm_bridge_consume_check, comm_bridge_export_importresult_stub_v1,
    comm_bridge_to_plc_import_v1, comm_config_load, comm_config_save, comm_evidence_pack_create,
    comm_evidence_verify_v1, comm_export_delivery_xlsx, comm_export_ir_v1, comm_export_xlsx,
    comm_import_union_xlsx, comm_merge_import_sources_v1, comm_ping, comm_plan_build,
    comm_points_load, comm_points_save, comm_profiles_load, comm_profiles_save, comm_project_copy,
    comm_project_create, comm_project_delete, comm_project_get, comm_project_load_v1,
    comm_project_save_v1, comm_project_ui_state_patch_v1, comm_projects_list, comm_run_latest,
    comm_run_latest_obs, comm_run_start, comm_run_start_obs, comm_run_stop, comm_run_stop_obs,
    comm_unified_export_plc_import_stub_v1, CommState,
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CommState::new())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            comm_ping,
            comm_project_create,
            comm_projects_list,
            comm_project_get,
            comm_project_load_v1,
            comm_project_save_v1,
            comm_project_ui_state_patch_v1,
            comm_project_copy,
            comm_project_delete,
            comm_config_load,
            comm_config_save,
            comm_profiles_save,
            comm_profiles_load,
            comm_points_save,
            comm_points_load,
            comm_plan_build,
            comm_run_start,
            comm_run_latest,
            comm_run_stop,
            comm_run_start_obs,
            comm_run_latest_obs,
            comm_run_stop_obs,
            comm_export_xlsx,
            comm_export_delivery_xlsx,
            comm_export_ir_v1,
            comm_bridge_to_plc_import_v1,
            comm_bridge_consume_check,
            comm_bridge_export_importresult_stub_v1,
            comm_merge_import_sources_v1,
            comm_unified_export_plc_import_stub_v1,
            comm_evidence_pack_create,
            comm_evidence_verify_v1,
            comm_import_union_xlsx,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
