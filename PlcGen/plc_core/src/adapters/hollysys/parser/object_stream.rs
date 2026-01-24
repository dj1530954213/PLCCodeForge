use anyhow::{Result, bail};
use encoding_rs::GBK;

use super::mfc::MfcReader;

#[derive(Debug, Default)]
pub(crate) struct ClassTable {
    classes: Vec<String>,
}

impl ClassTable {
    pub(crate) fn insert(&mut self, name: String) {
        self.classes.push(name);
    }

    pub(crate) fn get(&self, id: u32) -> Result<String> {
        if id == 0 || id as usize > self.classes.len() {
            bail!("未知的类引用 ID: {}", id);
        }
        Ok(self.classes[(id as usize) - 1].clone())
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.classes.iter().any(|item| item == name)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ObjectKind {
    Null,
    Reference(u32),
    New(String),
    UnknownClass(u32),
}

pub(crate) fn read_object_kind(reader: &mut MfcReader, table: &mut ClassTable) -> Result<ObjectKind> {
    let tag = reader.read_u16()?;
    if tag == 0x0000 {
        return Ok(ObjectKind::Null);
    }
    if tag == 0xFFFF {
        let name = read_runtime_class(reader)?;
        if !table.contains(&name) {
            table.insert(name.clone());
        }
        return Ok(ObjectKind::New(name));
    }
    if tag == 0x7FFF {
        let ext = reader.read_u32()?;
        if ext & 0x8000_0000 != 0 {
            let class_id = ext & 0x7FFF_FFFF;
            return match table.get(class_id) {
                Ok(name) => Ok(ObjectKind::New(name)),
                Err(_) => Ok(ObjectKind::UnknownClass(class_id)),
            };
        }
        return Ok(ObjectKind::Reference(ext));
    }
    if tag & 0x8000 != 0 {
        let class_id = (tag & 0x7FFF) as u32;
        return match table.get(class_id) {
            Ok(name) => Ok(ObjectKind::New(name)),
            Err(_) => Ok(ObjectKind::UnknownClass(class_id)),
        };
    }
    Ok(ObjectKind::Reference(tag as u32))
}

pub(crate) fn read_runtime_class(reader: &mut MfcReader) -> Result<String> {
    let _schema = reader.read_u16()?;
    let name_len = reader.read_u16()? as usize;
    if name_len >= 0x40 {
        bail!("runtime class name too long: {}", name_len);
    }
    let name_bytes = reader.read_bytes(name_len)?;
    let (cow, _, _) = GBK.decode(&name_bytes);
    Ok(cow.into_owned())
}

pub(crate) fn prefill_class_table(table: &mut ClassTable, buf: &[u8], end: usize) {
    let mut idx = 0usize;
    let end = end.min(buf.len());
    while idx + 6 <= end {
        if buf[idx] != 0xFF || buf[idx + 1] != 0xFF {
            idx += 1;
            continue;
        }
        let name_len = u16::from_le_bytes([buf[idx + 4], buf[idx + 5]]) as usize;
        if name_len == 0 || name_len >= 0x40 {
            idx += 1;
            continue;
        }
        if idx + 6 + name_len > end {
            break;
        }
        let name_bytes = &buf[idx + 6..idx + 6 + name_len];
        if !name_bytes.iter().all(|b| b.is_ascii_graphic()) {
            idx += 1;
            continue;
        }
        if !name_bytes.starts_with(b"C") {
            idx += 1;
            continue;
        }
        let (cow, _, _) = GBK.decode(name_bytes);
        let name = cow.into_owned();
        if !table.contains(&name) {
            table.insert(name);
        }
        idx += 6 + name_len;
    }
}
