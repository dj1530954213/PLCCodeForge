mod protocol;
mod serializer;
mod factory;
mod parser;
mod config;
mod backend;

// 导出解析器入口（仅保留必要的公共 API）。
pub use parser::{read_pou, read_pou_with_config, DEFAULT_SERIALIZE_VERSION};

// 对外导出：版本标识 / 配置 / 编解码器
pub use protocol::PlcVariant;
pub use config::HollysysConfig;
pub use backend::HollysysCodec;
