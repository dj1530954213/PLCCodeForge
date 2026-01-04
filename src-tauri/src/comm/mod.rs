#![allow(dead_code)]

pub mod error;
pub mod tauri_api;

pub mod adapters;
pub mod core;
pub mod usecase;

// --- Back-compat surface (internal callers/tests rely on these paths) ---
// Keep `crate::comm::model`, `crate::comm::plan`, `crate::comm::driver`, etc. stable.
pub use adapters::driver;
pub use adapters::storage::path_resolver;
pub use adapters::storage::storage;
pub use adapters::union_xlsx_parser;
pub use core::codec;
pub use core::model;
pub use core::plan;
pub use core::union_spec_v1;
pub use usecase::bridge::bridge_importresult_stub;
pub use usecase::bridge::bridge_plc_import;
pub use usecase::engine;
pub use usecase::export::export_delivery_xlsx;
pub use usecase::export::export_ir;
pub use usecase::export::export_plc_import_stub;
pub use usecase::export::export_xlsx;
pub use usecase::import_union_xlsx;
pub use usecase::merge_unified_import;
