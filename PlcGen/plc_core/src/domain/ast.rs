use serde::{Deserialize, Serialize};


/// 顶层 POU (Program Organization Unit) 结构
/// 这是我们与前端 Tauri 交互的核心数据对象
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct UniversalPou{
    ///POU名称
    pub name:String,
    ///梯形图逻辑网络列表
    pub networks:Vec<Network>,
    /// 变量表 (定义 Input, Output, Local 变量)
    /// default: 如果 JSON 没传这个字段，默认为空列表
    #[serde(default)]
    pub variables:Vec<Variable>
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Variable{
    pub name:String,
    pub data_type:String,
    pub init_value:String,
    pub soe_enable:bool,//SOE使能
    pub power_down_keep:bool,//掉电保护
    #[serde(default)]
    pub comment:String,
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Network{
    pub id:i32,
    pub label:String,//梯级标号
    pub comment:String,//梯级注释
    ///梯级内的元素列表
    pub elements:Vec<LdElement>,
}

/// 元件类型枚举 (对应底层 Type Code)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementType {
    Network = 0x09, // 梯级本身也是一种 Element
    Box = 0x03,     // 功能块 (MOVE, ADD, TON)
    Contact = 0x04, // 触点 (常开/常闭)
    Coil = 0x05,    // 线圈 (输出)
}
///功能块针脚
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct BoxPin{
    /// 引脚名: "EN", "ENO", "IN1"
    pub name:String,
    /// 连接的变量名: "Tag1". 若未连接则为空
    pub variable:String
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct LdElement{
    pub id:i32,
    pub type_code:ElementType,
    /// 名称字段：
    /// - 对于 Box: 它是指令名 (如 "MOVE")
    /// - 对于 Contact/Coil: 它是绑定的变量名 (如 "Motor_Start")
    pub name:String,
    /// 实例名 (关键差异点)
    /// - 普通指令 (MOVE): 此字段为空
    /// - 实例指令 (TON, MOV_CTRL): 此字段存放实例名 (如 "Timer1")
    #[serde(default)]
    pub instance:String,
    
    #[serde(default)]
    pub pins:Vec<BoxPin>,

    /// [Contact/Coil 专用] 子类型
    /// 0 = 常开/普通线圈
    /// 1 = 常闭/取反线圈
    #[serde(default)]
    pub sub_type: u8,
}