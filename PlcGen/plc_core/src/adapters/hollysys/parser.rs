#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use anyhow::{Result, bail};
use binrw::{binread, BinRead, BinResult, Endian};
use encoding_rs::GBK;
use log::{debug, warn};

use super::protocol::PlcVariant as Variant;
use crate::ast::{BoxPin, ElementType, LdElement, Network, PinDirection, SafetyTopologyToken, UniversalPou, Variable, VariableNode};
use crate::symbols_config::SymbolConfig;

/// MFC CString: 长度前缀 (u8 或 0xFF + u16) + 原始字节。
/// 样本中都是 1 字节字符（GBK），且字符串后没有对齐填充。
/// 如果未来遇到 Unicode/宽字符，需要在这里扩展解码逻辑。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MfcString(pub String);

impl BinRead for MfcString {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<Self> {
        // 先读长度前缀：u8；如果为 0xFF 则再读一个 u16 作为真实长度。
        let len_u8 = u8::read_options(reader, endian, ())?;
        let len = if len_u8 == 0xFF {
            u16::read_options(reader, endian, ())? as usize
        } else {
            len_u8 as usize
        };

        // 直接读取原始字节并按 GBK 解码。
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        let (cow, _, _) = GBK.decode(&buf);
        Ok(MfcString(cow.into_owned()))
    }
}

/// 引脚序列化的两种形态：
/// - Compact：仅 Name/Var（Safety 常见）。
/// - Standard：u8,u8 + Name + Var + u32（Normal 常见）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinFormat {
    Compact,
    Standard,
}

impl PinFormat {
    /// 输入引脚：Normal 使用标准格式；Safety 使用紧凑格式。
    pub fn for_input(variant: Variant) -> Self {
        match variant {
            Variant::Normal => PinFormat::Standard,
            Variant::Safety => PinFormat::Compact,
        }
    }

    /// 输出引脚：Normal 使用标准格式；Safety 使用紧凑格式。
    pub fn for_output(variant: Variant) -> Self {
        match variant {
            Variant::Normal => PinFormat::Standard,
            Variant::Safety => PinFormat::Compact,
        }
    }
}

/// CLDElement 基类：来自 CLDElement::Serialize 的字段顺序。
/// 说明：type_id 是 u8；name/comment/desc 为 MfcString；后跟连接列表。
#[binread]
#[br(little)]
pub struct CLDElement {
    pub id: u32,
    pub type_id: u8,
    pub name: MfcString,
    pub comment: MfcString,
    pub desc: MfcString,
    pub conn_count: u32,
    #[br(count = conn_count)]
    pub conns: Vec<u32>,
}

/// 触点（CLDContact）：Base + flag(u8)。
/// Normal 版额外包含一个 CString（IDA 中通过 sub_1000A380/sub_10022780 读写）。
/// 该字段可能是几何/位置信息（暂以 MfcString 占位）。
#[binread]
#[br(little, import(variant: Variant))]
pub struct CLDContact {
    pub base: CLDElement,
    pub flag: u8,
    #[br(if(variant == Variant::Normal))]
    pub geo: Option<MfcString>,
}

/// 线圈（CLDOutput）：Base + flag/flag2，Normal 可能还有版本相关的 flag3。
/// 末尾有一个 CString（疑似几何/位置信息），暂以 MfcString 表示。
#[binread]
#[br(little, import(variant: Variant, serialize_version: u32))]
pub struct CLDOutput {
    pub base: CLDElement,
    pub flag: u8,
    pub flag2: u8,
    #[br(if(variant == Variant::Normal && serialize_version > 0))]
    pub flag3: Option<u8>,
    pub geo: MfcString,
}

/// CLDBox 在 Normal 版本（serialize_version >= 6）会多出两个 u32。
#[binread]
#[br(little)]
pub struct BoxVersionFields {
    pub extra_a: u32,
    pub extra_b: u32,
}

/// 引脚结构：根据 PinFormat 决定是否带 flag/binding_id。
#[binread]
#[br(little, import(format: PinFormat))]
pub struct CLDPin {
    #[br(if(matches!(format, PinFormat::Standard)))]
    pub flag0: Option<u8>,
    #[br(if(matches!(format, PinFormat::Standard)))]
    pub flag1: Option<u8>,
    pub name: MfcString,
    pub var: MfcString,
    #[br(if(matches!(format, PinFormat::Standard)))]
    pub binding_id: Option<u32>,
}

/// CLDBox（功能块）：Base + 可选版本字段 + flag + geo + 输入/输出引脚列表。
/// geo 在 IDA 中通过 CString 读写函数处理，暂以 MfcString 占位。
#[binread]
#[br(little, import(variant: Variant, serialize_version: u32))]
pub struct CLDBox {
    pub base: CLDElement,
    #[br(if(variant == Variant::Normal && serialize_version >= 6))]
    pub ver_fields: Option<BoxVersionFields>,
    pub flag: u8,
    pub geo: MfcString,
    pub input_count: u32,
    #[br(count = input_count, args { inner: (PinFormat::for_input(variant),) })]
    pub input_pins: Vec<CLDPin>,
    pub output_count: u32,
    #[br(count = output_count, args { inner: (PinFormat::for_output(variant),) })]
    pub output_pins: Vec<CLDPin>,
}

/// 默认序列化版本：Normal >= 6 的逻辑依赖该值。
pub const DEFAULT_SERIALIZE_VERSION: u32 = 6;
const DEFAULT_SYMBOL_CONFIG_PATH: &str = "config/symbols_config.json";

/// MFC 二进制读取器（用于解析 Hollysys 的剪贴板数据）
pub struct MfcReader<'a> {
    inner: Cursor<&'a [u8]>,
}

impl<'a> MfcReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { inner: Cursor::new(data) }
    }

    fn position(&self) -> usize {
        self.inner.position() as usize
    }

    fn remaining_len(&self) -> usize {
        let pos = self.position();
        let len = self.inner.get_ref().len();
        len.saturating_sub(pos)
    }

    fn remaining_slice(&self) -> &'a [u8] {
        let pos = self.position();
        &self.inner.get_ref()[pos..]
    }

    fn remaining_all_zero(&self) -> bool {
        self.remaining_slice().iter().all(|v| *v == 0)
    }

    fn seek_to(&mut self, pos: usize) -> Result<()> {
        if pos > self.inner.get_ref().len() {
            bail!("seek 超出数据范围: {}", pos);
        }
        self.inner.set_position(pos as u64);
        Ok(())
    }


    fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(&mut self) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.inner.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    fn peek_u8(&self) -> Result<u8> {
        let pos = self.position();
        let buf = self.inner.get_ref();
        if pos >= buf.len() {
            bail!("到达数据末尾，无法继续读取");
        }
        Ok(buf[pos])
    }

    fn peek_u16(&self) -> Result<u16> {
        let pos = self.position();
        let buf = self.inner.get_ref();
        if pos + 2 > buf.len() {
            bail!("到达数据末尾，无法继续读取");
        }
        Ok(u16::from_le_bytes([buf[pos], buf[pos + 1]]))
    }

    fn peek_u32(&self) -> Result<u32> {
        let pos = self.position();
        let buf = self.inner.get_ref();
        if pos + 4 > buf.len() {
            bail!("到达数据末尾，无法继续读取");
        }
        Ok(u32::from_le_bytes([buf[pos], buf[pos + 1], buf[pos + 2], buf[pos + 3]]))
    }

    fn align_to_4bytes(&mut self) -> Result<()> {
        let remainder = self.position() % 4;
        if remainder != 0 {
            let padding = 4 - remainder;
            let _ = self.read_bytes(padding)?;
        }
        Ok(())
    }

    fn read_mfc_string(&mut self) -> Result<String> {
        let len_u8 = self.read_u8()? as usize;
        let len = if len_u8 == 0xFF {
            self.read_u16()? as usize
        } else {
            len_u8
        };
        let buf = self.read_bytes(len)?;
        let (cow, _, _) = GBK.decode(&buf);
        Ok(cow.into_owned())
    }
}

fn read_len32_string(reader: &mut MfcReader, max_len: usize) -> Result<Option<String>> {
    if reader.remaining_len() < 4 {
        return Ok(None);
    }
    let len = reader.peek_u32()? as usize;
    if len == 0 || len > max_len {
        return Ok(None);
    }
    if reader.remaining_len() < 4 + len {
        return Ok(None);
    }
    let _ = reader.read_u32()?;
    let buf = reader.read_bytes(len)?;
    let (cow, _, _) = GBK.decode(&buf);
    Ok(Some(cow.into_owned()))
}

fn read_element_string(reader: &mut MfcReader, variant: Variant, max_len: usize) -> Result<String> {
    if variant == Variant::Safety {
        if let Some(value) = read_len32_string(reader, max_len)? {
            return Ok(value);
        }
    }
    reader.read_mfc_string()
}

fn read_element_fields(
    reader: &mut MfcReader,
    variant: Variant,
) -> Result<(String, String, String, Vec<i32>)> {
    let name = read_element_string(reader, variant, 120)?;
    let (comment, desc) = if variant == Variant::Normal {
        (
            read_element_string(reader, variant, 160)?,
            read_element_string(reader, variant, 200)?,
        )
    } else {
        (String::new(), String::new())
    };
    let conn_count = reader.read_u32()? as usize;
    if conn_count > 20000 {
        bail!("连接数量异常: {}", conn_count);
    }
    let mut connections = Vec::new();
    if conn_count > 0 {
        connections = Vec::with_capacity(conn_count);
        for _ in 0..conn_count {
            let conn_u32 = reader.read_u32()?;
            let conn = checked_i32(conn_u32, "element.conn_id")?;
            connections.push(conn);
        }
    }
    Ok((name, comment, desc, connections))
}

/// 读取类签名 (MFC Class Signature)
fn read_class_sig(reader: &mut MfcReader) -> Result<String> {
    let magic = reader.read_u16()?;
    if magic != 0xFFFF {
        bail!("类签名 Magic 错误: 0x{:04X}", magic);
    }
    let _schema = reader.read_u16()?;
    let name_len = reader.read_u16()? as usize;
    let name_bytes = reader.read_bytes(name_len)?;
    let (cow, _, _) = GBK.decode(&name_bytes);
    Ok(cow.into_owned())
}

