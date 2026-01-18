#![allow(dead_code)]
/*
混合序列化引擎
*/
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Result, bail};
use byteorder::{LittleEndian, WriteBytesExt};
use encoding_rs::GBK;
use log::debug;
use crate::adapters::hollysys::protocol::PlcVariant;
use super::config::HollysysConfig;
use crate::ast::{ElementType, LdElement, Network, PinDirection, SafetyTopologyToken, UniversalPou, Variable};

/// 辅助类：处理 MFC 特有的二进制写入规则
struct MfcWriter<W:Write>{
    inner:W,
    pub offset:usize,
}

impl<W:Write> MfcWriter<W>{
    pub fn new(inner:W)->Self{
        Self{inner,offset:0}
    }

    ///获取底层的Writer(即最终要操作的那个Vec<u8>)
    pub fn into_inner(self)->W{
        self.inner
    }

    pub fn write_u8(&mut self,v:u8)->Result<()>{
        self.inner.write_u8(v)?;
        self.offset += 1;
        Ok(())
    }

    pub fn write_u16(&mut self,v:u16)->Result<()>{
        self.inner.write_u16::<LittleEndian>(v)?;
        self.offset += 2;
        Ok(())
    }

    pub fn write_u32(&mut self,v:u32)->Result<()>{
        self.inner.write_u32::<LittleEndian>(v)?;
        self.offset += 4;
        Ok(())
    }

    pub fn write_u64(&mut self,v:u64)->Result<()>{
        self.inner.write_u64::<LittleEndian>(v)?;
        self.offset += 8;
        Ok(())
    }

    pub fn write_i32(&mut self,v:i32)->Result<()>{
        self.inner.write_i32::<LittleEndian>(v)?;
        self.offset += 4;
        Ok(())
    }

    pub fn write_bytes(&mut self,v:&[u8])->Result<()>{
        self.inner.write_all(v)?;
        self.offset += v.len();
        Ok(())
    }

    /// 写入 MFC 字符串 (Length + Bytes)
    /// 关键点：必须转为 GBK，否则 PLC 显示乱码
    pub fn write_mfc_string(&mut self,v:&str)->Result<()>{
        let (encode_cow,_,error) = GBK.encode(v);
        if error{
            bail!("将字符串转换为GBK格式是发生错误");
        }
        let len = encode_cow.len();
        //判断是否需要写入长度前缀
        if encode_cow.len()<255{
            self.write_u8(len as u8)?;
        }else{
            self.write_u8(0xff)?;
            self.write_u16(len as u16)?;
        }

        //写入实际的内容
        self.write_bytes(&encode_cow)?;
        Ok(())
    }
    /// 写入 MFC 类签名 (Class Signature)
    /// 结构：[Magic(FFFF)] + [Schema(u16)] + [ClassName(String)]
    /// 作用：告诉反序列化器接下来是一个什么类型的对象 (如 "CLDNetwork")
    pub fn write_class_sig(&mut self,class_name:&str)->Result<()>{
        self.write_u16(super::protocol::MFC_PREFIX)?;//FFFF
        self.write_u16(0)?;          // Schema Version (通常为0)
        // 注意：类签名的长度前缀是 u16，而不是 MFC CString (u8/0xFF+u16)
        let (encode_cow,_,error) = GBK.encode(class_name);
        if error{
            bail!("将类签名转换为GBK格式时发生错误");
        }
        let name_len = encode_cow.len();
        if name_len > u16::MAX as usize{
            bail!("类签名长度超出u16范围: {}", name_len);
        }
        self.write_u16(name_len as u16)?;
        self.write_bytes(&encode_cow)?;
        Ok(())
    }

    /// 动态对齐到 4 字节边界 (Alignment Padding)
    /// 依据：HEX 数据显示，在写入 POU Name 后，总是会补 0 直到偏移量能被 4 整除。
    /// 例如：Name长度 9，Offset 9 -> 需要补 3 个字节 -> Offset 12。
    pub fn align_to_4bytes(&mut self)->Result<()>{
        let remainder = self.offset % 4;
        if remainder != 0{
            let padding = 4 - remainder;
            for _ in 0..padding{
                self.write_u8(0)?;
            }
        }
        
        Ok(())
    }
    
}

