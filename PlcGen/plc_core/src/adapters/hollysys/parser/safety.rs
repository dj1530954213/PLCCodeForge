use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{Result, bail};

use super::mfc::MfcReader;
use super::object_stream::{ClassTable, ObjectKind, prefill_class_table, read_object_kind};
use super::variables::{find_safety_var_table_offset, looks_like_safety_var_table, looks_like_safety_var_table_ahead, SAFETY_VAR_MAX};
use super::{
    checked_i32, element_type_from_id, looks_like_object_tag, read_element_base,
    read_element_fields, read_element_string, read_pin, ElementType, LdElement, Network,
    PinDirection, Variant,
};
use crate::ast::SafetyTopologyToken;

#[derive(Debug, Clone)]
struct SafetyNode {
    elem: LdElement,
    children: Vec<SafetyNode>,
    label: Option<String>,
    comment: Option<String>,
}

fn read_safety_string(reader: &mut MfcReader, _max_len: usize) -> Result<String> {
    reader.read_mfc_string()
}

fn read_safety_string_optional(reader: &mut MfcReader, _max_len: usize) -> Result<Option<String>> {
    if reader.remaining_len() == 0 {
        return Ok(None);
    }
    Ok(Some(reader.read_mfc_string()?))
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
) -> Result<(i32, u8, String, Vec<i32>)> {
    let id_u32 = reader.read_u32()?;
    let id = checked_i32(id_u32, "safety.element.id")?;
    let type_id = reader.read_u8()?;
    if let Some(expect) = expected_type {
        if expect != type_id {
            bail!(
                "safety element type mismatch: expected=0x{:02X}, got=0x{:02X}",
                expect,
                type_id
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
    Ok((id, type_id, name, connections))
}

fn read_safety_pin(reader: &mut MfcReader, direction: PinDirection) -> Result<super::BoxPin> {
    let name = read_safety_string(reader, 80)?;
    let variable = read_safety_string(reader, 200)?;
    Ok(super::BoxPin { name, variable, direction })
}

fn read_safety_node(
    reader: &mut MfcReader,
    expected_type: Option<u8>,
    _serialize_version: u32,
) -> Result<SafetyNode> {
    let (id, type_id, name, connections) = read_safety_base(reader, expected_type)?;
    let children = Vec::new();

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
            instance = read_safety_string(reader, 200)?;
            let input_count = reader.read_u32()? as usize;
            if input_count > SAFETY_VAR_MAX {
                bail!("Safety Box 输入数量异常: {}", input_count);
            }
            for _ in 0..input_count {
                pins.push(read_safety_pin(reader, PinDirection::Input)?);
            }
            let output_count = reader.read_u32()? as usize;
            if output_count > SAFETY_VAR_MAX {
                bail!("Safety Box 输出数量异常: {}", output_count);
            }
            for _ in 0..output_count {
                pins.push(read_safety_pin(reader, PinDirection::Output)?);
            }
        }
        ElementType::Contact => {
            sub_type = reader.read_u8()?;
        }
        ElementType::Coil => {
            sub_type = reader.read_u8()?;
            let _ = reader.read_u8()?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SafetyTypeId {
    Element = 0x00,
    Or = 0x01,
    And = 0x02,
    Box = 0x03,
    Contact = 0x04,
    Output = 0x05,
    Return = 0x06,
    Jump = 0x07,
    Assign = 0x08,
    Network = 0x09,
}

#[derive(Debug, Clone)]
struct SafetyParsedObject {
    id: i32,
    type_id: u8,
    connections: Vec<i32>,
    element: Option<LdElement>,
    label: Option<String>,
    comment: Option<String>,
    order: usize,
}

fn safety_type_from_id(type_id: u8) -> Result<SafetyTypeId> {
    match type_id {
        0x00 => Ok(SafetyTypeId::Element),
        0x01 => Ok(SafetyTypeId::Or),
        0x02 => Ok(SafetyTypeId::And),
        0x03 => Ok(SafetyTypeId::Box),
        0x04 => Ok(SafetyTypeId::Contact),
        0x05 => Ok(SafetyTypeId::Output),
        0x06 => Ok(SafetyTypeId::Return),
        0x07 => Ok(SafetyTypeId::Jump),
        0x08 => Ok(SafetyTypeId::Assign),
        0x09 => Ok(SafetyTypeId::Network),
        _ => bail!("未知的 Safety TypeID: 0x{:02X}", type_id),
    }
}

fn read_safety_object_body(reader: &mut MfcReader, serialize_version: u32) -> Result<SafetyParsedObject> {
    let (id, type_id, name, _comment, _desc, connections) =
        read_element_base(reader, Variant::Safety)?;
    let kind = safety_type_from_id(type_id)?;

    let mut element = None;
    let mut label = None;
    let mut comment = None;

    match kind {
        SafetyTypeId::Network => {
            label = Some(reader.read_mfc_string()?);
            comment = Some(reader.read_mfc_string()?);
        }
        SafetyTypeId::Box => {
            let _ = reader.read_u8()?; // box_flag
            let instance = read_element_string(reader, Variant::Safety, 200)?;
            let input_count = reader.read_u32()? as usize;
            if input_count > SAFETY_VAR_MAX {
                bail!("Safety Box 输入数量异常: {}", input_count);
            }
            let mut pins = Vec::new();
            for _ in 0..input_count {
                pins.push(read_pin(reader, Variant::Safety, serialize_version, PinDirection::Input)?);
            }
            let output_count = reader.read_u32()? as usize;
            if output_count > SAFETY_VAR_MAX {
                bail!("Safety Box 输出数量异常: {}", output_count);
            }
            for _ in 0..output_count {
                pins.push(read_pin(reader, Variant::Safety, serialize_version, PinDirection::Output)?);
            }
            element = Some(LdElement {
                id,
                type_code: ElementType::Box,
                name,
                comment: String::new(),
                desc: String::new(),
                instance,
                pins,
                connections: connections.clone(),
                sub_type: 0,
            });
        }
        SafetyTypeId::Contact => {
            let sub_type = reader.read_u8()?;
            element = Some(LdElement {
                id,
                type_code: ElementType::Contact,
                name,
                comment: String::new(),
                desc: String::new(),
                instance: String::new(),
                pins: Vec::new(),
                connections: connections.clone(),
                sub_type,
            });
        }
        SafetyTypeId::Output => {
            let sub_type = reader.read_u8()?;
            let _ = reader.read_u8()?;
            element = Some(LdElement {
                id,
                type_code: ElementType::Coil,
                name,
                comment: String::new(),
                desc: String::new(),
                instance: String::new(),
                pins: Vec::new(),
                connections: connections.clone(),
                sub_type,
            });
        }
        SafetyTypeId::Assign => {
            element = Some(LdElement {
                id,
                type_code: ElementType::Assign,
                name,
                comment: String::new(),
                desc: String::new(),
                instance: String::new(),
                pins: Vec::new(),
                connections: connections.clone(),
                sub_type: 0,
            });
        }
        SafetyTypeId::Element
        | SafetyTypeId::Or
        | SafetyTypeId::And
        | SafetyTypeId::Return
        | SafetyTypeId::Jump => {}
    }

    Ok(SafetyParsedObject {
        id,
        type_id,
        connections,
        element,
        label,
        comment,
        order: 0,
    })
}

fn build_safety_networks(objects: Vec<SafetyParsedObject>) -> Vec<Network> {
    let mut id_map: HashMap<i32, usize> = HashMap::new();
    for (idx, obj) in objects.iter().enumerate() {
        id_map.insert(obj.id, idx);
    }

    let mut adj: HashMap<i32, Vec<i32>> = HashMap::new();
    for obj in &objects {
        for &conn in &obj.connections {
            if conn < 0 {
                continue;
            }
            if !id_map.contains_key(&conn) {
                continue;
            }
            adj.entry(obj.id).or_default().push(conn);
            adj.entry(conn).or_default().push(obj.id);
        }
    }

    let mut network_nodes: Vec<&SafetyParsedObject> =
        objects.iter().filter(|obj| obj.type_id == 0x09).collect();
    network_nodes.sort_by_key(|obj| obj.order);

    if network_nodes.is_empty() {
        let mut elements: Vec<&SafetyParsedObject> =
            objects.iter().filter(|obj| obj.element.is_some()).collect();
        elements.sort_by_key(|obj| obj.order);
        return vec![Network {
            id: 0,
            label: String::new(),
            comment: String::new(),
            elements: elements
                .into_iter()
                .filter_map(|obj| obj.element.clone())
                .collect(),
            safety_topology: Vec::new(),
        }];
    }

    let assignments = assign_safety_networks(&adj, &network_nodes);
    let mut elements_by_network: HashMap<i32, Vec<&SafetyParsedObject>> = HashMap::new();
    let mut orphan_elements: Vec<&SafetyParsedObject> = Vec::new();

    for obj in &objects {
        if obj.element.is_none() {
            continue;
        }
        if let Some((_, _, net_id)) = assignments.get(&obj.id) {
            elements_by_network.entry(*net_id).or_default().push(obj);
        } else {
            orphan_elements.push(obj);
        }
    }

    let mut networks = Vec::new();
    for net in &network_nodes {
        let mut elements = elements_by_network.remove(&net.id).unwrap_or_default();
        elements.sort_by_key(|obj| obj.order);
        networks.push(Network {
            id: net.id,
            label: net.label.clone().unwrap_or_default(),
            comment: net.comment.clone().unwrap_or_default(),
            elements: elements
                .into_iter()
                .filter_map(|obj| obj.element.clone())
                .collect(),
            safety_topology: Vec::new(),
        });
    }

    if !orphan_elements.is_empty() {
        orphan_elements.sort_by_key(|obj| obj.order);
        let mut orphan_elems: Vec<LdElement> = orphan_elements
            .into_iter()
            .filter_map(|obj| obj.element.clone())
            .collect();
        if let Some(net) = networks.iter_mut().find(|net| net.id == 0) {
            net.elements.append(&mut orphan_elems);
        } else {
            networks.push(Network {
                id: 0,
                label: String::new(),
                comment: String::new(),
                elements: orphan_elems,
                safety_topology: Vec::new(),
            });
        }
    }

    networks
}

fn assign_safety_networks(
    adj: &HashMap<i32, Vec<i32>>,
    network_nodes: &[&SafetyParsedObject],
) -> HashMap<i32, (usize, usize, i32)> {
    let mut assignments: HashMap<i32, (usize, usize, i32)> = HashMap::new();
    for (net_order, net) in network_nodes.iter().enumerate() {
        let mut queue: VecDeque<(i32, usize)> = VecDeque::new();
        let mut visited: HashSet<i32> = HashSet::new();
        queue.push_back((net.id, 0));
        visited.insert(net.id);

        while let Some((node, dist)) = queue.pop_front() {
            let entry = assignments.entry(node).or_insert((dist, net_order, net.id));
            if dist < entry.0 || (dist == entry.0 && net_order < entry.1) {
                *entry = (dist, net_order, net.id);
            }
            if let Some(neighbors) = adj.get(&node) {
                for &next in neighbors {
                    if visited.insert(next) {
                        queue.push_back((next, dist + 1));
                    }
                }
            }
        }
    }
    assignments
}

pub(crate) fn read_networks_safety(reader: &mut MfcReader, serialize_version: u32) -> Result<Vec<Network>> {
    read_networks_safety_class(reader, serialize_version)
}

fn read_networks_safety_class(reader: &mut MfcReader, serialize_version: u32) -> Result<Vec<Network>> {
    let list = super::seek_to_network_list_start(reader)?;
    let stop_at = find_safety_var_table_offset(reader).map(|offset| reader.position() + offset);
    let mut remaining = list.count;
    let mut objects: Vec<SafetyParsedObject> = Vec::new();
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
            ObjectKind::New(_) | ObjectKind::UnknownClass(_) => {
                let mut obj = read_safety_object_body(reader, serialize_version)?;
                obj.order = objects.len();
                objects.push(obj);
            }
        }

        if reader.position() == pos {
            break;
        }
    }

    if let Some(stop_at) = stop_at {
        if reader.position() < stop_at {
            reader.seek_to(stop_at)?;
        }
    }
    Ok(build_safety_networks(objects))
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
        if looks_like_object_tag(reader) {
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
            let _flag = reader.read_u8()?;
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
            let _ = reader.read_u8()?;
            if variant == Variant::Normal && serialize_version > 0 {
                let _ = reader.read_u8()?;
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
    let count = reader.read_u32()? as usize;
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        let id_u32 = reader.read_u32()?;
        let id = checked_i32(id_u32, "safety.rung.id")?;
        ids.push(id);
    }
    if reader.remaining_len() == 0 || looks_like_object_tag(reader) {
        return Ok((ids, None));
    }
    let topology = read_safety_topology_raw(reader, Variant::Safety, serialize_version)?;
    Ok((ids, Some(topology)))
}

fn read_safety_assign(reader: &mut MfcReader) -> Result<LdElement> {
    let (id, _type_id, name, _comment, _desc, connections) = read_element_base(reader, Variant::Safety)?;
    Ok(LdElement {
        id,
        type_code: ElementType::Assign,
        name,
        comment: String::new(),
        desc: String::new(),
        instance: String::new(),
        pins: Vec::new(),
        connections,
        sub_type: 0,
    })
}

fn resolve_safety_topology(raw: Vec<SafetyTopologyEntry>) -> Result<Vec<SafetyTopologyToken>> {
    let mut topology = Vec::new();
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
    let peek = reader.peek_u16()?;
    if peek == 0x0000 || peek == 0xFFFF {
        let _ = reader.read_u16()?;
        return Ok(true);
    }
    Ok(false)
}

fn skip_safety_box_padding(reader: &mut MfcReader) -> Result<()> {
    for _ in 0..4 {
        if !skip_safety_reserved_u16(reader)? {
            break;
        }
    }
    Ok(())
}

fn read_compact_element_header(reader: &mut MfcReader) -> Result<(u32, u16)> {
    let id = reader.read_u32()?;
    let type_id = reader.read_u16()?;
    Ok((id, type_id))
}
