use crate::ast::UniversalPou;
use anyhow::Result;

/// PLC 后端生成器接口
/// 任何品牌的 PLC 实现 (和利时、西门子) 都必须实现这个 Trait
pub trait PlcBackend{
    /// 核心方法：将通用 POU 编译为特定品牌的二进制/XML 数据流
    fn compile(&self,pou:&UniversalPou)->Result<Vec<u8>>;
    /// 获取该品牌在 Windows 剪贴板中注册的格式名称
    /// e.g. "POU_TREE_Clipboard_PLC"
    fn format_name(&self)->&'static str;
}