///POU序列化器
pub struct PouSerializer{
    /// Hollysys 序列化配置（规则/版本/长度）
    config:HollysysConfig
}

impl PouSerializer{
    /// 基于 Variant 的快捷构建（使用默认配置）
    pub fn new(variant:PlcVariant)->Self{
        Self::from_config(HollysysConfig::new(variant))
    }

    /// 基于完整配置构建（推荐）
    pub fn from_config(config:HollysysConfig)->Self{
        Self{config}
    }

    /// 只读访问配置（便于上层诊断/调试）
    pub fn config(&self)->&HollysysConfig{
        &self.config
    }

    /// 主入口：将内存中的 UniversalPou 转换为二进制 Vec<u8>
    pub fn serialize(&mut self, pou: &UniversalPou) -> Result<Vec<u8>> {
        let mut writer = MfcWriter::new(Vec::new());

        debug!("Serializing POU: {}, Variant: {:?}", pou.name, self.config.variant);

        // 阶段 1: 写入头部 (Header) - 包含最复杂的版本差异逻辑
        self.write_header(&mut writer, pou)?;

        if self.config.variant == PlcVariant::Safety {
            // Safety: Header -> StringArray -> Variables -> Networks
            self.write_header_string_array(&mut writer, pou)?;
            self.write_variables(&mut writer, pou)?;
            self.write_networks(&mut writer, pou)?;
        } else {
            // Normal: Header -> Networks -> Variables
            self.write_networks(&mut writer, pou)?;
            self.write_variables(&mut writer, pou)?;
        }

        // 阶段 4: 写入尾部填充 (Footer)
        // 依据样本：POU 片段固定长度为 0x2000，不足部分用 0 填充。
        let total_len = self.config.pou_total_len;
        if writer.offset > total_len {
            bail!("POU序列化长度超出固定上限: {}", writer.offset);
        }
        let padding = total_len - writer.offset;
        if padding > 0 {
            writer.write_bytes(&vec![0u8; padding])?;
        }

        Ok(writer.into_inner())
    }
    
    fn write_header(&self,w:&mut MfcWriter<Vec<u8>>,pou:&UniversalPou)->Result<()>{
        // [1] 第一次写入 POU 名称
        // 依据：HEX 开头总是 POU 名称
        w.write_mfc_string(&pou.name)?;
        // [1.1] 对齐 Padding
        // 依据：HEX 中 Name 后紧跟 00 00 00，补齐到 4 字节边界
        w.align_to_4bytes()?;

        // [2] 写入时间戳 (版本分歧点)
        // 依据：Normal HEX 有 4 字节时间戳；Safety HEX 直接跳过此字段
        if self.config.variant==PlcVariant::Normal{
            let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as u32;
            w.write_u32(ts)?;
        } else {
            // Safety 版在第二次名称前存在 2 字节保留字段
            w.write_u16(0)?;
        }

        // [3] 第二次写入 POU 名称
        // 依据：IDA 代码显示 CPOU 对象内部有两个 String 成员存储名称
        w.write_mfc_string(&pou.name)?;
        w.align_to_4bytes()?;

        // [4] Metadata Flags 区域 (真空区 - 关键差异)
        // 依据：对比 Name2 结束到 LanguageID(1) 开始之间的字节数
        match self.config.variant {
            PlcVariant::Normal => {
                // 普通型：12 字节全 0 (3个 Int)
                // 对应 IDA: 成员变量初始化为 0
                w.write_u32(0)?;
                w.write_u32(0)?;
                w.write_u32(0)?;
            }
            PlcVariant::Safety => {
                // 安全型：20 字节 (5个 Int)
                // [Int 1] 0
                w.write_u32(0)?;

                // [Int 2] Safety Flag = 0x00010000
                // 依据：样本标注为 0x00010000（小端应为 00 00 01 00）
                w.write_u32(0x0001_0000)?;

                // [Int 3] 0
                w.write_u32(0)?;
                // [Int 4] 0
                w.write_u32(0)?;
                // [Int 5] 0 (此前漏算的第5个Int)
                w.write_u32(0)?;
            }
        }

        // [5] 语言与返回类型 (Normal/Safety 共享起点，但尾部字段不同)
        // 依据：样本显示这里从 LanguageID 开始进入固定字段序列
        w.write_u32(1)?;             // Language ID (1=LD)
        w.write_mfc_string("BOOL")?; // 返回类型
        w.write_u32(1)?;             // Flag1
        w.write_u32(0x00000100)?;    // Flag2 (样本固定为 0x00000100)
        match self.config.variant {
            PlcVariant::Normal => {
                // Normal 版会再跟一个 CString "BOOL"
                w.write_mfc_string("")?;
                w.write_mfc_string("BOOL")?;
            }
            PlcVariant::Safety => {
                // Safety 版在此处额外跟一个 u32=0
                w.write_u32(0)?;
            }
        }
        
        Ok(())
    }

