use super::protocol::PlcVariant;

/// Hollysys 序列化配置
/// 说明：该结构用于集中管理“规则参数”，避免散落在序列化逻辑中。
#[derive(Debug, Clone)]
pub struct HollysysConfig {
    /// 版本标识（Normal/Safety）
    pub variant: PlcVariant,
    /// POU 片段固定长度（样本固定为 0x2000）
    pub pou_total_len: usize,
    /// 序列化版本号（用于控制可选字段）
    /// - Normal: 影响 CLDBox/CLDOutput 的可选字段
    /// - Safety: 当前仅保留占位，便于未来扩展
    pub serialize_version: u32,
}

impl HollysysConfig {
    /// 默认 Normal 配置
    pub fn normal() -> Self {
        Self {
            variant: PlcVariant::Normal,
            pou_total_len: 0x2000,
            serialize_version: 6,
        }
    }

    /// 默认 Safety 配置
    pub fn safety() -> Self {
        Self {
            variant: PlcVariant::Safety,
            pou_total_len: 0x2000,
            serialize_version: 6,
        }
    }

    /// 根据 variant 生成默认配置
    pub fn new(variant: PlcVariant) -> Self {
        match variant {
            PlcVariant::Normal => Self::normal(),
            PlcVariant::Safety => Self::safety(),
        }
    }
}
