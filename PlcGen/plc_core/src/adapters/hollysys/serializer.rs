/*
混合序列化引擎
*/
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Result, bail};
use byteorder::{LittleEndian, WriteBytesExt};
use encoding_rs::GBK;
use log::debug;
use crate::adapters::hollysys::protocol::PlcVariant;
use crate::ast::{ElementType, LdElement, Network, UniversalPou};

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
        self.write_mfc_string(class_name)?;//写入实际的类签名
        Ok(())
    }

    /// 动态对齐到 4 字节边界 (Alignment Padding)
    /// 依据：HEX 数据显示，在写入 POU Name 后，总是会补 0 直到偏移量能被 4 整除。
    /// 例如：Name长度 9，Offset 9 -> 需要补 3 个字节 -> Offset 12。
    pub fn align_to_4bytes(&mut self)->Result<()>{
        let remainder = self.offset % 4;
        if !remainder == 0{
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
    //版本标识
    variant:PlcVariant
}

impl PouSerializer{
    pub fn new(variant:PlcVariant)->Self{
        Self{variant}
    }

    /// 主入口：将内存中的 UniversalPou 转换为二进制 Vec<u8>
    pub fn serialize(&mut self, pou: &UniversalPou) -> Result<Vec<u8>> {
        let mut writer = MfcWriter::new(Vec::new());

        debug!("Serializing POU: {}, Variant: {:?}", pou.name, self.variant);

        // 阶段 1: 写入头部 (Header) - 包含最复杂的版本差异逻辑
        self.write_header(&mut writer, pou)?;

        // 阶段 2: 写入逻辑网络 (Networks) - MFC CObList 结构
        self.write_networks(&mut writer, pou)?;

        // 阶段 3: 写入变量表 (Variables) - 自定义 Tag 结构
        self.write_variables(&mut writer, pou)?;

        // 阶段 4: 写入尾部填充 (Footer) - 安全气囊，防止读取越界
        writer.write_bytes(&[0u8; 64])?;

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
        if self.variant==PlcVariant::Normal{
            let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as u32;
            w.write_u32(ts)?;
        }

        // [3] 第二次写入 POU 名称
        // 依据：IDA 代码显示 CPOU 对象内部有两个 String 成员存储名称
        w.write_mfc_string(&pou.name)?;
        w.align_to_4bytes()?;

        // [4] Metadata Flags 区域 (真空区 - 关键差异)
        // 依据：对比 Name2 结束到 LanguageID(1) 开始之间的字节数
        match self.variant {
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

                // [Int 2] Safety Flag = 0x00010000 (Little Endian: 00 01 00 00)
                // 依据：HEX 中 Line 2 结尾跨 Line 3 开头确实存在 00 01 00 00
                // 且 IDA 代码显示显式写入了 offset + 25 位置的字节
                w.write_u32(256)?;

                // [Int 3] 0
                w.write_u32(0)?;
                // [Int 4] 0
                w.write_u32(0)?;
                // [Int 5] 0 (此前漏算的第5个Int)
                w.write_u32(0)?;
            }
        }

        // [5] 通用尾部 (Normal/Safety 一致)
        // 依据：所有样本此处完全相同
        w.write_u32(1)?;             // Language ID (1=LD),编程语言标识
        w.write_u32(1)?;             // Reserved
        w.write_mfc_string("BOOL")?; // Return Type
        w.write_u32(1)?;             // Flag
        w.write_u32(0)?;             // Flag
        
        Ok(())
    }

    // =========================================================
    // 核心逻辑：Network 列表写入 (MFC CObList)
    // =========================================================
    fn write_networks(&self, w: &mut MfcWriter<Vec<u8>>, pou: &UniversalPou) -> Result<()> {
        // [1] List Block Hint (预分配大小)
        // 依据：HEX 显示 06 00 (Normal) 或 04 00 (Safety)，写 6 兼容两者
        w.write_u16(6)?;

        // [2] 列表类签名
        // 依据：HEX 中出现的 FFFF ... CLDNetwork
        w.write_class_sig("CLDNetwork")?;

        // [3] 循环写入每个梯级
        for network in &pou.networks {
            self.write_network(w, network)?;
        }
        Ok(())
    }

    fn write_network(&self, w: &mut MfcWriter<Vec<u8>>, net: &Network) -> Result<()> {
        // [IDA] CLDNetwork::Serialize
        w.write_i32(net.id)?;           // Element ID
        w.write_u8(0x09)?;              // Type Code (09 = Network)
        w.write_i32(1)?;                // Flag (Expanded)
        w.write_i32(net.id + 1)?;       // RungID
        w.write_mfc_string(&net.label)?;   // 标号
        w.write_mfc_string(&net.comment)?; // 注释

        // [HEX] 拓扑列表头
        w.write_u16(0x8001)?; // Magic Number (01 80)
        // 子元素数量
        w.write_u16(net.elements.len() as u16)?;

        // 递归写入子元素
        for elem in &net.elements {
            self.write_element(w, elem)?;
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
            ElementType::Coil => "CLDCoil",
            _ => "CLDElement",
        };
        // 写入类签名
        w.write_class_sig(class_name)?;

        // [2] 写入基类共有数据
        w.write_i32(elem.id)?;
        w.write_u8(elem.type_code as u8)?;
        // 注意：Box 的 name 是指令名 (如 "MOV_CTRL")，Contact 是变量名
        w.write_mfc_string(&elem.name)?;

        // [3] 特化逻辑：功能块 (CLDBox)
        if elem.type_code == ElementType::Box {
            w.write_i32(0)?; // Padding (HEX: 00 00 00 00)

            // [关键发现] 功能块有两种存储模式
            if !elem.instance.is_empty() {
                // --- 模式 A: 有实例 (如 MOV_CTRL, TON) ---
                // 依据：Safety HEX 中 MOV_CTRL 结构
                w.write_u8(1)?; // Flag: Has Instance = True (HEX: 01)
                w.write_mfc_string(&elem.instance)?; // 写入实例名 (e.g. "MOV_CTRL_1001")

                // 紧凑引脚格式 (Compact Pins)
                w.write_u16(elem.pins.len() as u16)?; // Pin Count
                for pin in &elem.pins {
                    // 仅写入 Name 和 Variable，没有 Type 和 Flag
                    w.write_mfc_string(&pin.name)?;     // e.g. "EN"
                    w.write_mfc_string(&pin.variable)?; // e.g. "Tag1"
                }
            } else {
                // --- 模式 B: 无实例 (如 MOVE, ADD) ---
                // 依据：Normal HEX 中 MOVE 结构
                // 这里没有 01 Flag，也没有实例名

                // 标准引脚格式 (Standard Pins)
                w.write_u16(elem.pins.len() as u16)?; // Pin Count
                for pin in &elem.pins {
                    // 包含完整的 Type 和 Flag
                    w.write_i32(2)?; // Pin Type (固定 2, HEX: 02 00 00 00)
                    w.write_mfc_string(&pin.name)?;     // e.g. "EN"
                    w.write_u8(0)?;  // Pin Flag
                    w.write_mfc_string(&pin.variable)?; // e.g. "Tag1"
                }
            }
        }
        // [4] 特化逻辑：触点/线圈
        else if elem.type_code == ElementType::Contact || elem.type_code == ElementType::Coil {
            // 依据：HEX 显示只有 1 字节 SubType + 4 字节 FF
            w.write_u8(elem.sub_type)?; // 0=NO, 1=NC
            w.write_i32(-1)?; // Padding/Address (FFFFFFFF)
        }

        Ok(())
    }

    // =========================================================
    // 核心逻辑：变量表写入 (Custom Tag 0x15)
    // =========================================================
    fn write_variables(&self, w: &mut MfcWriter<Vec<u8>>, pou: &UniversalPou) -> Result<()> {
        if pou.variables.is_empty() { return Ok(()); }

        // 依据：C++ switch-case 0x15 分支
        for var in &pou.variables {
            w.write_u8(0x15)?; // Tag: Local Variable
            w.write_mfc_string(&var.name)?;
            w.write_u32(0)?;   // Padding
            w.write_mfc_string(&var.data_type)?;
            w.write_mfc_string(&var.init_value)?;
            w.write_u8(0x04)?; // Visibility (04=Public/Local)
            w.write_i32(-1)?;  // Address ID (-1=Auto)
        }
        Ok(())
    }
}