    // =========================================================
    // 核心逻辑：Network 列表写入 (MFC CObList)
    // =========================================================
    fn write_networks(&self, w: &mut MfcWriter<Vec<u8>>, pou: &UniversalPou) -> Result<()> {
        // [1] CObList 头：写入对象总数 (u16)
        // 依据：样本中列表头部就是对象数量 (例如 03 00)
        let element_count: usize = pou.networks.iter().map(|n| {
            if self.config.variant == PlcVariant::Safety {
                let inline_ids = inline_element_ids(&n.safety_topology);
                n.elements.iter().filter(|e| !inline_ids.contains(&e.id)).count()
            } else {
                n.elements.len()
            }
        }).sum();
        let total = pou.networks.len() + element_count;
        if total > u16::MAX as usize {
            bail!("CObList对象数量超出u16上限: {}", total);
        }
        w.write_u16(total as u16)?;

        // [2] 列表内容：按网络顺序扁平化写入
        // 关键点：CLDNetwork 只写自身头部，元素作为独立对象紧随其后。
        for network in &pou.networks {
            self.validate_network_topology(network)?;
            w.write_class_sig("CLDNetwork")?;
            self.write_network(w, network)?;
            if self.config.variant == PlcVariant::Safety && !network.safety_topology.is_empty() {
                self.write_safety_topology(w, network)?;
            }

            let inline_ids = if self.config.variant == PlcVariant::Safety {
                inline_element_ids(&network.safety_topology)
            } else {
                HashSet::new()
            };
            for elem in &network.elements {
                if inline_ids.contains(&elem.id) {
                    continue;
                }
                self.write_element(w, elem)?;
            }
        }
        Ok(())
    }

    fn write_network(&self, w: &mut MfcWriter<Vec<u8>>, net: &Network) -> Result<()> {
        // [IDA] CLDNetwork::Serialize
        // 注意：Normal 与 Safety 的字段宽度不同，需要分支处理。
        let id = checked_u32(net.id, "network.id")?;
        let rung_id_raw = net.id.checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("network.rung_id 发生溢出: {}", net.id))?;
        let rung_id = checked_u32(rung_id_raw, "network.rung_id")?;

        w.write_u32(id)?;
        match self.config.variant {
            PlcVariant::Normal => {
                w.write_u16(0x000A)?; // type = 0x000A
                w.write_u16(0x0001)?; // flag = 1
                w.write_u16(checked_u16(rung_id, "network.rung_id")?)?;
                w.write_u16(0)?; // pad
            }
            PlcVariant::Safety => {
                w.write_u16(0x0009)?; // type = 0x0009
                w.write_u32(1)?;      // flag = 1
                w.write_u32(rung_id)?; // rung_id
            }
        }