/// 读取 POU 头部，返回 POU 名称
fn read_header(reader: &mut MfcReader, variant: Variant) -> Result<String> {
    // [1] 第一次 POU 名称
    let name = reader.read_mfc_string()?;
    reader.align_to_4bytes()?;

    // [2] 时间戳（Normal 才有）
    if variant == Variant::Normal {
        let _ = reader.read_u32()?;
    }

    // [3] 第二次名称 + 对齐
    let _ = reader.read_mfc_string()?;
    reader.align_to_4bytes()?;

    // [4] Metadata Flags
    match variant {
        Variant::Normal => {
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
        }
        Variant::Safety => {
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
        }
    }

    // [5] Language + 返回类型区
    let _ = reader.read_u32()?;             // Language ID
    let _ = reader.read_mfc_string()?;      // 返回类型
    let _ = reader.read_u32()?;             // Flag1
    let _ = reader.read_u32()?;             // Flag2
    match variant {
        Variant::Normal => {
            let _ = reader.read_mfc_string()?; // Normal: 空字符串
            let _ = reader.read_mfc_string()?; // Normal: 额外 BOOL
        }
        Variant::Safety => {
            let _ = reader.read_u32()?;       // Safety: 额外 flag3
            let _ = reader.read_u32()?;       // Safety: 额外 flag4
        }
    }

    Ok(name)
}

fn read_network(reader: &mut MfcReader, variant: Variant) -> Result<Network> {
    let id_u32 = reader.read_u32()?;
    let id = checked_i32(id_u32, "network.id")?;
    match variant {
        Variant::Normal => {
            let _ = reader.read_u16()?; // type
            let _ = reader.read_u16()?; // flag
            let _ = reader.read_u16()?; // rung
            let _ = reader.read_u16()?; // pad
        }
        Variant::Safety => {
            let _ = reader.read_u16()?; // type
            let _ = reader.read_u32()?; // flag
            let _ = reader.read_u32()?; // rung
        }
    }
    let label = reader.read_mfc_string()?;
    let comment = reader.read_mfc_string()?;

    Ok(Network {
        id,
        label,
        comment,
        elements: Vec::new(),
        safety_topology: Vec::new(),
    })
}

fn read_element_base(
    reader: &mut MfcReader,
    variant: Variant,
) -> Result<(i32, u8, String, String, String, Vec<i32>)> {
    let id_u32 = reader.read_u32()?;
    let id = checked_i32(id_u32, "element.id")?;
    let type_id = reader.read_u8()?;
    let (name, comment, desc, connections) = read_element_fields(reader, variant)?;
    Ok((id, type_id, name, comment, desc, connections))
}

fn read_contact(reader: &mut MfcReader, variant: Variant) -> Result<LdElement> {
    let (id, _type_id, name, comment, desc, connections) = read_element_base(reader, variant)?;
    if variant == Variant::Safety {
        let _ = skip_safety_reserved_u16(reader)?;
    }
    let sub_type = reader.read_u8()?;
    if variant == Variant::Normal {
        let _ = reader.read_mfc_string()?;
    }
    Ok(LdElement {
        id,
        type_code: ElementType::Contact,
        name,
        comment,
        desc,
        instance: String::new(),
        pins: Vec::new(),
        connections,
        sub_type,
    })
}

fn read_output(reader: &mut MfcReader, variant: Variant, serialize_version: u32) -> Result<LdElement> {
    let (id, _type_id, name, comment, desc, connections) = read_element_base(reader, variant)?;
    if variant == Variant::Safety {
        let _ = skip_safety_reserved_u16(reader)?;
    }
    let sub_type = reader.read_u8()?;
    let _ = reader.read_u8()?; // flag2
    if variant == Variant::Normal && serialize_version > 0 {
        let _ = reader.read_u8()?; // flag3
    }
    let _ = reader.read_mfc_string()?; // geo/附加字段
    Ok(LdElement {
        id,
        type_code: ElementType::Coil,
        name,
        comment,
        desc,
        instance: String::new(),
        pins: Vec::new(),
        connections,
        sub_type,
    })
}

fn read_pin(
    reader: &mut MfcReader,
    variant: Variant,
    _serialize_version: u32,
    direction: PinDirection,
) -> Result<BoxPin> {
    match variant {
        Variant::Safety => {
            let name = read_element_string(reader, variant, 80)?;
            let variable = read_element_string(reader, variant, 200)?;
            Ok(BoxPin { name, variable, direction })
        }
        Variant::Normal => {
            let _ = reader.read_u8()?; // flag0
            let _ = reader.read_u8()?; // flag1
            let name = reader.read_mfc_string()?;
            let variable = reader.read_mfc_string()?;
            if direction == PinDirection::Input {
                let _ = reader.read_u32()?; // binding_id
            }
            Ok(BoxPin { name, variable, direction })
        }
    }
}

fn read_box(reader: &mut MfcReader, variant: Variant, serialize_version: u32) -> Result<LdElement> {
    let (id, _type_id, name, comment, desc, connections) = read_element_base(reader, variant)?;
    if variant == Variant::Normal && serialize_version >= 6 {
        let _ = reader.read_u32()?;
        let _ = reader.read_u32()?;
    }
    if variant == Variant::Safety {
        skip_safety_box_padding(reader)?;
    }
    let _ = reader.read_u8()?; // flag
    if variant == Variant::Safety && reader.remaining_len() >= 2 && reader.peek_u16()? == 0x0100 {
        let _ = reader.read_u16()?;
    }
    let instance = read_element_string(reader, variant, 200)?;

    let input_count = reader.read_u32()? as usize;
    let mut pins = Vec::new();
    for _ in 0..input_count {
        pins.push(read_pin(reader, variant, serialize_version, PinDirection::Input)?);
    }

    let output_count = reader.read_u32()? as usize;
    for _ in 0..output_count {
        pins.push(read_pin(reader, variant, serialize_version, PinDirection::Output)?);
    }

    if variant == Variant::Safety
        && !looks_like_class_sig(reader)
        && !looks_like_safety_var_table(reader)
        && !looks_like_element_object(reader, variant)
    {
        // Safety 的部分 CLDBox 在引脚后仍包含内部数据块（如嵌套拓扑）。
        // 直接跳到下一个类签名，保持顶层 CObList 对齐。
        skip_network_tail(reader)?;
    }

    Ok(LdElement {
        id,
        type_code: ElementType::Box,
        name,
        comment,
        desc,
        instance,
        pins,
        connections,
        sub_type: 0,
    })
}

fn read_element_dynamic(
    reader: &mut MfcReader,
    variant: Variant,
    serialize_version: u32,
) -> Result<LdElement> {
    let (id, type_id, name, comment, desc, connections) = read_element_base(reader, variant)?;
    let type_code = element_type_from_id(variant, type_id)?;
    match type_code {
        ElementType::Box => {
            if variant == Variant::Normal && serialize_version >= 6 {
                let _ = reader.read_u32()?;
                let _ = reader.read_u32()?;
            }
            if variant == Variant::Safety {
                skip_safety_box_padding(reader)?;
            }
            let _ = reader.read_u8()?; // flag
            if variant == Variant::Safety && reader.remaining_len() >= 2 && reader.peek_u16()? == 0x0100 {
                let _ = reader.read_u16()?;
            }
            let instance = read_element_string(reader, variant, 200)?;

            let input_count = reader.read_u32()? as usize;
            let mut pins = Vec::new();
            for _ in 0..input_count {
                pins.push(read_pin(reader, variant, serialize_version, PinDirection::Input)?);
            }

            let output_count = reader.read_u32()? as usize;
            for _ in 0..output_count {
                pins.push(read_pin(reader, variant, serialize_version, PinDirection::Output)?);
            }

            if variant == Variant::Safety
                && !looks_like_class_sig(reader)
                && !looks_like_safety_var_table(reader)
                && !looks_like_element_object(reader, variant)
            {
                skip_network_tail(reader)?;
            }

            Ok(LdElement {
                id,
                type_code,
                name,
                comment,
                desc,
                instance,
                pins,
                connections,
                sub_type: 0,
            })
        }
        ElementType::Contact => {
            if variant == Variant::Safety {
                let _ = skip_safety_reserved_u16(reader)?;
            }
            let sub_type = reader.read_u8()?;
            if variant == Variant::Normal {
                let _ = reader.read_mfc_string()?;
            }
            Ok(LdElement {
                id,
                type_code,
                name,
                comment,
                desc,
                instance: String::new(),
                pins: Vec::new(),
                connections,
                sub_type,
            })
        }
        ElementType::Coil => {
            if variant == Variant::Safety {
                let _ = skip_safety_reserved_u16(reader)?;
            }
            let sub_type = reader.read_u8()?;
            let _ = reader.read_u8()?; // flag2
            if variant == Variant::Normal && serialize_version > 0 {
                let _ = reader.read_u8()?; // flag3
            }
            let _ = reader.read_mfc_string()?;
            Ok(LdElement {
                id,
                type_code,
                name,
                comment,
                desc,
                instance: String::new(),
                pins: Vec::new(),
                connections,
                sub_type,
            })
        }
        _ => {
            bail!("动态元素类型不支持: {:?}", type_code);
        }
    }
}

#[derive(Default)]
struct ClassTable {
    classes: Vec<String>,
}

impl ClassTable {
    fn insert(&mut self, name: String) {
        self.classes.push(name);
    }

    fn get(&self, id: u16) -> Result<String> {
        if id == 0 || id as usize > self.classes.len() {
            bail!("未知的类引用 ID: {}", id);
        }
        Ok(self.classes[(id as usize) - 1].clone())
    }
}

#[derive(Debug, Clone)]
enum ClassTag {
    None,
    Known(String),
    UnknownRef(u16),
}

