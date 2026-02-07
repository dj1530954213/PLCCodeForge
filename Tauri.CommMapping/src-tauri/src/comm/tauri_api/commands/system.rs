use crate::comm::tauri_api::CommPingResponse;

#[tauri::command]
pub fn comm_ping() -> CommPingResponse {
    CommPingResponse { ok: true }
}

#[tauri::command]
pub fn comm_serial_ports_list() -> Result<Vec<String>, String> {
    let ports = tokio_serial::available_ports().map_err(|e| e.to_string())?;
    let mut names: Vec<String> = ports.into_iter().map(|p| p.port_name).collect();
    names.sort();
    names.dedup();
    Ok(names)
}