        w.write_mfc_string(&net.label)?;   // 标号
        w.write_mfc_string(&net.comment)?; // 注释
        Ok(())
    }

    /// 校验网络拓扑信息是否满足当前版本要求。
    fn validate_network_topology(&self, net: &Network) -> Result<()> {
        match self.config.variant {
            PlcVariant::Safety => {
                if net.safety_topology.is_empty() {
                    if net.elements.len() > 1 {
                        bail!("Safety 多元素网络缺少拓扑流: network.id={}", net.id);
                    }
                    return Ok(());
                }
            }
            PlcVariant::Normal => {
                if net.elements.len() <= 1 {
                    return Ok(());
                }
                // Normal 版必须使用连接图；若所有元素都没有连接，说明拓扑尚未生成
                let has_any_conn = net.elements.iter().any(|elem| !elem.connections.is_empty());
                if !has_any_conn {
                    bail!("Normal 多元素网络缺少连接图: network.id={}", net.id);
                }
            }
        }
        Ok(())
    }

    /// 写入 Safety 拓扑 Token 流（0x80xx 标记）
    /// 说明：Token 流是“原样输出”，由上层拓扑构建器保证正确性。
    fn write_safety_topology(&self, w: &mut MfcWriter<Vec<u8>>, net: &Network) -> Result<()> {
        if self.config.variant != PlcVariant::Safety || net.safety_topology.is_empty() {
            return Ok(());
        }
        for token in &net.safety_topology {
            match token {
                SafetyTopologyToken::BranchOpen => w.write_u16(0x8001)?,
                SafetyTopologyToken::BranchClose => w.write_u16(0x8003)?,
                SafetyTopologyToken::SeriesNext => w.write_u16(0x8005)?,
                SafetyTopologyToken::NetEnd => w.write_u16(0x8007)?,
                SafetyTopologyToken::BranchNext => w.write_u16(0x8009)?,
                SafetyTopologyToken::InlineElement(elem) => {
                    w.write_u16(0x800C)?;
                    self.write_inline_element(w, elem)?;
                }
                SafetyTopologyToken::ElementRef { id, type_id } => {
                    w.write_u32(*id)?;
                    w.write_u16(*type_id)?;
                    w.write_u32(0)?;
                }
                SafetyTopologyToken::Element(elem) => {
                    // 兼容旧格式：Box 优先内联，其它按引用输出
                    if elem.type_code == ElementType::Box {
                        w.write_u16(0x800C)?;
                        self.write_inline_element(w, elem)?;
                    } else {
                        let id = checked_u32(elem.id, "element.id")?;
                        let type_id = self.element_type_id(elem.type_code)? as u16;
                        w.write_u32(id)?;
                        w.write_u16(type_id)?;
                        w.write_u32(0)?;
                    }
                }
                SafetyTopologyToken::Raw(value) => {
                    w.write_u16(*value)?;
                }
            }
        }
        Ok(())
    }

    fn write_inline_element(&self, w: &mut MfcWriter<Vec<u8>>, elem: &LdElement) -> Result<()> {
        let id = checked_u32(elem.id, "element.id")?;
        let type_id = self.element_type_id(elem.type_code)?;

        w.write_u32(id)?;
        w.write_u8(type_id)?;
        w.write_mfc_string(&elem.name)?;
        w.write_mfc_string(&elem.comment)?;
        w.write_mfc_string(&elem.desc)?;
        w.write_u32(0)?; // Safety: conn_count 固定为 0

        match elem.type_code {
            ElementType::Box => {
                let has_instance = !elem.instance.is_empty();
                w.write_u8(if has_instance { 1 } else { 0 })?;
                w.write_mfc_string(&elem.instance)?;

                let (input_pins, output_pins) = split_pins(&elem.pins);
                w.write_u32(input_pins.len() as u32)?;
                for pin in input_pins {
                    self.write_pin(w, pin, PinDirection::Input)?;
                }

                w.write_u32(output_pins.len() as u32)?;
                for pin in output_pins {
                    self.write_pin(w, pin, PinDirection::Output)?;
                }
            }
            _ => bail!("0x800C 仅支持 Box 元件"),
        }
        Ok(())
    }

    // =========================================================
    // 核心逻辑：元件写入 (多态 + 实例模式区分)
    // =========================================================
    fn write_element(&self, w: &mut MfcWriter<Vec<u8>>, elem: &LdElement) -> Result<()> {
        // [1] 映射 Rust 枚举到 MFC 类名
        let class_name = match elem.type_code {
            ElementType::Box => "CLDBox",
            ElementType::Contact => "CLDContact",
            ElementType::Coil => "CLDOutput",
            _ => "CLDElement",
        };

        // [2] 写入类签名 + 基类字段
        w.write_class_sig(class_name)?;
        self.write_element_base(w, elem)?;

        // [3] 根据类型写入派生字段
        match elem.type_code {
            ElementType::Box => self.write_box(w, elem)?,
            ElementType::Contact => self.write_contact(w, elem)?,
            ElementType::Coil => self.write_output(w, elem)?,
            ElementType::Network => {
                // 理论上网络不应该走到这里
                bail!("ElementType::Network 不能走 write_element");
            }
        }

        Ok(())
    }

    /// 写入 CLDElement 基类字段。
    /// 顺序：id(u32) -> type_id(u8) -> name/comment/desc CString -> conn_count(u32) -> conns...
    fn write_element_base(&self, w: &mut MfcWriter<Vec<u8>>, elem: &LdElement) -> Result<()> {
        let id = checked_u32(elem.id, "element.id")?;
        let type_id = self.element_type_id(elem.type_code)?;

        w.write_u32(id)?;
        w.write_u8(type_id)?;
        w.write_mfc_string(&elem.name)?;
        w.write_mfc_string(&elem.comment)?;
        w.write_mfc_string(&elem.desc)?;

        // 连接列表：Normal 使用图连接表；Safety 使用 0x80xx Token 流，不依赖此处。
        // 为避免误写，Safety 始终写 0；Normal 则按 elem.connections 写入。
        if elem.connections.len() > u32::MAX as usize {
            bail!("element.connections 数量超出 u32 上限: {}", elem.connections.len());
        }
        let conn_count = if self.config.variant == PlcVariant::Normal {
            elem.connections.len() as u32
        } else {
            0
        };
        w.write_u32(conn_count)?;
        if self.config.variant == PlcVariant::Normal {
            for conn_id in &elem.connections {
                // 连接 ID 必须是非负整数
                let conn = checked_u32(*conn_id, "element.conn_id")?;
                w.write_u32(conn)?;
            }
        }
        Ok(())
    }

    /// 触点：Base + flag(u8) + (Normal 版) 额外 CString。
    fn write_contact(&self, w: &mut MfcWriter<Vec<u8>>, elem: &LdElement) -> Result<()> {
        w.write_u8(elem.sub_type)?;
        if self.config.variant == PlcVariant::Normal {
            // 该 CString 在 IDA 中存在，但具体含义未确认，暂写空字符串。
            w.write_mfc_string("")?;
        }
        Ok(())
    }

    /// 线圈（CLDOutput）：Base + flag/flag2 (+可选flag3) + CString。
    fn write_output(&self, w: &mut MfcWriter<Vec<u8>>, elem: &LdElement) -> Result<()> {
        w.write_u8(elem.sub_type)?;
        w.write_u8(0)?; // flag2，缺少样本细节时先写 0
        if self.config.variant == PlcVariant::Normal && self.config.serialize_version > 0 {
            // Normal 版在序列化版本非 0 时追加 1 字节
            w.write_u8(0)?;
        }
        // 该 CString 在 IDA 中存在，但具体含义未确认，暂写空字符串。
        w.write_mfc_string("")?;
        Ok(())
    }

    /// 功能块（CLDBox）：Base + (Normal版可选u32*2) + flag + CString + PinList。
    fn write_box(&self, w: &mut MfcWriter<Vec<u8>>, elem: &LdElement) -> Result<()> {
        if self.config.variant == PlcVariant::Normal && self.config.serialize_version >= 6 {
            // Normal 版序列化版本 >= 6 时会多出两个 u32（用途未知）
            w.write_u32(0)?;
            w.write_u32(0)?;
        }

        // flag：样本中常与“是否有实例”相关，先按实例名是否为空推断
        let has_instance = !elem.instance.is_empty();
        w.write_u8(if has_instance { 1 } else { 0 })?;

        // 该 CString 在 Safety 样本中常为实例名，Normal 中也有同位置字段
        w.write_mfc_string(&elem.instance)?;

        let (input_pins, output_pins) = split_pins(&elem.pins);

        w.write_u32(input_pins.len() as u32)?;
        for pin in input_pins {
            self.write_pin(w, pin, PinDirection::Input)?;
        }

        w.write_u32(output_pins.len() as u32)?;
        for pin in output_pins {
            self.write_pin(w, pin, PinDirection::Output)?;
        }
        Ok(())
    }

    /// 写入单个引脚：Normal/Safety 的格式不同。
    fn write_pin(&self, w: &mut MfcWriter<Vec<u8>>, pin: &crate::ast::BoxPin, direction: PinDirection) -> Result<()> {
        match self.config.variant {
            PlcVariant::Safety => {
                // Safety 版：紧凑格式，只写 name + var
                w.write_mfc_string(&pin.name)?;
                w.write_mfc_string(&pin.variable)?;
            }
            PlcVariant::Normal => {
                // Normal 版：标准格式，含 flag 与可选 addr
                w.write_u16(1)?; // flag 固定为 1（样本显示）
                w.write_mfc_string(&pin.name)?;

                // 未绑定的变量在 Normal 样本中常用 "???" 占位
                let var_name = if pin.variable.is_empty() { "???" } else { pin.variable.as_str() };
                w.write_mfc_string(var_name)?;

                if direction == PinDirection::Input {
                    // 输入引脚会带 addr，占位值一般为 0xFFFFFFFF
                    w.write_u32(0xFFFF_FFFF)?;
                }
            }
        }
        Ok(())
    }

    /// 元件类型到实际 TypeID 的映射（Normal/Safety 数值不同）。
    fn element_type_id(&self, elem_type: ElementType) -> Result<u8> {
        let type_id = match (self.config.variant, elem_type) {
            (PlcVariant::Normal, ElementType::Contact) => 0x05,
            (PlcVariant::Safety, ElementType::Contact) => 0x04,
            (PlcVariant::Normal, ElementType::Coil) => 0x06,
            (PlcVariant::Safety, ElementType::Coil) => 0x05,
            (_, ElementType::Box) => 0x03,
            // Network 不应落在这里
            (_, ElementType::Network) => {
                bail!("网络类型不应作为CLDElement序列化");
            }
        };
        Ok(type_id)
    }

    // =========================================================
    // 核心逻辑：变量表写入 (Custom Tag 0x15)
    // =========================================================
    fn write_variables(&self, w: &mut MfcWriter<Vec<u8>>, pou: &UniversalPou) -> Result<()> {
        if self.config.variant == PlcVariant::Safety {
            if pou.variables.len() > u32::MAX as usize {
                bail!("变量数量超出 u32 上限: {}", pou.variables.len());
            }
            w.write_u32(pou.variables.len() as u32)?;
            if pou.variables.is_empty() {
                return Ok(());
            }
        } else if pou.variables.is_empty() {
            return Ok(());
        }

        // 依据：C++ switch-case 0x15 分支
        // 已落地的结构来源：S09/S10/S11/S12/S13/S14 (Normal/Safety)。
        // 未确认的字段以固定模板填充，并保留 TODO 说明。
        // 变量 ID 分配策略：
        // 1) 如果上层显式提供 var_id，则直接使用；
        // 2) 否则按变量表顺序分配递增 u16（样本显示可能是顺序句柄）。
        // 注意：真实算法仍待确认，因此保留手动覆盖入口。
        let mut next_var_id: u16 = 1;
        let mut var_id_map: HashMap<String, u16> = HashMap::new();

        for var in &pou.variables {
            w.write_u8(0x15)?; // TypeID: Local Variable
            let var_id = assign_var_id(var, &mut var_id_map, &mut next_var_id)?;

            match self.config.variant {
                PlcVariant::Normal => {
                    // Normal: CBaseDB::Serialize 顺序
                    // name1 -> name2 -> comment_or_res -> type -> init_flag -> init -> tail
                    // [1] name1：变量名（样本中与变量表名称一致）
                    w.write_mfc_string(&var.name)?;
                    // [2] name2：保留字段，样本中为空
                    w.write_mfc_string("")?;
                    // [3] comment_or_res：变量注释或资源标识（S10-N 为中文注释）
                    w.write_mfc_string(&var.comment)?;
                    // [4] type：变量类型字符串
                    w.write_mfc_string(&var.data_type)?;

                    // [5] init_flag：TIME=0x07，BOOL=0x00
                    let init_flag = calc_init_flag(&var.data_type, &var.init_value);
                    w.write_u8(init_flag)?;
                    // [6] init_value：初始化字符串（FALSE/T#3S 等）
                    w.write_mfc_string(&var.init_value)?;

                    // tail: retain + addr_id + extra_str + mode + var_id + retain_mirror + id2 + SOE
                    // [7] retain：0x03=保持，0x04=不保持
                    let retain_flag = if var.power_down_keep { 0x03 } else { 0x04 };
                    w.write_u8(retain_flag)?;
                    // [8] addr_id(u64)：在流中为两个连续 u32，未绑定默认 FF...FF
                    let addr_id = resolve_addr_id(var);
                    w.write_u64(addr_id)?;
                    // [9] extra_str：保留 CString，样本中为空
                    w.write_mfc_string("")?;
                    // [10] mode(u8)：S12/S13 为 0x06，S09/S10/S11 为 0x16
                    let mode = resolve_mode(var, addr_id);
                    w.write_u8(mode)?;
                    // [11] var_id(u16)：顺序句柄（Sequential）
                    w.write_u16(var_id)?;
                    // [12] retain_mirror(u8)：与掉电保持同步
                    w.write_u8(if var.power_down_keep { 1 } else { 0 })?;
                    // [13] id2(u32)：用途未知，可由上层覆盖
                    let id2 = resolve_id2(var);
                    w.write_u32(id2)?;
                    // [14] soe(u16)：SOE 使能
                    w.write_u16(if var.soe_enable { 1 } else { 0 })?;
                }
                PlcVariant::Safety => {
                    // Safety: CBaseDB::Serialize 顺序
                    // name1 -> name2 -> lang_count(u32) -> (lang, comment)* -> type -> init_flag -> init_value -> tail
                    // [1] name1：变量名
                    w.write_mfc_string(&var.name)?;
                    // [2] name2：保留字符串，Safety 样本中为空
                    w.write_mfc_string("")?;

                    // [3] 语言映射数量：0=无注释，1=包含 "CH"+comment
                    let lang_count = if var.comment.is_empty() { 0 } else { 1 };
                    w.write_u32(lang_count)?;
                    if lang_count > 0 {
                        // Safety 版：固定语言标识 "CH"，后跟中文注释
                        w.write_mfc_string("CH")?;
                        w.write_mfc_string(&var.comment)?;
                    }

                    // [4] type：变量类型字符串
                    w.write_mfc_string(&var.data_type)?;
                    // [5] init_flag：TIME=0x07，其它默认 0x00
                    let init_flag = calc_init_flag(&var.data_type, &var.init_value);
                    w.write_u8(init_flag)?;
                    // [6] init_value：初始化字符串
                    w.write_mfc_string(&var.init_value)?;

                    // tail: area_code + u16(默认FFFF) + addr_id(u32) + extra_str + mode + var_id + soe_bytes
                    // [7] area_code：区域码，默认 0x04
                    let area_code = resolve_area_code(var);
                    w.write_u8(area_code)?;
                    // [8] u16 保留/标志位：样本固定为 0xFFFF
                    w.write_u16(0xFFFF)?;
                    // [9] addr_id：Safety 仅写入 u32（未绑定默认 0xFFFFFFFF）
                    let addr_id = resolve_addr_id_u32(var);
                    w.write_u32(addr_id)?;
                    // [10] extra_str：保留 CString，样本中为空
                    w.write_mfc_string("")?;
                    // [11] mode：样本固定 0x42，可由上层覆盖
                    let mode = resolve_mode_safety(var);
                    w.write_u8(mode)?;
                    // [12] var_id：顺序句柄（Sequential）
                    w.write_u16(var_id)?;
                    // [13] soe_val：Safety 以两个字节写入（低字节/高字节）
                    let soe_val = resolve_soe_value_safety(var);
                    w.write_u8((soe_val & 0xFF) as u8)?;        // 低字节
                    w.write_u8((soe_val >> 8) as u8)?;          // 高字节
                }
            }
        }
        Ok(())
    }

    fn write_header_string_array(&self, w: &mut MfcWriter<Vec<u8>>, pou: &UniversalPou) -> Result<()> {
        let count = pou.header_strings.len();
        if count > u16::MAX as usize {
            bail!("Header 字符串数量超出 u16 上限: {}", count);
        }
        // Safety 样本中存在 4 字节保留字段 + 2 字节数量
        w.write_u32(0)?;
        if w.offset % 2 != 0 {
            w.write_u8(0)?;
        }
        w.write_u16(count as u16)?;
        for item in &pou.header_strings {
            w.write_mfc_string(item)?;
        }
        Ok(())
    }
}