fn read_class_tag(reader: &mut MfcReader, table: &mut ClassTable) -> Result<ClassTag> {
    let tag = reader.read_u16()?;
    if tag == 0x0000 {
        return Ok(ClassTag::None);
    }
    if tag == 0xFFFF {
        let _schema = reader.read_u16()?;
        let name_len = reader.read_u16()? as usize;
        let name_bytes = reader.read_bytes(name_len)?;
        let (cow, _, _) = GBK.decode(&name_bytes);
        let name = cow.into_owned();
        table.insert(name.clone());
        return Ok(ClassTag::Known(name));
    }
    if tag & 0x8000 != 0 {
        let class_id = tag & 0x7FFF;
        if class_id == 0 || class_id as usize > table.classes.len() {
            return Ok(ClassTag::UnknownRef(class_id));
        }
        return table.get(class_id).map(ClassTag::Known);
    }
    bail!("类签名 Tag 错误: 0x{:04X}", tag);
}

fn read_networks(reader: &mut MfcReader, variant: Variant, serialize_version: u32) -> Result<Vec<Network>> {
    if variant == Variant::Safety {
        return read_networks_safety(reader, serialize_version);
    }

    let list = seek_to_network_list_start(reader)?;
    let stop_at = find_normal_var_table_offset(reader).map(|offset| reader.position() + offset);
    let mut remaining = list.count;
    let mut networks: Vec<Network> = Vec::new();
    let mut current: Option<Network> = None;
    let mut class_table = ClassTable::default();

    loop {
        if reader.remaining_len() == 0 {
            break;
        }
        if let Some(stop_at) = stop_at {
            if reader.position() >= stop_at {
                break;
            }
        }
        if remaining == Some(0) {
            break;
        }

        let pos = reader.position();
        let class_tag = read_class_tag(reader, &mut class_table)?;
        if let Some(rem) = remaining.as_mut() {
            *rem = rem.saturating_sub(1);
        }

        match class_tag {
            ClassTag::None => {}
            ClassTag::Known(class_name) => match class_name.as_str() {
                "CLDNetwork" => {
                    if let Some(net) = current.take() {
                        networks.push(net);
                    }
                    if !looks_like_network(reader, variant, stop_at) {
                        skip_network_tail(reader)?;
                        continue;
                    }
                    let net = read_network(reader, variant)?;
                    skip_network_tail(reader)?;
                    current = Some(net);
                }
                "CLDContact" => {
                    let elem = read_contact(reader, variant)?;
                    if let Some(net) = current.as_mut() {
                        net.elements.push(elem);
                    } else {
                        bail!("元素出现在网络之前: {}", class_name);
                    }
                }
                "CLDOutput" => {
                    let elem = read_output(reader, variant, serialize_version)?;
                    if let Some(net) = current.as_mut() {
                        net.elements.push(elem);
                    } else {
                        bail!("元素出现在网络之前: {}", class_name);
                    }
                }
                "CLDBox" => {
                    let elem = read_box(reader, variant, serialize_version)?;
                    if let Some(net) = current.as_mut() {
                        net.elements.push(elem);
                    } else {
                        bail!("元素出现在网络之前: {}", class_name);
                    }
                }
                "CLDAssign" => {
                    let _ = reader.read_u32()?;
                    let _ = reader.read_u32()?;
                    let _ = reader.read_u32()?;
                    let _ = reader.read_u32()?;
                }
                "CLDElement" => {
                    let _ = read_element_base(reader, variant)?;
                }
                _ => {
                    bail!("不支持的类签名: {}", class_name);
                }
            },
            ClassTag::UnknownRef(_) => {
                if let Some(type_id) = peek_element_type_id(reader) {
                    if is_element_type_id(variant, type_id) {
                        let elem = read_element_dynamic(reader, variant, serialize_version)?;
                        if let Some(net) = current.as_mut() {
                            net.elements.push(elem);
                        } else {
                            bail!("元素出现在网络之前: UnknownRef");
                        }
                    } else {
                        skip_network_tail(reader)?;
                    }
                } else {
                    break;
                }
            }
        }

        if reader.position() == pos {
            break;
        }
    }

    if let Some(net) = current.take() {
        networks.push(net);
    }
    if let Some(stop_at) = stop_at {
        if reader.position() < stop_at {
            reader.seek_to(stop_at)?;
        }
    }
    Ok(networks)
}

#[derive(Debug, Clone)]
struct SafetyNode {
    elem: LdElement,
    children: Vec<SafetyNode>,
    label: Option<String>,
    comment: Option<String>,
}

fn read_safety_string(reader: &mut MfcReader, max_len: usize) -> Result<String> {
    if reader.remaining_len() < 4 {
        bail!("Safety 字符串长度不足");
    }
    let len = reader.peek_u32()? as usize;
    if len <= max_len && reader.remaining_len() >= 4 + len {
        let _ = reader.read_u32()?;
        let buf = reader.read_bytes(len)?;
        let (cow, _, _) = GBK.decode(&buf);
        return Ok(cow.into_owned());
    }
    reader.read_mfc_string()
}

fn read_safety_string_optional(reader: &mut MfcReader, max_len: usize) -> Result<Option<String>> {
    if reader.remaining_len() < 4 {
        return Ok(None);
    }
    let len = reader.peek_u32()? as usize;
    if len > max_len || reader.remaining_len() < 4 + len {
        return Ok(None);
    }
    let _ = reader.read_u32()?;
    let buf = reader.read_bytes(len)?;
    let (cow, _, _) = GBK.decode(&buf);
    Ok(Some(cow.into_owned()))
}

fn element_type_from_safety_id(type_id: u8) -> Option<ElementType> {
    match type_id {
        0x03 => Some(ElementType::Box),
        0x04 => Some(ElementType::Contact),
        0x05 => Some(ElementType::Coil),
        0x08 => Some(ElementType::Assign),
        0x09 => Some(ElementType::Network),
        _ => None,
    }
}

fn read_safety_base(
    reader: &mut MfcReader,
    expected_type: Option<u8>,
) -> Result<(i32, u8, String, Vec<i32>, u32)> {
    let id_u32 = reader.read_u32()?;
    let id = checked_i32(id_u32, "safety.element.id")?;
    let type_id = reader.read_u8()?;
    if let Some(expect) = expected_type {
        if expect != type_id {
            debug!(
                "safety element type mismatch: expected=0x{:02X}, got=0x{:02X}",
                expect, type_id
            );
        }
    }
    let name = read_safety_string(reader, 120)?;
    let conn_count = reader.read_u32()? as usize;
    if conn_count > 20000 {
        bail!("Safety 连接数量异常: {}", conn_count);
    }
    let mut connections = Vec::with_capacity(conn_count);
    for _ in 0..conn_count {
        let conn_u32 = reader.read_u32()?;
        let conn = checked_i32(conn_u32, "safety.element.conn_id")?;
        connections.push(conn);
    }
    let child_count = reader.read_u32()?;
    if child_count > 20000 {
        bail!("Safety 子元素数量异常: {}", child_count);
    }
    Ok((id, type_id, name, connections, child_count))
}

fn read_safety_pin(reader: &mut MfcReader, direction: PinDirection) -> Result<BoxPin> {
    let name = read_safety_string(reader, 80)?;
    let variable = read_safety_string(reader, 200)?;
    Ok(BoxPin { name, variable, direction })
}

fn read_safety_node(
    reader: &mut MfcReader,
    expected_type: Option<u8>,
    serialize_version: u32,
) -> Result<SafetyNode> {
    let (id, type_id, name, connections, child_count) = read_safety_base(reader, expected_type)?;
    let mut children = Vec::with_capacity(child_count as usize);
    for _ in 0..child_count {
        let child_type = reader.read_u8()?;
        children.push(read_safety_node(reader, Some(child_type), serialize_version)?);
    }

    let mut label = None;
    let mut comment = None;
    let mut instance = String::new();
    let mut pins = Vec::new();
    let mut sub_type = 0u8;
    let type_code = element_type_from_safety_id(type_id).unwrap_or(ElementType::Assign);

    match type_code {
        ElementType::Network => {
            label = Some(read_safety_string(reader, 120)?);
            comment = Some(read_safety_string(reader, 200)?);
        }
        ElementType::Box => {
            let _flag = reader.read_u8()?;
            let inst_comment = read_safety_string(reader, 200)?;
            let inst_name = read_safety_string(reader, 200)?;
            instance = inst_name;
            let input_count = reader.read_u32()? as usize;
            if input_count > 2000 {
                bail!("Safety Box 输入数量异常: {}", input_count);
            }
            for _ in 0..input_count {
                pins.push(read_safety_pin(reader, PinDirection::Input)?);
            }
            let output_count = reader.read_u32()? as usize;
            if output_count > 2000 {
                bail!("Safety Box 输出数量异常: {}", output_count);
            }
            for _ in 0..output_count {
                pins.push(read_safety_pin(reader, PinDirection::Output)?);
            }
            if !inst_comment.is_empty() {
                comment = Some(inst_comment);
            }
        }
        ElementType::Contact => {
            if reader.remaining_len() >= 2 && reader.peek_u16()? == 0x0000 {
                let _ = reader.read_u16()?;
            }
            sub_type = reader.read_u8()?;
        }
        ElementType::Coil => {
            if reader.remaining_len() >= 2 && reader.peek_u16()? == 0x0000 {
                let _ = reader.read_u16()?;
            }
            sub_type = reader.read_u8()?;
            let _ = reader.read_u8()?;
            let _ = read_safety_string_optional(reader, 200)?;
        }
        ElementType::Assign => {}
    }

    Ok(SafetyNode {
        elem: LdElement {
            id,
            type_code,
            name,
            comment: comment.clone().unwrap_or_default(),
            desc: String::new(),
            instance,
            pins,
            connections,
            sub_type,
        },
        children,
        label,
        comment,
    })
}

fn read_safety_element_list(
    reader: &mut MfcReader,
    serialize_version: u32,
) -> Result<Vec<SafetyNode>> {
    let count = reader.read_u32()? as usize;
    if count > 20000 {
        bail!("Safety 列表数量异常: {}", count);
    }
    let mut nodes = Vec::with_capacity(count);
    for _ in 0..count {
        let type_id = reader.read_u8()?;
        nodes.push(read_safety_node(reader, Some(type_id), serialize_version)?);
    }
    Ok(nodes)
}

