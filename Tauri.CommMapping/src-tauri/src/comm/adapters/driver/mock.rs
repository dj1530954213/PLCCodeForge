//! Mock driver for offline/testing scenarios.

use tokio::io::duplex;
use tokio_modbus::client::{rtu, tcp};
use tokio_modbus::prelude::Slave;

use crate::comm::adapters::driver::{
    CommDriver, ConnectFuture, ConnectedClient, ConnectionKey, DriverError, DriverFuture,
    RawReadData,
};
use crate::comm::core::model::{ConnectionProfile, RegisterArea};
use crate::comm::core::plan::ReadJob;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MockMode {
    Ok,
    Timeout,
    CommError,
    DecodeError,
}

fn channel_name_from_profile(profile: &ConnectionProfile) -> &str {
    match profile {
        ConnectionProfile::Tcp { channel_name, .. } => channel_name.as_str(),
        ConnectionProfile::Rtu485 { channel_name, .. } => channel_name.as_str(),
    }
}

fn mock_mode(channel_name: &str) -> MockMode {
    let name = channel_name.to_ascii_lowercase();
    if name.contains("timeout") {
        return MockMode::Timeout;
    }
    if name.contains("comm") {
        return MockMode::CommError;
    }
    if name.contains("decode") {
        return MockMode::DecodeError;
    }
    MockMode::Ok
}

#[derive(Clone, Default)]
pub struct MockDriver;

impl MockDriver {
    pub fn new() -> Self {
        Self
    }
}

impl CommDriver for MockDriver {
    fn connection_key(&self, profile: &ConnectionProfile) -> Result<ConnectionKey, DriverError> {
        Ok(ConnectionKey::Mock {
            channel_name: channel_name_from_profile(profile).to_string(),
        })
    }

    fn connect<'a>(&'a self, profile: &'a ConnectionProfile) -> ConnectFuture<'a> {
        Box::pin(async move {
            let (stream, _peer) = duplex(64);
            let slave = match profile {
                ConnectionProfile::Tcp { device_id, .. } => Slave(*device_id),
                ConnectionProfile::Rtu485 { device_id, .. } => Slave(*device_id),
            };
            let ctx: ConnectedClient = match profile {
                ConnectionProfile::Tcp { .. } => tcp::attach_slave(stream, slave),
                ConnectionProfile::Rtu485 { .. } => rtu::attach_slave(stream, slave),
            };
            Ok(ctx)
        })
    }

    fn read_with_client<'a>(
        &'a self,
        _client: &'a mut ConnectedClient,
        job: &'a ReadJob,
    ) -> DriverFuture<'a> {
        Box::pin(async move {
            match mock_mode(&job.channel_name) {
                MockMode::Timeout => Err(DriverError::Timeout),
                MockMode::CommError => Err(DriverError::Comm {
                    message: "mock comm error".to_string(),
                }),
                MockMode::DecodeError => Ok(match job.read_area {
                    RegisterArea::Coil | RegisterArea::Discrete => {
                        RawReadData::Coils(Vec::new())
                    }
                    RegisterArea::Holding | RegisterArea::Input => {
                        RawReadData::Registers(Vec::new())
                    }
                }),
                MockMode::Ok => Ok(match job.read_area {
                    RegisterArea::Coil | RegisterArea::Discrete => RawReadData::Coils(
                        (0..job.length)
                            .map(|i| ((job.start_address as u32 + i as u32) % 2) == 0)
                            .collect(),
                    ),
                    RegisterArea::Holding | RegisterArea::Input => RawReadData::Registers(
                        (0..job.length)
                            .map(|i| job.start_address.saturating_add(i))
                            .collect(),
                    ),
                }),
            }
        })
    }
}

