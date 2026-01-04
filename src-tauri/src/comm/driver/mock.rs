//! Mock 通讯驱动（用于无真实 PLC 环境的 demo/单测）。
//!
//! 行为约定（便于在前端快速构造不同质量结果）：
//! - `channelName` 包含 `timeout` → 返回 `DriverError::Timeout`
//! - `channelName` 包含 `decode` → 返回“短数据”（长度比请求少 1），用于触发上层 `DecodeError`
//! - 其他 → 返回确定性的假数据（按 startAddress 递增）

use super::{CommDriver, DriverError, DriverFuture, RawReadData};
use crate::comm::model::{ConnectionProfile, RegisterArea};
use crate::comm::plan::ReadJob;

#[derive(Clone, Debug, Default)]
pub struct MockDriver;

impl MockDriver {
    pub fn new() -> Self {
        Self
    }
}

impl CommDriver for MockDriver {
    fn read<'a>(&'a self, _profile: &'a ConnectionProfile, job: &'a ReadJob) -> DriverFuture<'a> {
        Box::pin(async move {
            let channel_name = job.channel_name.as_str();
            if channel_name.contains("timeout") {
                return Err(DriverError::Timeout);
            }

            let requested = job.length as usize;
            let actual = if channel_name.contains("decode") {
                requested.saturating_sub(1)
            } else {
                requested
            };

            match job.read_area {
                RegisterArea::Coil | RegisterArea::Discrete => {
                    let bits: Vec<bool> = (0..actual).map(|i| ((job.start_address as usize + i) % 2) == 1).collect();
                    Ok(RawReadData::Coils(bits))
                }
                RegisterArea::Holding | RegisterArea::Input => {
                    let registers: Vec<u16> = (0..actual)
                        .map(|i| job.start_address.wrapping_add(i as u16))
                        .collect();
                    Ok(RawReadData::Registers(registers))
                }
            }
        })
    }
}

