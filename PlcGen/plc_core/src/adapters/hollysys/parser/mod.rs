#![allow(dead_code)]

mod mfc;
mod object_stream;
mod safety;
mod variables;

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};
use std::path::Path;

use anyhow::{Context, Result, bail};
use binrw::{binread, BinRead, BinResult, Endian};
use log::warn;

pub(crate) use super::protocol::PlcVariant as Variant;
use crate::ast::{BoxPin, ElementType, LdElement, Network, PinDirection, UniversalPou, Variable, VariableNode};
use crate::symbols_config::SymbolConfig;

use mfc::{MfcReader, MfcString};
use object_stream::{ClassTable, ObjectKind, prefill_class_table, read_object_kind};
use safety::read_networks_safety;
use variables::{find_normal_var_table_offset, looks_like_safety_var_table, read_variables};

/// 引脚序列化的两种形态：
/// - Compact：仅 Name/Var（Safety 常见）。
/// - StandardInput：u8,u8 + Name + Var + u32（Normal 输入）。
/// - StandardOutput：u8,u8 + Name + Var（Normal 输出）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinFormat {
    Compact,
    StandardInput,
    StandardOutput,
}

impl PinFormat {
    /// 输入引脚：Normal 使用标准格式；Safety 使用紧凑格式。
    pub fn for_input(variant: Variant) -> Self {
        match variant {
            Variant::Normal => PinFormat::StandardInput,
            Variant::Safety => PinFormat::Compact,
        }
    }

