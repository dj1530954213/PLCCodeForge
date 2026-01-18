use serde::{Deserialize, Serialize};


/// 顶层 POU (Program Organization Unit) 结构
/// 这是我们与前端 Tauri 交互的核心数据对象
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct UniversalPou{
    ///POU名称
    pub name:String,
    /// Safety 头部中的 CStringArray 列表（引用/依赖项）
    #[serde(default)]
    pub header_strings:Vec<String>,
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
    /// 变量ID (var_id, u16)：当前算法未知，若不提供则由序列化器生成占位值
    #[serde(default)]
    pub var_id:Option<u16>,
    /// 地址/资源ID (addr_id, u64)：Normal 版本 tail 的 8 字节字段
    /// None 表示未绑定，序列化器会填 0xFFFFFFFFFFFFFFFF
    #[serde(default)]
    pub addr_id:Option<u64>,
    /// Normal 版本 tail 的 mode(u8)，样本中常见 0x06/0x16
    /// None 时由序列化器按 addr_id 是否为 FF 自动推断
    #[serde(default)]
    pub mode:Option<u8>,
    /// Normal 版本 tail 的 id2(u32)，用途未明
    #[serde(default)]
    pub id2:Option<u32>,
    /// Safety 版本 tail 的 area_code(u8)，默认 0x04
    #[serde(default)]
    pub area_code:Option<u8>,
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Network{
    pub id:i32,
    pub label:String,//梯级标号
    pub comment:String,//梯级注释
    ///梯级内的元素列表
    pub elements:Vec<LdElement>,
    /// Safety 拓扑 Token 流（0x80xx 标记流）
    /// - Safety 多元素网络必须提供该流，否则序列化器会拒绝写入
    /// - 单元素网络可为空（样本中不出现 Token 流）
    /// - 该流是“原子写入”的序列，可按 u16/u32 组合精确控制字节布局
    #[serde(default)]
    pub safety_topology:Vec<SafetyTopologyToken>,
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
    pub variable:String,
    /// 引脚方向：输入或输出
    /// 默认输入，兼容旧数据（未提供方向时不会反序列化失败）
    #[serde(default)]
    pub direction: PinDirection,
}

/// 功能块引脚方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinDirection {
    Input,
    Output,
}

impl Default for PinDirection {
    fn default() -> Self {
        PinDirection::Input
    }
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct LdElement{
    pub id:i32,
    pub type_code:ElementType,
    /// 名称字段：
    /// - 对于 Box: 它是指令名 (如 "MOVE")
    /// - 对于 Contact/Coil: 它是绑定的变量名 (如 "Motor_Start")
    pub name:String,
    /// 元素注释（映射到 CLDElement.comment）
    #[serde(default)]
    pub comment:String,
    /// 元素描述（映射到 CLDElement.desc）
    #[serde(default)]
    pub desc:String,
    /// 实例名 (关键差异点)
    /// - 普通指令 (MOVE): 此字段为空
    /// - 实例指令 (TON, MOV_CTRL): 此字段存放实例名 (如 "Timer1")
    #[serde(default)]
    pub instance:String,
    
    #[serde(default)]
    pub pins:Vec<BoxPin>,

    /// 连接 ID 列表（Normal 拓扑使用图连接表）
    /// - Normal: 由外部拓扑构建器生成（元素之间的有向连接）
    /// - Safety: 通常为空，因 Safety 使用 0x80xx Token 流描述拓扑
    #[serde(default)]
    pub connections:Vec<i32>,

    /// [Contact/Coil 专用] 子类型
    /// 0 = 常开/普通线圈
    /// 1 = 常闭/取反线圈
    #[serde(default)]
    pub sub_type: u8,
}

/// Safety 拓扑 Token（递归 Token 流）
/// 说明：
/// - Marker 使用固定语义，元素以 Token 形式出现
/// - Element Token 保存完整元件，便于构建树状拓扑
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", content = "value")]
pub enum SafetyTopologyToken {
    BranchOpen,
    BranchClose,
    SeriesNext,
    NetEnd,
    BranchNext,
    /// 0x800C: 内联定义的完整元件 (通常为 Box)
    InlineElement(Box<LdElement>),
    /// 引用已定义的元件 ID + Type
    ElementRef { id: u32, type_id: u16 },
    /// 兼容旧格式：直接携带元件数据
    Element(Box<LdElement>),
    /// 无法语义化的原始 Token（保留现场字节）
    Raw(u16),
}