/// 收集 Safety 拓扑中内联元件的 ID（用于过滤 CObList 元素列表）。
fn inline_element_ids(tokens: &[SafetyTopologyToken]) -> HashSet<i32> {
    let mut ids = HashSet::new();
    for token in tokens {
        if let SafetyTopologyToken::InlineElement(elem) = token {
            ids.insert(elem.id);
        }
    }
    ids
}

/// 将引脚按方向拆分为输入/输出两组。
fn split_pins(pins: &[crate::ast::BoxPin]) -> (Vec<&crate::ast::BoxPin>, Vec<&crate::ast::BoxPin>) {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for pin in pins {
        match pin.direction {
            PinDirection::Input => inputs.push(pin),
            PinDirection::Output => outputs.push(pin),
        }
    }
    (inputs, outputs)
}

/// i32 -> u32 的安全转换（避免负数静默溢出）。
fn checked_u32(value: i32, field: &str) -> Result<u32> {
    if value < 0 {
        bail!("{} 不能为负数: {}", field, value);
    }
    Ok(value as u32)
}

/// u32 -> u16 的安全转换（避免超过范围）。
fn checked_u16(value: u32, field: &str) -> Result<u16> {
    if value > u16::MAX as u32 {
        bail!("{} 超出u16范围: {}", field, value);
    }
    Ok(value as u16)
}

