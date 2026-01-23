use crate::comm::tauri_api::CommPingResponse;

#[tauri::command]
pub fn comm_ping() -> CommPingResponse {
    CommPingResponse { ok: true }
}
