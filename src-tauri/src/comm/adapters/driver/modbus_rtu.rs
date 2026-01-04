//! Modbus RTU（485）驱动（真实读段）。
//!
//! MVP：支持 Holding / Input / Coil / Discrete 的读取；上层 engine 负责 timeout/retry。

use super::{CommDriver, DriverError, DriverFuture, RawReadData};
use crate::comm::model::{ConnectionProfile, RegisterArea, SerialParity};
use crate::comm::plan::ReadJob;

use tokio_modbus::client::rtu;
use tokio_modbus::prelude::*;
use tokio_serial::{DataBits, Parity, SerialStream, StopBits};

#[derive(Clone, Debug, Default)]
pub struct ModbusRtuDriver;

impl ModbusRtuDriver {
    pub fn new() -> Self {
        Self
    }
}

impl CommDriver for ModbusRtuDriver {
    fn read<'a>(&'a self, profile: &'a ConnectionProfile, job: &'a ReadJob) -> DriverFuture<'a> {
        Box::pin(async move {
            let (serial_port, baud_rate, parity, data_bits, stop_bits, slave_id) = match profile {
                ConnectionProfile::Rtu485 {
                    serial_port,
                    baud_rate,
                    parity,
                    data_bits,
                    stop_bits,
                    device_id,
                    ..
                } => (
                    serial_port.as_str(),
                    *baud_rate,
                    parity,
                    *data_bits,
                    *stop_bits,
                    *device_id,
                ),
                _ => {
                    return Err(DriverError::Comm {
                        message: "ModbusRtuDriver requires a 485 profile".to_string(),
                    });
                }
            };

            let mut builder = tokio_serial::new(serial_port, baud_rate);
            builder = builder.parity(map_parity(parity));
            builder = builder.data_bits(map_data_bits(data_bits)?);
            builder = builder.stop_bits(map_stop_bits(stop_bits)?);

            let port = SerialStream::open(&builder).map_err(|e| DriverError::Comm {
                message: e.to_string(),
            })?;

            let slave = Slave(slave_id);
            let mut ctx = rtu::attach_slave(port, slave);

            match job.read_area {
                RegisterArea::Holding => {
                    let data = ctx
                        .read_holding_registers(job.start_address, job.length)
                        .await
                        .map_err(|e| DriverError::Comm {
                            message: e.to_string(),
                        })?
                        .map_err(|e| DriverError::Comm {
                            message: format!("modbus exception: {e}"),
                        })?;
                    Ok(RawReadData::Registers(data))
                }
                RegisterArea::Input => {
                    let data = ctx
                        .read_input_registers(job.start_address, job.length)
                        .await
                        .map_err(|e| DriverError::Comm {
                            message: e.to_string(),
                        })?
                        .map_err(|e| DriverError::Comm {
                            message: format!("modbus exception: {e}"),
                        })?;
                    Ok(RawReadData::Registers(data))
                }
                RegisterArea::Coil => {
                    let data = ctx
                        .read_coils(job.start_address, job.length)
                        .await
                        .map_err(|e| DriverError::Comm {
                            message: e.to_string(),
                        })?
                        .map_err(|e| DriverError::Comm {
                            message: format!("modbus exception: {e}"),
                        })?;
                    Ok(RawReadData::Coils(data))
                }
                RegisterArea::Discrete => {
                    let data = ctx
                        .read_discrete_inputs(job.start_address, job.length)
                        .await
                        .map_err(|e| DriverError::Comm {
                            message: e.to_string(),
                        })?
                        .map_err(|e| DriverError::Comm {
                            message: format!("modbus exception: {e}"),
                        })?;
                    Ok(RawReadData::Coils(data))
                }
            }
        })
    }
}

fn map_parity(parity: &SerialParity) -> Parity {
    match parity {
        SerialParity::None => Parity::None,
        SerialParity::Even => Parity::Even,
        SerialParity::Odd => Parity::Odd,
    }
}

fn map_data_bits(bits: u8) -> Result<DataBits, DriverError> {
    match bits {
        5 => Ok(DataBits::Five),
        6 => Ok(DataBits::Six),
        7 => Ok(DataBits::Seven),
        8 => Ok(DataBits::Eight),
        other => Err(DriverError::Comm {
            message: format!("unsupported dataBits: {other}"),
        }),
    }
}

fn map_stop_bits(bits: u8) -> Result<StopBits, DriverError> {
    match bits {
        1 => Ok(StopBits::One),
        2 => Ok(StopBits::Two),
        other => Err(DriverError::Comm {
            message: format!("unsupported stopBits: {other}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;

    #[tokio::test]
    async fn it_can_read_holding_registers_over_rtu_when_enabled() {
        if env::var("COMM_IT_ENABLE").ok().as_deref() != Some("1") {
            return;
        }

        let port = match env::var("COMM_IT_RTU_PORT") {
            Ok(v) => v,
            Err(_) => return,
        };
        let baud: u32 = match env::var("COMM_IT_RTU_BAUD")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            Some(v) => v,
            None => return,
        };
        let parity = match env::var("COMM_IT_RTU_PARITY").ok().as_deref() {
            Some("None") => SerialParity::None,
            Some("Even") => SerialParity::Even,
            Some("Odd") => SerialParity::Odd,
            _ => return,
        };
        let data_bits: u8 = match env::var("COMM_IT_RTU_DATABITS")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            Some(v) => v,
            None => return,
        };
        let stop_bits: u8 = match env::var("COMM_IT_RTU_STOPBITS")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            Some(v) => v,
            None => return,
        };
        let slave_id: u8 = match env::var("COMM_IT_RTU_SLAVEID")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            Some(v) => v,
            None => return,
        };

        let driver = ModbusRtuDriver::new();
        let profile = ConnectionProfile::Rtu485 {
            channel_name: "it-rtu".to_string(),
            device_id: slave_id,
            read_area: RegisterArea::Holding,
            start_address: 0,
            length: 10,
            serial_port: port,
            baud_rate: baud,
            parity,
            data_bits,
            stop_bits,
            timeout_ms: 1000,
            retry_count: 0,
            poll_interval_ms: 500,
        };
        let job = ReadJob {
            channel_name: "it-rtu".to_string(),
            read_area: RegisterArea::Holding,
            start_address: 0,
            length: 2,
            points: vec![],
        };

        let raw = driver.read(&profile, &job).await;
        assert!(matches!(raw, Ok(RawReadData::Registers(_))));
    }
}