    /// 输出引脚：Normal 使用标准格式；Safety 使用紧凑格式。
    pub fn for_output(variant: Variant) -> Self {
        match variant {
            Variant::Normal => PinFormat::StandardOutput,
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
    #[br(if(matches!(format, PinFormat::StandardInput | PinFormat::StandardOutput)))]
    pub flag0: Option<u8>,
    #[br(if(matches!(format, PinFormat::StandardInput | PinFormat::StandardOutput)))]
    pub flag1: Option<u8>,
    pub name: MfcString,
    pub var: MfcString,
    #[br(if(matches!(format, PinFormat::StandardInput)))]
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

/// 默认序列化版本：与当前版本输出保持一致。
pub const DEFAULT_SERIALIZE_VERSION: u32 = 13;
const DEFAULT_SYMBOL_CONFIG_PATH: &str = "config/symbols_config.json";

fn read_element_string(reader: &mut MfcReader, variant: Variant, max_len: usize) -> Result<String> {
    let _ = max_len;
    let _ = variant;
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

/// 读取 POU 头部，返回 POU 名称
fn read_header(reader: &mut MfcReader, variant: Variant, serialize_version: u32) -> Result<String> {
    let start = reader.position();
    if variant == Variant::Safety {
        if let Ok(name) = read_header_variant_b(reader, serialize_version) {
            return Ok(name);
        }
    }
    reader.seek_to(start)?;
    read_header_legacy(reader, variant)
}

fn read_header_variant_b(reader: &mut MfcReader, serialize_version: u32) -> Result<String> {
    let start = reader.position();
    if serialize_version >= 0x0F {
        return read_header_variant_b_inner(reader, serialize_version, true);
    }
    if let Ok(name) = read_header_variant_b_inner(reader, serialize_version, false) {
        return Ok(name);
    }
    reader.seek_to(start)?;
    read_header_variant_b_inner(reader, serialize_version, true)
}

fn read_header_variant_b_inner(
    reader: &mut MfcReader,
    serialize_version: u32,
    with_seed: bool,
) -> Result<String> {
    if with_seed {
        let _ = reader.read_u32()?;
    }
    let name = reader.read_mfc_string()?;
    let _ = reader.read_mfc_string()?;
    let _ = reader.read_u8()?;
    let _ = reader.read_u8()?;
    let _ = reader.read_u32()?;
    let _ = reader.read_u32()?;
    let _ = reader.read_u32()?;
    let _ = reader.read_u32()?;
    let _ = reader.read_mfc_string()?;
    let _ = reader.read_u8()?;
    let _ = reader.read_u32()?;
    let _ = reader.read_u32()?;
    if serialize_version >= 0x44 {
        let _ = reader.read_mfc_string()?;
    }

    if reader.remaining_len() < 4 {
        bail!("header truncated before string array");
    }
    let count = reader.peek_u32()? as usize;
    if count > 5000 {
        bail!("string array count too large: {}", count);
    }
    Ok(name)
}

fn read_header_legacy(reader: &mut MfcReader, variant: Variant) -> Result<String> {
    let name = reader.read_mfc_string()?;
    reader.align_to_4bytes()?;

    if variant == Variant::Normal {
        let _ = reader.read_u32()?;
    }

    let _ = reader.read_mfc_string()?;
    reader.align_to_4bytes()?;

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

    let _ = reader.read_u32()?;
    let _ = reader.read_mfc_string()?;
    let _ = reader.read_u32()?;
    let _ = reader.read_u32()?;
    match variant {
        Variant::Normal => {
            let _ = reader.read_mfc_string()?;
            let _ = reader.read_mfc_string()?;
        }
        Variant::Safety => {
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
        }
    }

    Ok(name)
}

fn read_network(reader: &mut MfcReader, variant: Variant) -> Result<Network> {
    let (id, _type_id, _name, _comment, _desc, _connections) = read_element_base(reader, variant)?;
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
    let sub_type = reader.read_u8()?;
    let _ = reader.read_u8()?; // flag2
    if variant == Variant::Normal && serialize_version > 0 {
        let _ = reader.read_u8()?; // flag3
    }
    if variant == Variant::Normal {
        let _ = reader.read_mfc_string()?; // geo/附加字段
    }
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
    serialize_version: u32,
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
            if serialize_version >= 13 && direction == PinDirection::Input {
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
    let _ = reader.read_u8()?; // flag
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
            let _ = reader.read_u8()?; // flag
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
            let sub_type = reader.read_u8()?;
            let _ = reader.read_u8()?; // flag2
            if variant == Variant::Normal && serialize_version > 0 {
                let _ = reader.read_u8()?; // flag3
            }
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
        _ => {
            bail!("动态元素类型不支持: {:?}", type_code);
        }
    }
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
    prefill_class_table(&mut class_table, reader.inner.get_ref(), reader.inner.get_ref().len());

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
        let object_kind = read_object_kind(reader, &mut class_table)?;
        if let Some(rem) = remaining.as_mut() {
            *rem = rem.saturating_sub(1);
        }

        match object_kind {
            ObjectKind::Null => {}
            ObjectKind::Reference(_) => {}
            ObjectKind::New(class_name) => match class_name.as_str() {
                "CLDNetwork" => {
                    if let Some(net) = current.take() {
                        networks.push(net);
                    }
                    let net = read_network(reader, variant)?;
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
                    let _ = read_element_base(reader, variant)?;
                }
                "CLDElement" | "CLDOr" | "CLDJump" | "CLDReturn" | "CLDBranches" => {
                    let _ = read_element_base(reader, variant)?;
                }
                "CLDBracket" => {
                    let _ = read_element_base(reader, variant)?;
                    skip_network_tail(reader)?;
                }
                _ => {
                    skip_network_tail(reader)?;
                }
            },
            ObjectKind::UnknownClass(_) => {
                let mut parsed_any = false;
                loop {
                    if let Some(type_id) = peek_element_type_id(reader) {
                        if !is_element_type_id(variant, type_id) {
                            break;
                        }
                        let elem = read_element_dynamic(reader, variant, serialize_version)?;
                        parsed_any = true;
                        if let Some(net) = current.as_mut() {
                            net.elements.push(elem);
                        } else {
                            bail!("元素出现在网络之前: UnknownClass");
                        }
                    } else {
                        break;
                    }
                    if looks_like_object_tag(reader) || looks_like_safety_var_table(reader) {
                        break;
                    }
                    if !looks_like_element_object(reader, variant) {
                        break;
                    }
                }
                if !parsed_any {
                    skip_network_tail(reader)?;
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

fn skip_network_tail(reader: &mut MfcReader) -> Result<()> {
    while reader.remaining_len() > 0 {
        if looks_like_object_tag(reader) || looks_like_safety_var_table(reader) {
            break;
        }
        let _ = reader.read_u8()?;
    }
    Ok(())
}

pub(crate) struct NetworkListStart {
    count: Option<usize>,
    count_len: usize,
    offset: usize,
}

pub(crate) fn seek_to_network_list_start(reader: &mut MfcReader) -> Result<NetworkListStart> {
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

pub(crate) fn looks_like_object_tag(reader: &MfcReader) -> bool {
    let buf = reader.remaining_slice();
    if buf.len() < 2 {
        return false;
    }
    let tag = u16::from_le_bytes([buf[0], buf[1]]);
    tag == 0x0000 || tag == 0xFFFF || tag == 0x7FFF || (tag & 0x8000 != 0)
}

fn is_element_type_id(variant: Variant, type_id: u8) -> bool {
    match variant {
        Variant::Normal => matches!(type_id, 0x03 | 0x05 | 0x06),
        Variant::Safety => type_id <= 0x09,
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
    let type_id = buf[4];
    if !is_element_type_id(variant, type_id) {
        return false;
    }
    let mut idx = 5usize;
    if !mfc::scan_mfc_string_any(buf, &mut idx, 120).unwrap_or(false) {
        return false;
    }
    if variant == Variant::Normal {
        if !mfc::scan_mfc_string_any(buf, &mut idx, 160).unwrap_or(false) {
            return false;
        }
        if !mfc::scan_mfc_string_any(buf, &mut idx, 200).unwrap_or(false) {
            return false;
        }
    }
    true
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
    let name = read_header(&mut reader, variant, serialize_version)?;
    let header_strings = if variant == Variant::Safety {
        read_string_array(&mut reader)?
    } else {
        Vec::new()
    };
    let (variables, networks) = if variant == Variant::Safety {
        let nets = read_networks(&mut reader, variant, serialize_version)
            .with_context(|| "读取 Safety networks 失败")?;
        let vars = read_variables(&mut reader, variant, serialize_version)
            .with_context(|| "读取 Safety variables 失败")?;
        (vars, nets)
    } else {
        let nets = read_networks(&mut reader, variant, serialize_version)
            .with_context(|| "读取 Normal networks 失败")?;
        let vars = read_variables(&mut reader, variant, serialize_version)
            .with_context(|| "读取 Normal variables 失败")?;
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
    let count = reader.read_u32()? as usize;
    if count > 5000 {
        reader.seek_to(start)?;
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
