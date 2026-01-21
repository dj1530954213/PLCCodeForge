use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::comm::adapters::driver::modbus_rtu::ModbusRtuDriver;
use crate::comm::adapters::driver::modbus_tcp::ModbusTcpDriver;
use crate::comm::core::model::{PointsV1, ProfilesV1};
use crate::comm::core::plan::ReadPlan;
use crate::comm::usecase::engine::CommRunEngine;

#[derive(Default, Clone)]
pub(crate) struct CommMemoryScope {
    pub(crate) profiles: Option<ProfilesV1>,
    pub(crate) points: Option<PointsV1>,
    pub(crate) plan: Option<ReadPlan>,
}

#[derive(Default)]
pub(crate) struct CommMemoryStore {
    scopes: HashMap<String, CommMemoryScope>,
}

impl CommMemoryStore {
    pub(crate) fn scope_mut(&mut self, key: &str) -> &mut CommMemoryScope {
        self.scopes.entry(key.to_string()).or_default()
    }

    pub(crate) fn scope(&self, key: &str) -> Option<&CommMemoryScope> {
        self.scopes.get(key)
    }
}

pub struct CommState {
    pub(crate) memory: Mutex<CommMemoryStore>,
    pub(crate) engine: CommRunEngine,
    pub(crate) tcp_driver: Arc<ModbusTcpDriver>,
    pub(crate) rtu_driver: Arc<ModbusRtuDriver>,
}

impl CommState {
    pub fn new() -> Self {
        Self {
            memory: Mutex::new(CommMemoryStore::default()),
            engine: CommRunEngine::new(),
            tcp_driver: Arc::new(ModbusTcpDriver::new()),
            rtu_driver: Arc::new(ModbusRtuDriver::new()),
        }
    }
}
