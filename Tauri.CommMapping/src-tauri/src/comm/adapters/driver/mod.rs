//! 通讯驱动（driver）模块。

use std::future::Future;
use std::pin::Pin;

use thiserror::Error;

use crate::comm::core::model::ConnectionProfile;
use crate::comm::core::plan::ReadJob;

pub mod connection_manager;
pub mod mock;
pub mod modbus_rtu;
pub mod modbus_tcp;

#[derive(Clone, Debug, PartialEq)]
pub enum RawReadData {
    Coils(Vec<bool>),
    Registers(Vec<u16>),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DriverError {
    #[error("timeout")]
    Timeout,

    #[error("comm error: {message}")]
    Comm { message: String },
}

pub type DriverFuture<'a> =
    Pin<Box<dyn Future<Output = Result<RawReadData, DriverError>> + Send + 'a>>;

pub type ConnectFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ConnectedClient, DriverError>> + Send + 'a>>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConnectionKey {
    Tcp {
        ip: String,
        port: u16,
        unit_id: u8,
    },
    Rtu485 {
        serial_port: String,
        baud_rate: u32,
        parity: String,
        data_bits: u8,
        stop_bits: u8,
        slave_id: u8,
    },
    Mock {
        channel_name: String,
    },
}

impl std::fmt::Display for ConnectionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionKey::Tcp { ip, port, unit_id } => {
                write!(f, "tcp://{ip}:{port}?unitId={unit_id}")
            }
            ConnectionKey::Rtu485 {
                serial_port,
                baud_rate,
                parity,
                data_bits,
                stop_bits,
                slave_id,
            } => write!(
                f,
                "rtu://{serial_port}?baud={baud_rate}&parity={parity}&dataBits={data_bits}&stopBits={stop_bits}&slaveId={slave_id}"
            ),
            ConnectionKey::Mock { channel_name } => write!(f, "mock://{channel_name}"),
        }
    }
}

/// A connected Modbus client that can be reused across multiple reads.
pub type ConnectedClient = tokio_modbus::client::Context;

pub trait CommDriver: Send + Sync {
    fn connection_key(&self, profile: &ConnectionProfile) -> Result<ConnectionKey, DriverError>;

    fn connect<'a>(&'a self, profile: &'a ConnectionProfile) -> ConnectFuture<'a>;

    fn read_with_client<'a>(
        &'a self,
        client: &'a mut ConnectedClient,
        job: &'a ReadJob,
    ) -> DriverFuture<'a>;
}
