/*
逆向出来的二进制结构
*/
/// 定义和利时的两个版本
#[derive(Debug,Clone,Eq,PartialEq,Copy)]
pub enum PlcVariant{
    Normal,
    Safety
}

/// MFC 序列化中的固定魔数
pub const MFC_PREFIX: u16 = 0xFFFF;