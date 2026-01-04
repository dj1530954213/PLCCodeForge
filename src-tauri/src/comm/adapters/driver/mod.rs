//! 通讯驱动（driver）模块。

use std::future::Future;
use std::pin::Pin;

use thiserror::Error;

use crate::comm::core::model::ConnectionProfile;
use crate::comm::core::plan::ReadJob;

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

pub trait CommDriver: Send + Sync {
    fn read<'a>(&'a self, profile: &'a ConnectionProfile, job: &'a ReadJob) -> DriverFuture<'a>;
}
