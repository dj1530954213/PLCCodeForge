pub mod backend;

// 统一导出端口 trait，便于上层依赖注入
pub use backend::PouCodec;
