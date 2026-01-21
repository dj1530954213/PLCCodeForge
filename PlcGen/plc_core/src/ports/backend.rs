use crate::ast::UniversalPou;
use anyhow::Result;

/// POU 编解码端口接口
/// 说明：crate 仅负责“解析/序列化”，剪贴板交互由上层处理。
#[allow(dead_code)]
pub trait PouCodec{
    /// 解码：将二进制数据解析为通用 POU
    fn decode(&self,data:&[u8])->Result<UniversalPou>;
    /// 编码：将通用 POU 序列化为二进制数据
    fn encode(&self,pou:&UniversalPou)->Result<Vec<u8>>;
    /// 获取该品牌在 Windows 剪贴板中注册的格式名称
    /// e.g. "POU_TREE_Clipboard_PLC"
    fn format_name(&self)->&'static str;
}
