use anyhow::{Context, Result, bail};

use super::mfc::{MfcReader, scan_mfc_string_ascii};
use super::Variant;
use crate::ast::Variable;

pub(crate) const SAFETY_VAR_MAX: usize = 2000;

pub(crate) fn read_variables(
    reader: &mut MfcReader,
    variant: Variant,
    serialize_version: u32,
) -> Result<Vec<Variable>> {
    match variant {
        Variant::Normal => read_variables_normal(reader),
        Variant::Safety => read_variables_safety(reader, serialize_version),
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

fn read_variables_safety(reader: &mut MfcReader, serialize_version: u32) -> Result<Vec<Variable>> {
    let start = reader.position();
    let looks_like = looks_like_safety_var_table(reader);
    if let Ok(vars) = try_read_variables_safety(reader, serialize_version) {
        if !vars.is_empty() || looks_like {
            return Ok(vars);
        }
    }
    reader.seek_to(start)?;
    seek_to_safety_var_table(reader)?;
    try_read_variables_safety(reader, serialize_version)
}

fn try_read_variables_safety(reader: &mut MfcReader, serialize_version: u32) -> Result<Vec<Variable>> {
    skip_safety_var_header(reader)?;
    if reader.remaining_len() < 4 {
        return Ok(Vec::new());
    }
    let count = reader.read_u32()? as usize;
    if count == 0 {
        return Ok(Vec::new());
    }
    if count > SAFETY_VAR_MAX {
        bail!("变量数量异常: {}", count);
    }
    let mut vars = Vec::with_capacity(count);
    for idx in 0..count {
        skip_safety_zero_padding(reader)?;
        let offset = reader.position();
        let type_id = reader
            .read_u8()
            .with_context(|| format!("safety var entry idx={} type_id read offset={}", idx, offset))?;
        let mut entry = read_safety_db_object(reader, type_id, serialize_version, None).with_context(|| {
            format!(
                "safety var entry idx={} type_id=0x{:02X} offset={}",
                idx, type_id, offset
            )
        })?;
        vars.append(&mut entry);
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
            if reader.remaining_len() >= 5 {
                let tail = reader.remaining_slice();
                if tail[0] == 0x00 {
                    let count = u32::from_le_bytes([tail[1], tail[2], tail[3], tail[4]]) as usize;
                    if count > 0 && count <= SAFETY_VAR_MAX {
                        let _ = reader.read_u8()?;
                    }
                }
            }
            if reader.remaining_len() >= 4 {
                let count = reader.peek_u32()? as usize;
                if count > SAFETY_VAR_MAX && reader.remaining_len() >= 5 {
                    let extra_pos = reader.position();
                    let _ = reader.read_u8()?;
                    let count2 = reader.peek_u32()? as usize;
                    if count2 > SAFETY_VAR_MAX {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SafetyDbKind {
    Base,
    Struct,
    Array,
    Pointer,
    FunctionBlock,
}

fn safety_db_kind(type_id: u8) -> SafetyDbKind {
    match type_id {
        0x18 => SafetyDbKind::FunctionBlock,
        0x0B => SafetyDbKind::Struct,
        0x09 => SafetyDbKind::Array,
        0x0D => SafetyDbKind::Pointer,
        _ => SafetyDbKind::Base,
    }
}

fn read_safety_db_object(
    reader: &mut MfcReader,
    type_id: u8,
    serialize_version: u32,
    prefix: Option<&str>,
) -> Result<Vec<Variable>> {
    match safety_db_kind(type_id) {
        SafetyDbKind::Base => read_safety_base_db(reader, serialize_version, prefix),
        SafetyDbKind::Struct => read_safety_struct_db(reader, serialize_version, prefix),
        SafetyDbKind::Array => read_safety_array_db(reader, serialize_version, prefix),
        SafetyDbKind::Pointer => read_safety_pointer_db(reader, serialize_version, prefix),
        SafetyDbKind::FunctionBlock => read_safety_function_block_db(reader, serialize_version, prefix),
    }
}

fn read_safety_base_db(
    reader: &mut MfcReader,
    serialize_version: u32,
    prefix: Option<&str>,
) -> Result<Vec<Variable>> {
    let mut var = read_variable_safety(reader, serialize_version)?;
    if let Some(prefix) = prefix {
        if !prefix.is_empty() {
            var.name = format!("{}.{}", prefix, var.name);
        }
    }
    Ok(vec![var])
}

fn read_safety_struct_db(
    reader: &mut MfcReader,
    serialize_version: u32,
    prefix: Option<&str>,
) -> Result<Vec<Variable>> {
    let mut vars = read_safety_base_db(reader, serialize_version, prefix)?;
    let base_name = vars.first().map(|v| v.name.clone()).unwrap_or_default();
    let count = reader.read_u32()? as usize;
    vars.extend(read_safety_db_member_list(
        reader,
        count,
        serialize_version,
        &base_name,
        false,
    )?);
    Ok(vars)
}

fn read_safety_array_db(
    reader: &mut MfcReader,
    serialize_version: u32,
    prefix: Option<&str>,
) -> Result<Vec<Variable>> {
    let mut vars = read_safety_base_db(reader, serialize_version, prefix)?;
    let base_name = vars.first().map(|v| v.name.clone()).unwrap_or_default();

    let pair_count = reader.read_u32()? as usize;
    let _pair_count_dup = reader.read_u32()? as usize;
    if pair_count > SAFETY_VAR_MAX {
        bail!("数组维度数量异常: {}", pair_count);
    }
    for _ in 0..pair_count {
        let _ = reader.read_u32()?;
        let _ = reader.read_u32()?;
    }
    if serialize_version >= 0x44 {
        for _ in 0..pair_count {
            let _ = reader.read_mfc_string()?;
            let _ = reader.read_mfc_string()?;
        }
    }

    let count = reader.read_u32()? as usize;
    vars.extend(read_safety_db_member_list(
        reader,
        count,
        serialize_version,
        &base_name,
        false,
    )?);
    Ok(vars)
}

fn read_safety_pointer_db(
    reader: &mut MfcReader,
    serialize_version: u32,
    prefix: Option<&str>,
) -> Result<Vec<Variable>> {
    let vars = read_safety_base_db(reader, serialize_version, prefix)?;
    let _ = reader.read_mfc_string()?;
    if serialize_version >= 0x44 {
        let _ = reader.read_mfc_string()?;
    }
    Ok(vars)
}

fn read_safety_function_block_db(
    reader: &mut MfcReader,
    serialize_version: u32,
    prefix: Option<&str>,
) -> Result<Vec<Variable>> {
    let mut vars = read_safety_base_db(reader, serialize_version, prefix)?;
    let base_name = vars.first().map(|v| v.name.clone()).unwrap_or_default();

    for _ in 0..5 {
        let count = reader.read_u32()? as usize;
        vars.extend(read_safety_db_member_list(
            reader,
            count,
            serialize_version,
            &base_name,
            true,
        )?);
    }

    let kv_count = reader.read_u32()? as usize;
    if kv_count > SAFETY_VAR_MAX {
        bail!("FunctionBlock KV 数量异常: {}", kv_count);
    }
    for _ in 0..kv_count {
        let _ = reader.read_mfc_string()?;
        let _ = reader.read_mfc_string()?;
    }
    Ok(vars)
}

fn read_safety_db_member_list(
    reader: &mut MfcReader,
    count: usize,
    serialize_version: u32,
    prefix: &str,
    force_type_byte: bool,
) -> Result<Vec<Variable>> {
    if count > SAFETY_VAR_MAX {
        bail!("成员变量数量异常: {}", count);
    }
    let use_type_byte = if force_type_byte || serialize_version >= 0x2D {
        true
    } else {
        looks_like_member_type_byte(reader)
    };
    let mut vars = Vec::new();
    for idx in 0..count {
        let offset = reader.position();
        let type_id = if use_type_byte { reader.read_u8()? } else { 0x15 };
        let mut entry = read_safety_db_object(reader, type_id, serialize_version, Some(prefix))
            .with_context(|| {
                format!(
                    "safety db member idx={} type_id=0x{:02X} offset={}",
                    idx, type_id, offset
                )
            })?;
        vars.append(&mut entry);
    }
    Ok(vars)
}

fn looks_like_member_type_byte(reader: &MfcReader) -> bool {
    let buf = reader.remaining_slice();
    if buf.is_empty() {
        return false;
    }
    let type_id = buf[0];
    if !matches!(type_id, 0x08 | 0x09 | 0x0B | 0x0D | 0x18 | 0x15) {
        return false;
    }
    let mut idx = 1usize;
    scan_mfc_string_ascii(buf, &mut idx, 200).unwrap_or(false)
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
    if count == 0 || count > SAFETY_VAR_MAX {
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
    if count > SAFETY_VAR_MAX {
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

pub(crate) fn looks_like_safety_var_table(reader: &MfcReader) -> bool {
    let buf = reader.remaining_slice();
    if buf.len() < 6 {
        return false;
    }
    if buf.starts_with(&[0x00, 0x02, 0x41, 0x78]) {
        return true;
    }
    let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if count == 0 || count > SAFETY_VAR_MAX {
        return false;
    }
    let mut idx = 4usize;
    if idx >= buf.len() {
        return false;
    }
    idx += 1; // type_id
    scan_mfc_string_ascii(buf, &mut idx, 80).unwrap_or(false)
}

pub(crate) fn looks_like_safety_var_table_ahead(reader: &MfcReader, window: usize) -> bool {
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

pub(crate) fn find_safety_var_table_offset(reader: &MfcReader) -> Option<usize> {
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

fn seek_to_safety_var_table(reader: &mut MfcReader) -> Result<()> {
    let start = reader.position();
    let buf = reader.inner.get_ref();
    if start >= buf.len() {
        bail!("已到达文件末尾，无法寻找变量表");
    }
    if let Some(offset) = find_safety_var_table_offset(reader) {
        reader.seek_to(start + offset)?;
        return Ok(());
    }
    let slice = &buf[start..];
    for offset in 0..slice.len().saturating_sub(6) {
        let count = u32::from_le_bytes([
            slice[offset],
            slice[offset + 1],
            slice[offset + 2],
            slice[offset + 3],
        ]) as usize;
        if count == 0 || count > SAFETY_VAR_MAX {
            continue;
        }
        let mut idx = offset + 4;
        if idx >= slice.len() {
            continue;
        }
        idx += 1; // type_id
        if !scan_mfc_string_ascii(slice, &mut idx, 80).unwrap_or(false) {
            continue;
        }
        reader.seek_to(start + offset)?;
        return Ok(());
    }
    bail!("未找到 Safety 变量表起点");
}

pub(crate) fn find_normal_var_table_offset(reader: &MfcReader) -> Option<usize> {
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
        let count =
            u32::from_le_bytes([buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]) as usize;
        if count == 0 || count > SAFETY_VAR_MAX {
            continue;
        }
        if idx + 5 > buf.len() {
            continue;
        }
        let type_id = buf[idx + 4];
        if type_id != 0x15 && type_id != 0x18 {
            continue;
        }
        return Some(offset);
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
    for offset in 0..slice.len().saturating_sub(8) {
        let mut idx = offset;
        if !scan_mfc_string_ascii(slice, &mut idx, 80).unwrap_or(false) {
            continue;
        }
        if idx < slice.len() && slice[idx] == 0x00 {
            idx += 1;
        }
        if idx + 4 > slice.len() {
            continue;
        }
        let count =
            u32::from_le_bytes([slice[idx], slice[idx + 1], slice[idx + 2], slice[idx + 3]])
                as usize;
        if count == 0 || count > SAFETY_VAR_MAX {
            continue;
        }
        if idx + 5 > slice.len() {
            continue;
        }
        let type_id = slice[idx + 4];
        if type_id != 0x15 && type_id != 0x18 {
            continue;
        }
        reader.seek_to(start + offset)?;
        return Ok(());
    }
    bail!("未找到 Normal 变量表起点");
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

fn read_variable_safety(reader: &mut MfcReader, serialize_version: u32) -> Result<Variable> {
    if serialize_version >= 0x34 {
        return read_variable_safety_v34(reader, serialize_version);
    }
    read_variable_safety_legacy(reader)
}

fn read_variable_safety_legacy(reader: &mut MfcReader) -> Result<Variable> {
    let name = reader.read_mfc_string()?;
    let _ = reader.read_mfc_string()?; // name2
    let comment = read_base_db_comment(reader)?;
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

fn read_variable_safety_v34(reader: &mut MfcReader, serialize_version: u32) -> Result<Variable> {
    let name = reader.read_mfc_string()?;
    let _ = reader.read_mfc_string()?; // name2
    let comment = read_base_db_comment(reader)?;
    let data_type = reader.read_mfc_string()?;
    let _flag20 = reader.read_u8()?;
    let init_value = reader.read_mfc_string()?;
    let _flag48 = reader.read_u8()?;
    let addr_id = reader.read_u32()? as u64;
    let id2 = reader.read_u32()?;
    let _extra = reader.read_mfc_string()?;
    let mode = reader.read_u8()?;
    let var_id = reader.read_u16()?;
    let soe_flag = reader.read_u8()?;
    let _field76 = reader.read_u32()?;
    let area_code = if serialize_version >= 0x38 {
        Some(reader.read_u8()?)
    } else {
        None
    };
    if serialize_version >= 0x44 {
        let _ = reader.read_u8()?;
    }

    Ok(Variable {
        name,
        data_type,
        init_value,
        soe_enable: soe_flag != 0,
        power_down_keep: false,
        comment,
        var_id: Some(var_id),
        addr_id: Some(addr_id),
        mode: Some(mode),
        id2: Some(id2),
        area_code,
    })
}

fn read_base_db_comment(reader: &mut MfcReader) -> Result<String> {
    let start = reader.position();
    let count = reader.read_u32()? as usize;
    if count > SAFETY_VAR_MAX {
        reader.seek_to(start)?;
        return reader.read_mfc_string();
    }
    if count == 0 {
        return Ok(String::new());
    }
    let mut comment = String::new();
    for _ in 0..count {
        let lang = match reader.read_mfc_string() {
            Ok(value) => value,
            Err(_) => {
                reader.seek_to(start)?;
                return reader.read_mfc_string();
            }
        };
        let value = match reader.read_mfc_string() {
            Ok(value) => value,
            Err(_) => {
                reader.seek_to(start)?;
                return reader.read_mfc_string();
            }
        };
        let _ = lang;
        if comment.is_empty() {
            comment = value;
        }
    }
    Ok(comment)
}