fn collect_safety_elements(node: &SafetyNode, out: &mut Vec<LdElement>) {
    match node.elem.type_code {
        ElementType::Box | ElementType::Contact | ElementType::Coil => {
            out.push(node.elem.clone());
        }
        _ => {}
    }
    for child in &node.children {
        collect_safety_elements(child, out);
    }
}

fn collect_safety_networks(node: &SafetyNode, out: &mut Vec<Network>) {
    if node.elem.type_code == ElementType::Network {
        let mut elements = Vec::new();
        for child in &node.children {
            collect_safety_elements(child, &mut elements);
        }
        out.push(Network {
            id: node.elem.id,
            label: node.label.clone().unwrap_or_default(),
            comment: node.comment.clone().unwrap_or_default(),
            elements,
            safety_topology: Vec::new(),
        });
    } else {
        for child in &node.children {
            collect_safety_networks(child, out);
        }
    }
}

fn read_networks_safety_tree(reader: &mut MfcReader, serialize_version: u32) -> Result<Vec<Network>> {
    let nodes = read_safety_element_list(reader, serialize_version)?;
    let mut networks = Vec::new();
    for node in &nodes {
        collect_safety_networks(node, &mut networks);
    }
    if networks.is_empty() && !nodes.is_empty() {
        let mut elements = Vec::new();
        for node in &nodes {
            collect_safety_elements(node, &mut elements);
        }
        networks.push(Network {
            id: 0,
            label: String::new(),
            comment: String::new(),
            elements,
            safety_topology: Vec::new(),
        });
    }
    Ok(networks)
}

fn read_networks_safety(reader: &mut MfcReader, serialize_version: u32) -> Result<Vec<Network>> {
    let start = reader.position();
    if let Ok(nets) = read_networks_safety_tree(reader, serialize_version) {
        if !nets.is_empty() {
            return Ok(nets);
        }
    }
    reader.seek_to(start)?;
    read_networks_safety_class(reader, serialize_version)
}

fn read_networks_safety_class(reader: &mut MfcReader, serialize_version: u32) -> Result<Vec<Network>> {
    let list = seek_to_network_list_start(reader)?;
    let stop_at = find_safety_var_table_offset(reader).map(|offset| reader.position() + offset);
    let mut remaining = list.count;
    let mut networks: Vec<Network> = Vec::new();
    let mut current: Option<Network> = None;
    let mut class_table = ClassTable::default();

    loop {
        if reader.remaining_len() == 0 {
            break;
        }
        if let Some(stop_at) = stop_at {
            if reader.position() >= stop_at {
                break;
            }
        }
        if remaining == Some(0) {
            break;
        }

        let pos = reader.position();
        let class_tag = read_class_tag(reader, &mut class_table)?;
        if let Some(rem) = remaining.as_mut() {
            *rem = rem.saturating_sub(1);
        }

        match class_tag {
            ClassTag::None => {}
            ClassTag::Known(class_name) => match class_name.as_str() {
                "CLDNetwork" => {
                    if let Some(net) = current.take() {
                        networks.push(net);
                    }
                    let id = if reader.remaining_len() >= 4 {
                        reader.read_u32().unwrap_or(0) as i32
                    } else {
                        0
                    };
                    current = Some(Network {
                        id,
                        label: String::new(),
                        comment: String::new(),
                        elements: Vec::new(),
                        safety_topology: Vec::new(),
                    });
                    skip_network_tail(reader)?;
                }
                "CLDContact" => {
                    let elem = read_contact(reader, Variant::Safety)?;
                    if let Some(net) = current.as_mut() {
                        net.elements.push(elem);
                    } else {
                        bail!("元素出现在网络之前: {}", class_name);
                    }
                }
                "CLDOutput" => {
                    let elem = read_output(reader, Variant::Safety, serialize_version)?;
                    if let Some(net) = current.as_mut() {
                        net.elements.push(elem);
                    } else {
                        bail!("元素出现在网络之前: {}", class_name);
                    }
                }
                "CLDBox" => {
                    let elem = read_box(reader, Variant::Safety, serialize_version)?;
                    if let Some(net) = current.as_mut() {
                        net.elements.push(elem);
                    } else {
                        bail!("元素出现在网络之前: {}", class_name);
                    }
                }
                "CLDAssign" => {
                    let elem = read_safety_assign(reader)?;
                    if let Some(net) = current.as_mut() {
                        net.elements.push(elem);
                    } else {
                        bail!("元素出现在网络之前: {}", class_name);
                    }
                }
                "CLDElement" => {
                    let _ = read_element_base(reader, Variant::Safety)?;
                }
                _ => {
                    skip_network_tail(reader)?;
                }
            },
            ClassTag::UnknownRef(_) => {
                if let Some(type_id) = peek_element_type_id(reader) {
                    if is_element_type_id(Variant::Safety, type_id) {
                        let elem = read_element_dynamic(reader, Variant::Safety, serialize_version)?;
                        if let Some(net) = current.as_mut() {
                            net.elements.push(elem);
                        } else {
                            bail!("元素出现在网络之前: UnknownRef");
                        }
                    } else {
                        skip_network_tail(reader)?;
                    }
                } else {
                    break;
                }
            }
        }

        if reader.position() == pos {
            break;
        }
    }

    if let Some(net) = current.take() {
        networks.push(net);
    }
    if let Some(stop_at) = stop_at {
        if reader.position() < stop_at {
            reader.seek_to(stop_at)?;
        }
    }
    Ok(networks)
}

fn looks_like_class_sig(reader: &MfcReader) -> bool {
    let buf = reader.remaining_slice();
    if buf.len() < 6 {
        return false;
    }
    if u16::from_le_bytes([buf[0], buf[1]]) != 0xFFFF {
        return false;
    }
    let name_len = u16::from_le_bytes([buf[4], buf[5]]) as usize;
    if name_len == 0 || name_len > 64 {
        return false;
    }
    if buf.len() < 6 + name_len {
        return false;
    }
    buf[6..6 + name_len].iter().all(|b| b.is_ascii_graphic())
}

fn is_element_type_id(variant: Variant, type_id: u8) -> bool {
    match variant {
        Variant::Normal => matches!(type_id, 0x03 | 0x05 | 0x06),
        Variant::Safety => matches!(type_id, 0x03 | 0x04 | 0x05),
    }
}

fn peek_element_type_id(reader: &MfcReader) -> Option<u8> {
    let buf = reader.remaining_slice();
    if buf.len() < 5 {
        return None;
    }
    Some(buf[4])
}

fn looks_like_element_object(reader: &MfcReader, variant: Variant) -> bool {
    let buf = reader.remaining_slice();
    if buf.len() < 10 {
        return false;
    }
    let tag = u16::from_le_bytes([buf[0], buf[1]]);
    if tag & 0x8000 == 0 {
        return false;
    }
    let type_id = buf[6];
    if !is_element_type_id(variant, type_id) {
        return false;
    }
    let mut idx = 7usize;
    if !scan_mfc_string_any(buf, &mut idx, 80).unwrap_or(false) {
        return false;
    }
    if !scan_mfc_string_any(buf, &mut idx, 80).unwrap_or(false) {
        return false;
    }
    if !scan_mfc_string_any(buf, &mut idx, 120).unwrap_or(false) {
        return false;
    }
    true
}

fn looks_like_network(reader: &MfcReader, variant: Variant, limit: Option<usize>) -> bool {
    let buf_all = reader.remaining_slice();
    let header_len = match variant {
        Variant::Normal => 4 + 2 + 2 + 2 + 2,
        Variant::Safety => 4 + 2 + 4 + 4,
    };
    let buf = match limit {
        Some(stop_at) => {
            let remaining = stop_at.saturating_sub(reader.position());
            if remaining == 0 {
                return false;
            }
            if remaining < buf_all.len() {
                &buf_all[..remaining]
            } else {
                buf_all
            }
        }
        None => buf_all,
    };
    if buf.len() < header_len + 2 {
        return false;
    }
    let mut idx = header_len;
    if !scan_mfc_string_any(buf, &mut idx, 80).unwrap_or(false) {
        return false;
    }
    if !scan_mfc_string_any(buf, &mut idx, 200).unwrap_or(false) {
        return false;
    }
    true
}

fn skip_network_tail(reader: &mut MfcReader) -> Result<()> {
    while reader.remaining_len() > 0 {
        if looks_like_class_sig(reader) || looks_like_safety_var_table(reader) {
            break;
        }
        let _ = reader.read_u8()?;
    }
    Ok(())
}

struct NetworkListStart {
    count: Option<usize>,
    count_len: usize,
    offset: usize,
}

fn seek_to_network_list_start(reader: &mut MfcReader) -> Result<NetworkListStart> {
    let start = reader.position();
    let buf = reader.inner.get_ref();
    if let Some(info) = find_network_list_start(&buf[start..]) {
        reader.seek_to(start + info.offset + info.count_len)?;
        return Ok(info);
    }
    if start > 0 {
        if let Some(info) = find_network_list_start(buf) {
            reader.seek_to(info.offset + info.count_len)?;
            return Ok(info);
        }
    }
    bail!("未找到网络列表起点");
}