/// 变量初始值标志：TIME 使用 0x07，其它类型默认 0x00。
/// 依据：S11-N/S11-S（TIME=0x07，BOOL=0x00）。
fn calc_init_flag(data_type: &str, init_value: &str) -> u8 {
    let ty = data_type.trim().to_ascii_uppercase();
    if ty == "TIME" || init_value.trim().to_ascii_uppercase().starts_with("T#") {
        0x07
    } else {
        0x00
    }
}

/// var_id 分配：优先使用上层提供的值；否则为新变量分配递增句柄。
fn assign_var_id(
    var: &Variable,
    var_id_map: &mut HashMap<String, u16>,
    next_var_id: &mut u16,
) -> Result<u16> {
    if let Some(value) = var.var_id {
        return Ok(value);
    }
    if let Some(value) = var_id_map.get(&var.name) {
        return Ok(*value);
    }
    let value = *next_var_id;
    if value == u16::MAX {
        bail!("var_id 超出 u16 上限，变量数量过多");
    }
    *next_var_id = next_var_id.wrapping_add(1);
    var_id_map.insert(var.name.clone(), value);
    Ok(value)
}

/// addr_id 解析：Normal tail 的 8 字节字段，默认视为未绑定。
fn resolve_addr_id(var: &Variable) -> u64 {
    var.addr_id.unwrap_or(0xFFFF_FFFF_FFFF_FFFF)
}

