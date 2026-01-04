//! Modbus TCP 驱动（真实读段）。
//!
//! MVP：支持 Holding / Input / Coil / Discrete 的读取；上层 engine 负责 timeout/retry。

use super::{CommDriver, DriverError, DriverFuture, RawReadData};
use crate::comm::model::{ConnectionProfile, RegisterArea};
use crate::comm::plan::ReadJob;

use tokio_modbus::client::tcp;
use tokio_modbus::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct ModbusTcpDriver;

impl ModbusTcpDriver {
    pub fn new() -> Self {
        Self
    }
}

impl CommDriver for ModbusTcpDriver {
    fn read<'a>(&'a self, profile: &'a ConnectionProfile, job: &'a ReadJob) -> DriverFuture<'a> {
        Box::pin(async move {
            let (ip, port, unit_id) = match profile {
                ConnectionProfile::Tcp {
                    ip,
                    port,
                    device_id,
                    ..
                } => (ip.as_str(), *port, *device_id),
                _ => {
                    return Err(DriverError::Comm {
                        message: "ModbusTcpDriver requires a TCP profile".to_string(),
                    });
                }
            };

            let socket_addr = format!("{ip}:{port}").parse().map_err(|e| DriverError::Comm {
                message: format!("invalid socket addr: {e}"),
            })?;

            let slave = Slave(unit_id);
            let mut ctx = tcp::connect_slave(socket_addr, slave)
                .await
                .map_err(|e| DriverError::Comm {
                    message: e.to_string(),
                })?;

            match job.read_area {
                RegisterArea::Holding => {
                    let data = ctx
                        .read_holding_registers(job.start_address, job.length)
                        .await
                        .map_err(|e| DriverError::Comm {
                            message: e.to_string(),
                        })?;
                    Ok(RawReadData::Registers(data))
                }
                RegisterArea::Input => {
                    let data = ctx
                        .read_input_registers(job.start_address, job.length)
                        .await
                        .map_err(|e| DriverError::Comm {
                            message: e.to_string(),
                        })?;
                    Ok(RawReadData::Registers(data))
                }
                RegisterArea::Coil => {
                    let data = ctx
                        .read_coils(job.start_address, job.length)
                        .await
                        .map_err(|e| DriverError::Comm {
                            message: e.to_string(),
                        })?;
                    Ok(RawReadData::Coils(data))
                }
                RegisterArea::Discrete => {
                    let data = ctx
                        .read_discrete_inputs(job.start_address, job.length)
                        .await
                        .map_err(|e| DriverError::Comm {
                            message: e.to_string(),
                        })?;
                    Ok(RawReadData::Coils(data))
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;

    use crate::comm::plan::ReadJob;

    #[tokio::test]
    async fn it_can_read_holding_registers_when_enabled() {
        if env::var("COMM_IT_ENABLE").ok().as_deref() != Some("1") {
            return;
        }

        let host = match env::var("COMM_IT_TCP_HOST") {
            Ok(v) => v,
            Err(_) => return,
        };
        let port: u16 = match env::var("COMM_IT_TCP_PORT").ok().and_then(|v| v.parse().ok()) {
            Some(v) => v,
            None => return,
        };
        let unit_id: u8 = match env::var("COMM_IT_TCP_UNITID").ok().and_then(|v| v.parse().ok()) {
            Some(v) => v,
            None => return,
        };

        let driver = ModbusTcpDriver::new();
        let profile = ConnectionProfile::Tcp {
            channel_name: "it-tcp".to_string(),
            device_id: unit_id,
            read_area: RegisterArea::Holding,
            start_address: 0,
            length: 10,
            ip: host,
            port,
            timeout_ms: 1000,
            retry_count: 0,
            poll_interval_ms: 500,
        };
        let job = ReadJob {
            channel_name: "it-tcp".to_string(),
            read_area: RegisterArea::Holding,
            start_address: 0,
            length: 2,
            points: vec![],
        };

        let raw = driver.read(&profile, &job).await;
        assert!(matches!(raw, Ok(RawReadData::Registers(_))));
    }
}