fn find_network_list_start(buf: &[u8]) -> Option<NetworkListStart> {
    for offset in 0..buf.len().saturating_sub(10) {
        let count = u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]]) as usize;
        if count == 0 || count > 10000 {
            continue;
        }
        if u16::from_le_bytes([buf[offset + 4], buf[offset + 5]]) != 0xFFFF {
            continue;
        }
        let name_len = u16::from_le_bytes([buf[offset + 8], buf[offset + 9]]) as usize;
        if name_len == 0 || name_len > 64 {
            continue;
        }
        if offset + 10 + name_len > buf.len() {
            continue;
        }
        let name_bytes = &buf[offset + 10..offset + 10 + name_len];
        if name_bytes != b"CLDNetwork" {
            continue;
        }
        if !name_bytes.iter().all(|b| b.is_ascii_graphic()) {
            continue;
        }
        return Some(NetworkListStart { count: Some(count), count_len: 4, offset });
    }
    for offset in 0..buf.len().saturating_sub(8) {
        let count = u16::from_le_bytes([buf[offset], buf[offset + 1]]) as usize;
        if count == 0 || count > 5000 {
            continue;
        }
        if u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]) != 0xFFFF {
            continue;
        }
        let name_len = u16::from_le_bytes([buf[offset + 6], buf[offset + 7]]) as usize;
        if name_len == 0 || name_len > 64 {
            continue;
        }
        if offset + 8 + name_len > buf.len() {
            continue;
        }
        let name_bytes = &buf[offset + 8..offset + 8 + name_len];
        if name_bytes != b"CLDNetwork" {
            continue;
        }
        if !name_bytes.iter().all(|b| b.is_ascii_graphic()) {
            continue;
        }
        return Some(NetworkListStart { count: Some(count), count_len: 2, offset });
    }
    for offset in 0..buf.len().saturating_sub(6) {
        if u16::from_le_bytes([buf[offset], buf[offset + 1]]) != 0xFFFF {
            continue;
        }
        let name_len = u16::from_le_bytes([buf[offset + 4], buf[offset + 5]]) as usize;
        if name_len == 0 || name_len > 64 {
            continue;
        }
        if offset + 6 + name_len > buf.len() {
            continue;
        }
        let name_bytes = &buf[offset + 6..offset + 6 + name_len];
        if name_bytes != b"CLDNetwork" {
            continue;
        }
        if !name_bytes.iter().all(|b| b.is_ascii_graphic()) {
            continue;
        }
        return Some(NetworkListStart { count: None, count_len: 0, offset });
    }
    None
}

#[derive(Debug, Clone)]
enum SafetyTopologyEntry {
    BranchOpen,
    BranchClose,
    SeriesNext,
    NetEnd,
    BranchNext,
    ElementRef { id: u32, type_id: u16 },
    InlineElement(LdElement),
    Raw(u16),
}

fn read_safety_topology_raw(
    reader: &mut MfcReader,
    variant: Variant,
    serialize_version: u32,
) -> Result<(Vec<SafetyTopologyEntry>, Vec<LdElement>)> {
    let mut topology = Vec::new();
    let mut inline_elements = Vec::new();
    let start_pos = reader.position();
    let stop_at = {
        let class_offset = find_class_sig_ahead(reader, reader.remaining_len());
        let var_offset = find_safety_var_table_offset(reader);
        match (class_offset, var_offset) {
            (Some(a), Some(b)) => Some(start_pos + a.min(b)),
            (Some(a), None) => Some(start_pos + a),
            (None, Some(b)) => Some(start_pos + b),
            (None, None) => None,
        }
    };
    loop {
        if looks_like_safety_var_table(reader) || looks_like_safety_var_table_ahead(reader, 16) {
            break;
        }
        if looks_like_class_sig(reader) {
            let pos = reader.position();
            reader.seek_to(pos)?;
            break;
        }
        if let Some(stop_at) = stop_at {
            let pos = reader.position();
            if pos >= stop_at {
                reader.seek_to(stop_at)?;
                break;
            }
            let remaining = stop_at.saturating_sub(pos);
            if remaining < 2 {
                reader.seek_to(stop_at)?;
                break;
            }
            if remaining < 6 {
                if let Ok(tag) = reader.peek_u16() {
                    if tag < 0x8000 {
                        reader.seek_to(stop_at)?;
                        break;
                    }
                }
            }
        }
        if reader.remaining_len() < 2 {
            break;
        }
        let tag = reader.peek_u16()?;
        if tag >= 0x8000 {
            let token = reader.read_u16()?;
            match token {
                0x8001 => topology.push(SafetyTopologyEntry::BranchOpen),
                0x8003 => topology.push(SafetyTopologyEntry::BranchClose),
                0x8005 => topology.push(SafetyTopologyEntry::SeriesNext),
                0x8007 => topology.push(SafetyTopologyEntry::NetEnd),
                0x8009 => topology.push(SafetyTopologyEntry::BranchNext),
                0x800C | 0x800B => {
                    let elem = read_safety_inline_element(reader, variant, serialize_version)?;
                    inline_elements.push(elem.clone());
                    topology.push(SafetyTopologyEntry::InlineElement(elem));
                }
                _ => topology.push(SafetyTopologyEntry::Raw(token)),
            }
            continue;
        }
        let (id, type_id) = read_compact_element_header(reader)?;
        topology.push(SafetyTopologyEntry::ElementRef { id, type_id });
    }
    Ok((topology, inline_elements))
}

fn read_safety_inline_element(
    reader: &mut MfcReader,
    variant: Variant,
    serialize_version: u32,
) -> Result<LdElement> {
    let id_u32 = reader.read_u32()?;
    let id = checked_i32(id_u32, "inline_element.id")?;
    let type_id = reader.read_u8()?;
    let type_code = element_type_from_id(variant, type_id)?;
    if variant == Variant::Safety && type_code == ElementType::Network {
        if reader.remaining_len() < 8 {
            bail!("inline network 长度不足: id={}", id);
        }
        let _flag = reader.read_u32()?;
        let _rung_id = reader.read_u32()?;
        return Ok(LdElement {
            id,
            type_code,
            name: String::new(),
            comment: String::new(),
            desc: String::new(),
            instance: String::new(),
            pins: Vec::new(),
            connections: Vec::new(),
            sub_type: 0,
        });
    }

    let (name, comment, desc, connections) = read_element_fields(reader, variant)?;
    match type_code {
        ElementType::Box => {
            if variant == Variant::Safety {
                skip_safety_box_padding(reader)?;
            }
            let _flag = reader.read_u8()?;
            if variant == Variant::Safety && reader.remaining_len() >= 2 && reader.peek_u16()? == 0x0100 {
                let _ = reader.read_u16()?;
            }
            let instance = read_element_string(reader, variant, 200)?;

            let input_count = reader.read_u32()? as usize;
            let mut pins = Vec::new();
            for _ in 0..input_count {
                pins.push(read_pin(reader, variant, serialize_version, PinDirection::Input)?);
            }

            let output_count = reader.read_u32()? as usize;
            for _ in 0..output_count {
                pins.push(read_pin(reader, variant, serialize_version, PinDirection::Output)?);
            }

            Ok(LdElement {
                id,
                type_code,
                name,
                comment,
                desc,
                instance,
                pins,
                connections,
                sub_type: 0,
            })
        }
        ElementType::Contact => {
            if variant == Variant::Safety {
                let _ = skip_safety_reserved_u16(reader)?;
            }
            let sub_type = reader.read_u8()?;
            if variant == Variant::Normal {
                let _ = reader.read_mfc_string()?;
            }
            Ok(LdElement {
                id,
                type_code,
                name,
                comment,
                desc,
                instance: String::new(),
                pins: Vec::new(),
                connections,
                sub_type,
            })
        }
        ElementType::Coil => {
            if variant == Variant::Safety {
                let _ = skip_safety_reserved_u16(reader)?;
            }
            let sub_type = reader.read_u8()?;
            let _ = reader.read_u8()?; // flag2
            if variant == Variant::Normal && serialize_version > 0 {
                let _ = reader.read_u8()?; // flag3
            }
            let _ = reader.read_mfc_string()?; // geo/附加字段
            Ok(LdElement {
                id,
                type_code,
                name,
                comment,
                desc,
                instance: String::new(),
                pins: Vec::new(),
                connections,
                sub_type,
            })
        }
        ElementType::Assign => {
            bail!("inline element 不应为 Assign");
        }
        ElementType::Network => {
            bail!("inline element Network 解析失败: id={}", id);
        }
    }
}

fn read_safety_rung_list_with_topology(
    reader: &mut MfcReader,
    serialize_version: u32,
) -> Result<(Vec<i32>, Option<(Vec<SafetyTopologyEntry>, Vec<LdElement>)>)> {
    let start = reader.position();
    let mut rung_ids = Vec::new();
    let mut header_ok = false;

    if reader.remaining_len() >= 10 {
        let first = reader.read_u16()?;
        if first < 0x8000 {
            let second = reader.read_u16()?;
            let third = reader.read_u16()?;
            let count = reader.read_u16()? as usize;
            let fourth = reader.read_u16()?;
            if count > 0 && count <= 2000 && second == 0 && fourth == 0 {
                let needed = count.saturating_mul(4);
                if reader.remaining_len() >= needed {
                    header_ok = true;
                    for _ in 0..count {
                        let id = reader.read_u16()? as i32;
                        let _ = reader.read_u16()?; // padding
                        rung_ids.push(id);
                    }
                }
            }
            let _ = third; // reserved
        }
    }

    if !header_ok {
        reader.seek_to(start)?;
        rung_ids.clear();
    }

    if reader.remaining_len() == 0 || looks_like_class_sig(reader) {
        return Ok((rung_ids, None));
    }

    let topology = read_safety_topology_raw(reader, Variant::Safety, serialize_version)?;
    Ok((rung_ids, Some(topology)))
}

fn read_safety_assign(reader: &mut MfcReader) -> Result<LdElement> {
    let id_u32 = reader.read_u32()?;
    let id = checked_i32(id_u32, "assign.id")?;
    let type_id = reader.read_u16()?;
    let val1 = reader.read_u32()?;
    let val2 = reader.read_u32()?;

    // 余下结构尚不明确，先跳到下一个类签名以保持流对齐。
    skip_network_tail(reader)?;

    Ok(LdElement {
        id,
        type_code: ElementType::Assign,
        name: "CLDAssign".to_string(),
        comment: String::new(),
        desc: format!("type_id=0x{:04X}", type_id),
        instance: String::new(),
        pins: Vec::new(),
        connections: vec![val1 as i32, val2 as i32],
        sub_type: 0,
    })
}

