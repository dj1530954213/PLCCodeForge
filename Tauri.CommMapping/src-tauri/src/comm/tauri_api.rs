//! Tauri command 层（冻结契约入口）。
//!
//! 注意：一旦 DTO/命令在这里对外暴露，即视为稳定契约（后续只允许新增可选字段）。
//!
//! 硬约束（来自 Docs/通讯数据采集验证/执行要求.md）：
//! - `comm_run_start` 只能 spawn 后台任务；不得在 command 内循环采集
//! - `comm_run_latest` 只读缓存，不触发采集
//! - `comm_run_stop` 必须在 1s 内生效（MVP）
//! - DTO 契约冻结：只允许新增可选字段，不得改名/删字段/改语义

mod common;
mod commands;
mod services;
mod state;
mod types;

pub use commands::*;
pub use state::CommState;
pub use types::*;