/// Normal 版 mode 解析：当 addr_id 为 FF..FF 时默认 0x06，否则默认 0x16。
/// 若上层显式传入，则优先使用。
fn resolve_mode(var: &Variable, addr_id: u64) -> u8 {
    if let Some(value) = var.mode {
        return value;
    }
    if addr_id == 0xFFFF_FFFF_FFFF_FFFF {
        0x06
    } else {
        0x16
    }
}

/// Safety 版 addr_id 解析：仅使用低 32 位。
/// 说明：Safety 的 CBaseDB::Serialize 只写入 u32，未绑定时为 0xFFFFFFFF。
fn resolve_addr_id_u32(var: &Variable) -> u32 {
    let value = resolve_addr_id(var);
    (value & 0xFFFF_FFFF) as u32
}

/// Safety 版 mode 解析：样本中固定为 0x42，可由上层覆盖。
fn resolve_mode_safety(var: &Variable) -> u8 {
    var.mode.unwrap_or(0x42)
}

/// Safety 版 SOE 写入值：BOOL 使用 0x0100，其它类型使用 0x0001。
/// 注意：当 soe_enable=false 时必须写 0x0000。
fn resolve_soe_value_safety(var: &Variable) -> u16 {
    if !var.soe_enable {
        return 0;
    }
    let ty = var.data_type.trim().to_ascii_uppercase();
    if ty == "BOOL" {
        0x0100
    } else {
        0x0001
    }
}

/// id2 解析：Normal tail 的 u32 字段，当前意义未知。
fn resolve_id2(var: &Variable) -> u32 {
    var.id2.unwrap_or(0)
}

/// area_code 解析：Safety tail 的区域字段，默认 0x04。
fn resolve_area_code(var: &Variable) -> u8 {
    var.area_code.unwrap_or(0x04)
}