fn read_compact_element_header(reader: &mut MfcReader) -> Result<(u32, u16)> {
    let id = reader.read_u32()?;
    let type_id = reader.read_u16()?;
    if type_id == 0x0009 {
        debug!("safety topology: type_id=0x0009 (Rung) id={}", id);
        if reader.remaining_len() >= 8 {
            let flag = reader.peek_u32()?;
            if flag <= 1 {
                let _flag = reader.read_u32()?;
                let _rung_id = reader.read_u32()?;
            }
        }
        if reader.remaining_len() >= 4 && reader.peek_u16()? == 0x0000 {
            let pos = reader.position();
            let _ = reader.read_u16()?;
            if !is_topology_boundary(reader) {
                reader.seek_to(pos)?;
            }
        }
        return Ok((id, type_id));
    }
    if !is_topology_boundary(reader) {
        let _flag = reader.read_u32()?;
        if !is_topology_boundary(reader) {
            let _rung = reader.read_u32()?;
        }
    }
    // 兼容少量 padding：0x0000 后紧跟 Token/ClassSig
    if reader.remaining_len() >= 4 && reader.peek_u16()? == 0x0000 {
        let pos = reader.position();
        let _ = reader.read_u16()?;
        if !is_topology_boundary(reader) {
            reader.seek_to(pos)?;
        }
    }
    Ok((id, type_id))
}

fn is_topology_boundary(reader: &MfcReader) -> bool {
    if looks_like_class_sig(reader) {
        return true;
    }
    if reader.remaining_len() < 2 {
        return true;
    }
    match reader.peek_u16() {
        Ok(tag) => tag >= 0x8000,
        Err(_) => true,
    }
}

fn resolve_safety_topology(raw: Vec<SafetyTopologyEntry>) -> Result<Vec<SafetyTopologyToken>> {
    let mut topology = Vec::with_capacity(raw.len());
    for entry in raw {
        match entry {
            SafetyTopologyEntry::BranchOpen => topology.push(SafetyTopologyToken::BranchOpen),
            SafetyTopologyEntry::BranchClose => topology.push(SafetyTopologyToken::BranchClose),
            SafetyTopologyEntry::SeriesNext => topology.push(SafetyTopologyToken::SeriesNext),
            SafetyTopologyEntry::NetEnd => topology.push(SafetyTopologyToken::NetEnd),
            SafetyTopologyEntry::BranchNext => topology.push(SafetyTopologyToken::BranchNext),
            SafetyTopologyEntry::ElementRef { id, type_id } => {
                topology.push(SafetyTopologyToken::ElementRef { id, type_id });
            }
            SafetyTopologyEntry::InlineElement(elem) => {
                topology.push(SafetyTopologyToken::InlineElement(Box::new(elem)));
            }
            SafetyTopologyEntry::Raw(value) => topology.push(SafetyTopologyToken::Raw(value)),
        }
    }
    Ok(topology)
}

fn skip_safety_reserved_u16(reader: &mut MfcReader) -> Result<bool> {
    if reader.remaining_len() < 3 {
        return Ok(false);
    }
    if reader.peek_u16()? != 0x0000 {
        return Ok(false);
    }
    let buf = reader.remaining_slice();
    if buf[2] <= 1 {
        let _ = reader.read_u16()?;
        return Ok(true);
    }
    Ok(false)
}

fn skip_safety_box_padding(reader: &mut MfcReader) -> Result<()> {
    if reader.remaining_len() < 2 {
        return Ok(());
    }

    let buf = reader.remaining_slice();
    if buf.len() >= 3 && buf[0] == 0 && buf[1] == 0 && buf[2] <= 1 {
        let _ = reader.read_bytes(2)?;
        return Ok(());
    }

    let padding = (4 - (reader.position() % 4)) % 4;
    if padding == 0 || reader.remaining_len() < padding + 1 {
        return Ok(());
    }
    let buf = reader.remaining_slice();
    if buf[..padding].iter().all(|b| *b == 0) && buf[padding] <= 1 {
        let _ = reader.read_bytes(padding)?;
    }
    Ok(())
}

fn placeholder_from_ref(id: i32, type_id: u16) -> Option<LdElement> {
    let type_code = match type_id {
        0x03 => ElementType::Box,
        0x04 => ElementType::Contact,
        0x05 => ElementType::Coil,
        0x08 => ElementType::Assign, // CLDAssign
        0x09 => ElementType::Network,
        _ => return None,
    };
    Some(LdElement {
        id,
        type_code,
        name: if type_id == 0x08 { "CLDAssign".to_string() } else { String::new() },
        comment: String::new(),
        desc: String::new(),
        instance: String::new(),
        pins: Vec::new(),
        connections: Vec::new(),
        sub_type: 0,
    })
}

fn nearest_network_index(network_order: &[(usize, usize)], element_order: usize) -> Option<usize> {
    if network_order.is_empty() {
        return None;
    }
    let mut candidate = None;
    for (order, net_idx) in network_order {
        if *order <= element_order {
            candidate = Some(*net_idx);
        } else {
            break;
        }
    }
    if candidate.is_some() {
        return candidate;
    }
    Some(network_order[0].1)
}

fn read_variables(reader: &mut MfcReader, variant: Variant) -> Result<Vec<Variable>> {
    match variant {
        Variant::Normal => read_variables_normal(reader),
        Variant::Safety => read_variables_safety(reader),
    }
}

fn read_variables_normal(reader: &mut MfcReader) -> Result<Vec<Variable>> {
    let start = reader.position();
    if let Ok(vars) = try_read_variables_normal(reader) {
        return Ok(vars);
    }
    reader.seek_to(start)?;
    seek_to_normal_var_table(reader)?;
    try_read_variables_normal(reader)
}

fn try_read_variables_normal(reader: &mut MfcReader) -> Result<Vec<Variable>> {
    let _ = try_read_normal_var_header(reader)?;
    let mut vars = Vec::new();
    while reader.remaining_len() > 0 {
        skip_normal_zero_padding(reader)?;
        if reader.remaining_all_zero() {
            break;
        }
        let next = reader.peek_u8()?;
        if next != 0x15 && next != 0x18 {
            if vars.is_empty() {
                bail!("未对齐到 Normal 变量表");
            }
            break;
        }
        let tag = reader.read_u8()?;
        if tag == 0x18 {
            vars.extend(read_normal_group_variables(reader)?);
            continue;
        }
        let var = read_variable_normal(reader);
        match var {
            Ok(v) => {
                vars.push(v);
                continue;
            }
            Err(err) => {
                if reader.remaining_len() == 0 || reader.remaining_all_zero() {
                    break;
                }
                return Err(err);
            }
        }
    }
    Ok(vars)
}

fn read_variables_safety(reader: &mut MfcReader) -> Result<Vec<Variable>> {
    let start = reader.position();
    let looks_like = looks_like_safety_var_table(reader);
    if let Ok(vars) = try_read_variables_safety(reader) {
        if !vars.is_empty() || looks_like {
            return Ok(vars);
        }
    }
    reader.seek_to(start)?;
    seek_to_safety_var_table(reader)?;
    try_read_variables_safety(reader)
}

fn try_read_variables_safety(reader: &mut MfcReader) -> Result<Vec<Variable>> {
    skip_safety_var_header(reader)?;
    if reader.remaining_len() < 4 {
        return Ok(Vec::new());
    }
    let count = reader.read_u32()? as usize;
    if count == 0 {
        return Ok(Vec::new());
    }
    if count > 2000 {
        bail!("变量数量异常: {}", count);
    }
    let mut vars = Vec::with_capacity(count);
    for _ in 0..count {
        skip_safety_zero_padding(reader)?;
        let type_id = reader.read_u8()?;
        match type_id {
            0x15 | 0x0A => vars.push(read_variable_safety(reader)?),
            0x18 => vars.extend(read_safety_group_variables(reader)?),
            _ => bail!("不支持的变量类型: 0x{:02X}", type_id),
        }
    }
    Ok(vars)
}

fn skip_safety_zero_padding(reader: &mut MfcReader) -> Result<()> {
    while reader.remaining_len() > 0 && reader.peek_u8()? == 0x00 {
        let _ = reader.read_u8()?;
    }
    Ok(())
}

fn skip_safety_var_header(reader: &mut MfcReader) -> Result<()> {
    if reader.remaining_len() < 4 {
        return Ok(());
    }
    let buf = reader.remaining_slice();
    if buf.len() >= 4 && buf[0] == 0x00 && buf[1] == 0x02 && buf[2] == 0x41 && buf[3] == 0x78 {
        let _ = reader.read_bytes(4)?;
        let name_start = reader.position();
        if let Ok(_) = reader.read_mfc_string() {
            if reader.remaining_len() >= 4 {
                let count = reader.peek_u32()? as usize;
                if count > 2000 && reader.remaining_len() >= 5 {
                    let extra_pos = reader.position();
                    let _ = reader.read_u8()?;
                    let count2 = reader.peek_u32()? as usize;
                    if count2 > 2000 {
                        reader.seek_to(extra_pos)?;
                    }
                }
            }
        } else {
            reader.seek_to(name_start)?;
        }
    }
    Ok(())
}

fn read_safety_group_variables(reader: &mut MfcReader) -> Result<Vec<Variable>> {
    let parent = read_variable_safety(reader)?;
    let prefix = parent.name.clone();
    let input_count = reader.read_u32()? as usize;
    let mut vars = read_safety_group_members(reader, input_count, &prefix)?;
    let output_count = reader.read_u32()? as usize;
    vars.extend(read_safety_group_members(reader, output_count, &prefix)?);
    skip_safety_group_extra(reader)?;
    Ok(vars)
}

fn read_safety_group_members(
    reader: &mut MfcReader,
    count: usize,
    prefix: &str,
) -> Result<Vec<Variable>> {
    if count > 2000 {
        bail!("成员变量数量异常: {}", count);
    }
    let mut vars = Vec::with_capacity(count);
    for _ in 0..count {
        let type_id = reader.read_u8()?;
        match type_id {
            0x15 | 0x0A => {
                let mut var = read_variable_safety(reader)?;
                var.name = format!("{}.{}", prefix, var.name);
                vars.push(var);
            }
            _ => bail!("不支持的成员变量类型: 0x{:02X}", type_id),
        }
    }
    Ok(vars)
}

