//! Core PLC POU codec crate.
//! Responsibilities: parse/serialize POU binary to/from domain AST.
//! Non-goals: clipboard/UI/automation (handled by upper layers).

pub mod domain;
pub mod ports;
pub mod adapters;
pub mod application;

pub use domain::ast;
pub use application::service::PouService;
pub use ports::backend::PouCodec;
pub use adapters::hollysys::{HollysysCodec, HollysysConfig, PlcVariant};

pub mod symbols_config;
