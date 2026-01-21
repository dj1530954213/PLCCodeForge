use anyhow::Result;

use crate::ast::UniversalPou;
use crate::ports::backend::PouCodec;

use super::config::HollysysConfig;
use super::protocol::PlcVariant;
use super::serializer::PouSerializer;

/// Hollysys 编解码器：把通用 POU 与和利时二进制互转
#[derive(Debug, Clone)]
pub struct HollysysCodec {
    config: HollysysConfig,
}

impl HollysysCodec {
    /// 使用指定配置创建编解码器
    pub fn new(config: HollysysConfig) -> Self {
        Self { config }
    }

    /// 快捷构建：Normal 版本
    pub fn normal() -> Self {
        Self::new(HollysysConfig::normal())
    }

    /// 快捷构建：Safety 版本
    pub fn safety() -> Self {
        Self::new(HollysysConfig::safety())
    }

    /// 只读访问配置（便于上层做诊断）
    pub fn config(&self) -> &HollysysConfig {
        &self.config
    }
}

impl PouCodec for HollysysCodec {
    /// 解码入口：从剪贴板二进制流解析为 POU
    fn decode(&self, data: &[u8]) -> Result<UniversalPou> {
        super::parser::read_pou_with_config(
            data,
            self.config.variant,
            self.config.serialize_version,
        )
    }

    /// 编码入口：生成剪贴板二进制流
    fn encode(&self, pou: &UniversalPou) -> Result<Vec<u8>> {
        let mut serializer = PouSerializer::from_config(self.config.clone());
        serializer.serialize(pou)
    }

    /// 剪贴板格式名称（根据版本分流）
    fn format_name(&self) -> &'static str {
        match self.config.variant {
            PlcVariant::Normal => "POU_TREE_Clipboard_PLC",
            PlcVariant::Safety => "POU_TREE_Clipboard_ITCC",
        }
    }
}