fn skip_safety_group_extra(reader: &mut MfcReader) -> Result<()> {
    let slice = reader.remaining_slice();
    if slice.len() < 9 {
        return Ok(());
    }
    let count_a = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as usize;
    let count_b = u32::from_le_bytes([slice[4], slice[5], slice[6], slice[7]]) as usize;
    let next_type = slice[8];
    if count_a > 2000 || count_b > 2000 {
        return Ok(());
    }
    if next_type != 0x18 && next_type != 0x15 && next_type != 0x0A {
        return Ok(());
    }
    let _ = reader.read_u32()?;
    let extra_count = reader.read_u32()? as usize;
    if extra_count > 2000 {
        bail!("扩展变量数量异常: {}", extra_count);
    }
    for _ in 0..extra_count {
        let type_id = reader.read_u8()?;
        match type_id {
            0x18 => skip_safety_group(reader)?,
            0x15 | 0x0A => {
                let _ = read_variable_safety(reader)?;
            }
            _ => bail!("不支持的扩展变量类型: 0x{:02X}", type_id),
        }
    }
    Ok(())
}

fn skip_safety_group(reader: &mut MfcReader) -> Result<()> {
    let _ = read_variable_safety(reader)?;
    let input_count = reader.read_u32()? as usize;
    skip_safety_group_members(reader, input_count)?;
    let output_count = reader.read_u32()? as usize;
    skip_safety_group_members(reader, output_count)?;
    skip_safety_group_extra(reader)?;
    Ok(())
}

fn skip_safety_group_members(reader: &mut MfcReader, count: usize) -> Result<()> {
    if count > 2000 {
        bail!("成员变量数量异常: {}", count);
    }
    for _ in 0..count {
        let type_id = reader.read_u8()?;
        match type_id {
            0x15 | 0x0A => {
                let _ = read_variable_safety(reader)?;
            }
            _ => bail!("不支持的成员变量类型: 0x{:02X}", type_id),
        }
    }
    Ok(())
}

fn try_read_normal_var_header(reader: &mut MfcReader) -> Result<Option<usize>> {
    let start = reader.position();
    let name = match reader.read_mfc_string() {
        Ok(s) => s,
        Err(_) => {
            reader.seek_to(start)?;
            return Ok(None);
        }
    };
    if name.is_empty() || !name.is_ascii() {
        reader.seek_to(start)?;
        return Ok(None);
    }
    if reader.remaining_len() == 0 {
        reader.seek_to(start)?;
        return Ok(None);
    }
    if reader.peek_u8()? == 0x00 {
        let _ = reader.read_u8()?;
    }
    if reader.remaining_len() < 4 {
        reader.seek_to(start)?;
        return Ok(None);
    }
    let count = reader.read_u32()? as usize;
    if count == 0 || count > 2000 {
        reader.seek_to(start)?;
        return Ok(None);
    }
    if reader.remaining_len() == 0 {
        reader.seek_to(start)?;
        return Ok(None);
    }
    let next = reader.peek_u8()?;
    if next != 0x18 && next != 0x15 {
        reader.seek_to(start)?;
        return Ok(None);
    }
    Ok(Some(count))
}

fn skip_normal_zero_padding(reader: &mut MfcReader) -> Result<()> {
    while reader.remaining_len() > 0 && reader.peek_u8()? == 0x00 {
        let _ = reader.read_u8()?;
    }
    Ok(())
}

fn read_normal_group_variables(reader: &mut MfcReader) -> Result<Vec<Variable>> {
    let parent = read_variable_normal(reader)?;
    let prefix = parent.name.clone();
    let input_count = reader.read_u32()? as usize;
    let mut vars = read_normal_group_members(reader, input_count, &prefix)?;
    let output_count = reader.read_u32()? as usize;
    vars.extend(read_normal_group_members(reader, output_count, &prefix)?);
    Ok(vars)
}

fn read_normal_group_members(
    reader: &mut MfcReader,
    count: usize,
    prefix: &str,
) -> Result<Vec<Variable>> {
    if count > 2000 {
        bail!("成员变量数量异常: {}", count);
    }
    let mut vars = Vec::with_capacity(count);
    for _ in 0..count {
        let type_id = reader.read_u8()?;
        if type_id != 0x15 {
            bail!("不支持的成员变量类型: 0x{:02X}", type_id);
        }
        let mut var = read_variable_normal(reader)?;
        var.name = format!("{}.{}", prefix, var.name);
        vars.push(var);
    }
    Ok(vars)
}

fn looks_like_safety_var_table(reader: &MfcReader) -> bool {
    let buf = reader.remaining_slice();
    if buf.len() < 6 {
        return false;
    }
    if buf.starts_with(&[0x00, 0x02, 0x41, 0x78]) {
        return true;
    }
    let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if count == 0 || count > 2000 {
        return false;
    }
    let type_id = buf[4];
    if type_id != 0x15 && type_id != 0x0A {
        return false;
    }
    let name_len = buf[5] as usize;
    if name_len == 0 || name_len > 80 || 6 + name_len > buf.len() {
        return false;
    }
    buf[6..6 + name_len].iter().all(|b| b.is_ascii_graphic())
}

fn looks_like_safety_var_table_ahead(reader: &MfcReader, window: usize) -> bool {
    let buf = reader.remaining_slice();
    if buf.len() < 4 {
        return false;
    }
    let max = window.min(buf.len().saturating_sub(4));
    for offset in 0..=max {
        if buf[offset..].starts_with(&[0x00, 0x02, 0x41, 0x78]) {
            return true;
        }
    }
    false
}

fn find_safety_var_table_offset(reader: &MfcReader) -> Option<usize> {
    let buf = reader.remaining_slice();
    if buf.len() < 4 {
        return None;
    }
    for offset in 0..=buf.len().saturating_sub(4) {
        if buf[offset..].starts_with(&[0x00, 0x02, 0x41, 0x78]) {
            return Some(offset);
        }
    }
    None
}

fn find_class_sig_ahead(reader: &MfcReader, window: usize) -> Option<usize> {
    let buf = reader.remaining_slice();
    if buf.len() < 6 {
        return None;
    }
    let max = window.min(buf.len().saturating_sub(6));
    for offset in 0..=max {
        if buf[offset] != 0xFF || buf[offset + 1] != 0xFF {
            continue;
        }
        let name_len = u16::from_le_bytes([buf[offset + 4], buf[offset + 5]]) as usize;
        if name_len == 0 || name_len > 64 {
            continue;
        }
        if offset + 6 + name_len > buf.len() {
            continue;
        }
        let name_bytes = &buf[offset + 6..offset + 6 + name_len];
        if !name_bytes.starts_with(b"CLD") {
            continue;
        }
        if !name_bytes.iter().all(|b| b.is_ascii_graphic()) {
            continue;
        }
        return Some(offset);
    }
    None
}

fn seek_to_safety_var_table(reader: &mut MfcReader) -> Result<()> {
    let start = reader.position();
    let buf = reader.inner.get_ref();
    if start >= buf.len() {
        bail!("已到达文件末尾，无法寻找变量表");
    }
    let slice = &buf[start..];
    for offset in 0..slice.len().saturating_sub(6) {
        let count = u32::from_le_bytes([
            slice[offset],
            slice[offset + 1],
            slice[offset + 2],
            slice[offset + 3],
        ]) as usize;
        if count == 0 || count > 2000 {
            continue;
        }
        let type_id = slice[offset + 4];
        if type_id != 0x15 && type_id != 0x0A {
            continue;
        }
        let name_len = slice[offset + 5] as usize;
        if name_len == 0 || name_len > 80 {
            continue;
        }
        if offset + 6 + name_len > slice.len() {
            continue;
        }
        let name_bytes = &slice[offset + 6..offset + 6 + name_len];
        if !name_bytes.iter().all(|b| b.is_ascii_graphic()) {
            continue;
        }
        reader.seek_to(start + offset)?;
        return Ok(());
    }
    bail!("未找到 Safety 变量表起点");
}

fn find_normal_var_table_offset(reader: &MfcReader) -> Option<usize> {
    let buf = reader.remaining_slice();
    if buf.len() < 8 {
        return None;
    }
    for offset in 0..buf.len().saturating_sub(8) {
        let mut idx = offset;
        if !scan_mfc_string_ascii(buf, &mut idx, 80).unwrap_or(false) {
            continue;
        }
        if idx < buf.len() && buf[idx] == 0x00 {
            idx += 1;
        }
        if idx + 4 > buf.len() {
            continue;
        }
        let count = u32::from_le_bytes([buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]) as usize;
        idx += 4;
        if count == 0 || count > 2000 {
            continue;
        }
        if idx >= buf.len() {
            continue;
        }
        let tag = buf[idx];
        if tag != 0x18 && tag != 0x15 {
            continue;
        }
        return Some(idx);
    }
    None
}

fn seek_to_normal_var_table(reader: &mut MfcReader) -> Result<()> {
    let start = reader.position();
    let buf = reader.inner.get_ref();
    if start >= buf.len() {
        bail!("已到达文件末尾，无法寻找变量表");
    }
    let slice = &buf[start..];
    let probe = MfcReader::new(slice);
    if let Some(offset) = find_normal_var_table_offset(&probe) {
        reader.seek_to(start + offset)?;
        return Ok(());
    }
    bail!("未找到 Normal 变量表起点");
}

fn scan_mfc_string_any(buf: &[u8], idx: &mut usize, max_len: usize) -> Result<bool> {
    if *idx >= buf.len() {
        return Ok(false);
    }
    let len_u8 = buf[*idx] as usize;
    *idx += 1;
    let len = if len_u8 == 0xFF {
        if *idx + 2 > buf.len() {
            return Ok(false);
        }
        let len = u16::from_le_bytes([buf[*idx], buf[*idx + 1]]) as usize;
        *idx += 2;
        len
    } else {
        len_u8
    };
    if len > max_len || *idx + len > buf.len() {
        return Ok(false);
    }
    *idx += len;
    Ok(true)
}

fn scan_mfc_string_ascii(buf: &[u8], idx: &mut usize, max_len: usize) -> Result<bool> {
    if *idx >= buf.len() {
        return Ok(false);
    }
    let len_u8 = buf[*idx] as usize;
    *idx += 1;
    let len = if len_u8 == 0xFF {
        if *idx + 2 > buf.len() {
            return Ok(false);
        }
        let len = u16::from_le_bytes([buf[*idx], buf[*idx + 1]]) as usize;
        *idx += 2;
        len
    } else {
        len_u8
    };
    if len > max_len || *idx + len > buf.len() {
        return Ok(false);
    }
    let s = &buf[*idx..*idx + len];
    *idx += len;
    Ok(s.iter().all(|b| b.is_ascii_graphic()))
}

fn read_variable_normal(reader: &mut MfcReader) -> Result<Variable> {
    let name = reader.read_mfc_string()?;
    let _ = reader.read_mfc_string()?; // name2
    let comment = reader.read_mfc_string()?;
    let data_type = reader.read_mfc_string()?;
    let _init_flag = reader.read_u8()?;
    let init_value = reader.read_mfc_string()?;
    let retain_flag = reader.read_u8()?;
    let addr_id = reader.read_u64()?;
    let _ = reader.read_mfc_string()?; // extra_str
    let mode = reader.read_u8()?;
    let var_id = reader.read_u16()?;
    let _retain_mirror = reader.read_u8()?;
    let id2 = reader.read_u32()?;
    let soe = reader.read_u16()?;

    Ok(Variable {
        name,
        data_type,
        init_value,
        soe_enable: soe != 0,
        power_down_keep: retain_flag == 0x03,
        comment,
        var_id: Some(var_id),
        addr_id: Some(addr_id),
        mode: Some(mode),
        id2: Some(id2),
        area_code: None,
    })
}

fn read_variable_safety(reader: &mut MfcReader) -> Result<Variable> {
    let name = reader.read_mfc_string()?;
    let _ = reader.read_mfc_string()?; // name2
    let lang_count = reader.read_u32()?;
    let mut comment = String::new();
    for _ in 0..lang_count {
        let _ = reader.read_mfc_string()?; // lang
        let cmt = reader.read_mfc_string()?;
        if comment.is_empty() {
            comment = cmt;
        }
    }
    let data_type = reader.read_mfc_string()?;
    let _init_flag = reader.read_u8()?;
    let init_value = reader.read_mfc_string()?;
    let area_code = reader.read_u8()?;
    let _ = reader.read_u16()?; // flag_78
    let addr_id = reader.read_u32()? as u64;
    let _ = reader.read_mfc_string()?; // extra_str
    let mode = reader.read_u8()?;
    let var_id = reader.read_u16()?;
    let soe_low = reader.read_u8()?;
    let soe_high = reader.read_u8()?;
    let soe = ((soe_high as u16) << 8) | soe_low as u16;

    Ok(Variable {
        name,
        data_type,
        init_value,
        soe_enable: soe != 0,
        power_down_keep: false,
        comment,
        var_id: Some(var_id),
        addr_id: Some(addr_id),
        mode: Some(mode),
        id2: None,
        area_code: Some(area_code),
    })
}

fn element_type_from_id(variant: Variant, type_id: u8) -> Result<ElementType> {
    let ty = match (variant, type_id) {
        (Variant::Normal, 0x05) => ElementType::Contact,
        (Variant::Safety, 0x04) => ElementType::Contact,
        (Variant::Normal, 0x06) => ElementType::Coil,
        (Variant::Safety, 0x05) => ElementType::Coil,
        (Variant::Normal, 0x09) => ElementType::Assign,
        (Variant::Safety, 0x08) => ElementType::Assign,
        (Variant::Safety, 0x09) => ElementType::Network,
        (_, 0x03) => ElementType::Box,
        _ => bail!("未知的 type_id: 0x{:02X}", type_id),
    };
    Ok(ty)
}

fn element_type_id_from_element(variant: Variant, elem_type: ElementType) -> Result<u8> {
    let type_id = match (variant, elem_type) {
        (Variant::Normal, ElementType::Contact) => 0x05,
        (Variant::Safety, ElementType::Contact) => 0x04,
        (Variant::Normal, ElementType::Coil) => 0x06,
        (Variant::Safety, ElementType::Coil) => 0x05,
        (Variant::Normal, ElementType::Assign) => 0x09,
        (Variant::Safety, ElementType::Assign) => 0x08,
        (_, ElementType::Box) => 0x03,
        _ => bail!("未知的元素类型: {:?}", elem_type),
    };
    Ok(type_id)
}

fn checked_i32(value: u32, field: &str) -> Result<i32> {
    if value > i32::MAX as u32 {
        bail!("{} 超出 i32 范围: {}", field, value);
    }
    Ok(value as i32)
}

/// 解析入口：读取 Hollysys 剪贴板数据并输出通用 POU
pub fn read_pou(data: &[u8], variant: Variant) -> Result<UniversalPou> {
    read_pou_with_config(data, variant, DEFAULT_SERIALIZE_VERSION)
}

/// 解析入口（带序列化版本配置）
pub fn read_pou_with_config(data: &[u8], variant: Variant, serialize_version: u32) -> Result<UniversalPou> {
    let mut reader = MfcReader::new(data);
    let name = read_header(&mut reader, variant)?;
    let header_strings = if variant == Variant::Safety {
        read_string_array(&mut reader)?
    } else {
        Vec::new()
    };
    let (variables, networks) = if variant == Variant::Safety {
        let nets = read_networks(&mut reader, variant, serialize_version)?;
        let vars = read_variables(&mut reader, variant)?;
        (vars, nets)
    } else {
        let nets = read_networks(&mut reader, variant, serialize_version)?;
        let vars = read_variables(&mut reader, variant)?;
        (vars, nets)
    };
    let symbol_lookup = load_symbol_lookup();
    let variable_nodes = organize_variables(variables, &header_strings, &symbol_lookup);
    Ok(UniversalPou {
        name,
        header_strings,
        variables: variable_nodes,
        networks,
    })
}

fn load_symbol_lookup() -> HashMap<String, HashSet<String>> {
    let mut candidates = Vec::new();
    candidates.push(Path::new(DEFAULT_SYMBOL_CONFIG_PATH).to_path_buf());
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest_dir.join("..").join(DEFAULT_SYMBOL_CONFIG_PATH));

    for path in candidates {
        match SymbolConfig::load_from_file(&path) {
            Ok(config) => {
                let map = config.to_lookup_map();
                if !map.is_empty() {
                    return map;
                }
            }
            Err(err) => {
                warn!("符号表配置加载失败: {} ({})", path.display(), err);
            }
        }
    }

    HashMap::new()
}

fn organize_variables(
    flat_vars: Vec<Variable>,
    header_strings: &[String],
    symbol_lookup: &HashMap<String, HashSet<String>>,
) -> Vec<VariableNode> {
    let mut root_nodes = Vec::new();
    let mut processed_indices = HashSet::new();

    let mut matched_type = None;
    for header in header_strings {
        if symbol_lookup.contains_key(header) {
            matched_type = Some(header.clone());
            break;
        }
    }

    if let Some(type_name) = matched_type {
        if let Some(members) = symbol_lookup.get(&type_name) {
            let mut group_children = Vec::new();
            for (idx, var) in flat_vars.iter().enumerate() {
                if var.name.contains('.') {
                    continue;
                }
                let base = var.name.rsplit('.').next().unwrap_or(var.name.as_str());
                if members.contains(base) {
                    group_children.push(VariableNode::Leaf(var.clone()));
                    processed_indices.insert(idx);
                }
            }

            if !group_children.is_empty() {
                root_nodes.push(VariableNode::Group {
                    name: type_name.clone(),
                    type_name: Some(type_name),
                    children: group_children,
                });
            }
        }
    }

    let mut dot_groups: HashMap<String, Vec<VariableNode>> = HashMap::new();
    let mut remaining_vars = Vec::new();

    for (idx, mut var) in flat_vars.into_iter().enumerate() {
        if processed_indices.contains(&idx) {
            continue;
        }
        let name = var.name.clone();
        if let Some((prefix, suffix)) = name.split_once('.') {
            var.name = suffix.to_string();
            dot_groups
                .entry(prefix.to_string())
                .or_default()
                .push(VariableNode::Leaf(var));
        } else {
            remaining_vars.push(VariableNode::Leaf(var));
        }
    }

    let mut dot_keys: Vec<String> = dot_groups.keys().cloned().collect();
    dot_keys.sort();
    for key in dot_keys {
        if let Some(children) = dot_groups.remove(&key) {
            root_nodes.push(VariableNode::Group {
                name: key,
                type_name: None,
                children,
            });
        }
    }

    if !remaining_vars.is_empty() {
        root_nodes.push(VariableNode::Group {
            name: "Local Variables".to_string(),
            type_name: None,
            children: remaining_vars,
        });
    }

    root_nodes
}

fn read_string_array(reader: &mut MfcReader) -> Result<Vec<String>> {
    let start = reader.position();
    if reader.position() % 2 != 0 {
        let _ = reader.read_u8()?;
    }
    let count = reader.read_u16()? as usize;
    if count > 1000 {
        reader.seek_to(start)?;
        let _ = reader.read_u32()?;
        if reader.position() % 2 != 0 {
            let _ = reader.read_u8()?;
        }
        let count = reader.read_u16()? as usize;
        return read_string_array_items(reader, count);
    }
    read_string_array_items(reader, count)
}

fn read_string_array_items(reader: &mut MfcReader, count: usize) -> Result<Vec<String>> {
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(reader.read_mfc_string()?);
    }
    Ok(items)
}

/// 读取触点（CLDContact）二进制结构（供调试或单元测试使用）。
pub fn read_contact_bin<R: Read + Seek>(
    reader: &mut R,
    variant: Variant,
) -> BinResult<CLDContact> {
    CLDContact::read_options(reader, Endian::Little, (variant,))
}

/// 读取线圈（CLDOutput）二进制结构（供调试或单元测试使用）。
pub fn read_output_bin<R: Read + Seek>(
    reader: &mut R,
    variant: Variant,
    serialize_version: u32,
) -> BinResult<CLDOutput> {
    CLDOutput::read_options(reader, Endian::Little, (variant, serialize_version))
}

/// 读取功能块（CLDBox）二进制结构（供调试或单元测试使用）。
pub fn read_box_bin<R: Read + Seek>(
    reader: &mut R,
    variant: Variant,
    serialize_version: u32,
) -> BinResult<CLDBox> {
    CLDBox::read_options(reader, Endian::Little, (variant, serialize_version))
}
