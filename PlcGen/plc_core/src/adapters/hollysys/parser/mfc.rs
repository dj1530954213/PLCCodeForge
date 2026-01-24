use std::io::{Cursor, Read, Seek};

use anyhow::{Result, bail};
use binrw::{BinRead, BinResult, Endian};
use encoding_rs::GBK;

/// MFC CString: AfxReadStringLength + raw bytes (ANSI or UTF-16LE).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MfcString(pub String);

impl BinRead for MfcString {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<Self> {
        let (len, mode) = read_mfc_string_length(reader, endian)?;
        if len == 0 {
            return Ok(MfcString(String::new()));
        }
        match mode {
            1 => {
                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf)?;
                let (cow, _, _) = GBK.decode(&buf);
                Ok(MfcString(cow.into_owned()))
            }
            2 => {
                let byte_len = len.checked_mul(2).ok_or_else(|| binrw::Error::AssertFail {
                    pos: 0,
                    message: "wide string length overflow".into(),
                })?;
                let mut buf = vec![0u8; byte_len];
                reader.read_exact(&mut buf)?;
                let mut units = Vec::with_capacity(len);
                for chunk in buf.chunks_exact(2) {
                    units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
                }
                Ok(MfcString(String::from_utf16_lossy(&units)))
            }
            _ => Err(binrw::Error::AssertFail {
                pos: 0,
                message: "unknown CString encoding mode".into(),
            }),
        }
    }
}

fn read_mfc_string_length<R: Read + Seek>(reader: &mut R, endian: Endian) -> BinResult<(usize, u8)> {
    let mut mode = 1u8;
    let first = u8::read_options(reader, endian, ())?;
    if first != 0xFF {
        return Ok((first as usize, mode));
    }

    let len16 = u16::read_options(reader, endian, ())?;
    if len16 == 0xFFFE {
        mode = 2;
        let len8 = u8::read_options(reader, endian, ())?;
        if len8 != 0xFF {
            return Ok((len8 as usize, mode));
        }
        let len16b = u16::read_options(reader, endian, ())?;
        if len16b != 0xFFFF {
            return Ok((len16b as usize, mode));
        }
        let len32 = u32::read_options(reader, endian, ())?;
        if len32 == 0xFFFF_FFFF {
            let _ = u32::read_options(reader, endian, ())?;
            return Ok((0, mode));
        }
        return Ok((len32 as usize, mode));
    }

    if len16 != 0xFFFF {
        return Ok((len16 as usize, mode));
    }

    let len32 = u32::read_options(reader, endian, ())?;
    if len32 == 0xFFFF_FFFF {
        let _ = u32::read_options(reader, endian, ())?;
        return Ok((0, mode));
    }
    Ok((len32 as usize, mode))
}

/// MFC 二进制读取器（用于解析 Hollysys 的剪贴板数据）
pub struct MfcReader<'a> {
    pub(crate) inner: Cursor<&'a [u8]>,
}

impl<'a> MfcReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { inner: Cursor::new(data) }
    }

    pub(crate) fn position(&self) -> usize {
        self.inner.position() as usize
    }

    pub(crate) fn remaining_len(&self) -> usize {
        let pos = self.position();
        let len = self.inner.get_ref().len();
        len.saturating_sub(pos)
    }

    pub(crate) fn remaining_slice(&self) -> &'a [u8] {
        let pos = self.position();
        &self.inner.get_ref()[pos..]
    }

    pub(crate) fn remaining_all_zero(&self) -> bool {
        self.remaining_slice().iter().all(|v| *v == 0)
    }

    pub(crate) fn seek_to(&mut self, pos: usize) -> Result<()> {
        if pos > self.inner.get_ref().len() {
            bail!("seek 超出数据范围: {}", pos);
        }
        self.inner.set_position(pos as u64);
        Ok(())
    }

    pub(crate) fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub(crate) fn read_u64(&mut self) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.inner.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    pub(crate) fn peek_u8(&self) -> Result<u8> {
        let pos = self.position();
        let buf = self.inner.get_ref();
        if pos >= buf.len() {
            bail!("到达数据末尾，无法继续读取");
        }
        Ok(buf[pos])
    }

    pub(crate) fn peek_u16(&self) -> Result<u16> {
        let pos = self.position();
        let buf = self.inner.get_ref();
        if pos + 2 > buf.len() {
            bail!("到达数据末尾，无法继续读取");
        }
        Ok(u16::from_le_bytes([buf[pos], buf[pos + 1]]))
    }

    pub(crate) fn peek_u32(&self) -> Result<u32> {
        let pos = self.position();
        let buf = self.inner.get_ref();
        if pos + 4 > buf.len() {
            bail!("到达数据末尾，无法继续读取");
        }
        Ok(u32::from_le_bytes([buf[pos], buf[pos + 1], buf[pos + 2], buf[pos + 3]]))
    }

    pub(crate) fn align_to_4bytes(&mut self) -> Result<()> {
        let remainder = self.position() % 4;
        if remainder != 0 {
            let padding = 4 - remainder;
            let _ = self.read_bytes(padding)?;
        }
        Ok(())
    }

    pub(crate) fn read_mfc_string_length(&mut self) -> Result<(usize, u8)> {
        let mut mode = 1u8;
        let first = self.read_u8()?;
        if first != 0xFF {
            return Ok((first as usize, mode));
        }

        let len16 = self.read_u16()?;
        if len16 == 0xFFFE {
            mode = 2;
            let len8 = self.read_u8()?;
            if len8 != 0xFF {
                return Ok((len8 as usize, mode));
            }
            let len16b = self.read_u16()?;
            if len16b != 0xFFFF {
                return Ok((len16b as usize, mode));
            }
            let len32 = self.read_u32()?;
            if len32 == 0xFFFF_FFFF {
                let extra = self.read_u32()?;
                if extra != 0xFFFF_FFFF {
                    bail!("CString length sentinel mismatch");
                }
                return Ok((0, mode));
            }
            return Ok((len32 as usize, mode));
        }

        if len16 != 0xFFFF {
            return Ok((len16 as usize, mode));
        }

        let len32 = self.read_u32()?;
        if len32 == 0xFFFF_FFFF {
            let extra = self.read_u32()?;
            if extra != 0xFFFF_FFFF {
                bail!("CString length sentinel mismatch");
            }
            return Ok((0, mode));
        }
        Ok((len32 as usize, mode))
    }

    pub(crate) fn read_mfc_string(&mut self) -> Result<String> {
        let (len, mode) = self.read_mfc_string_length()?;
        if len == 0 {
            return Ok(String::new());
        }
        match mode {
            1 => {
                if len > self.remaining_len() {
                    bail!(
                        "CString length exceeds remaining bytes: len={} remaining={} pos={}",
                        len,
                        self.remaining_len(),
                        self.position()
                    );
                }
                let buf = self.read_bytes(len)?;
                let (cow, _, _) = GBK.decode(&buf);
                Ok(cow.into_owned())
            }
            2 => {
                let byte_len = len
                    .checked_mul(2)
                    .ok_or_else(|| anyhow::anyhow!("wide CString length overflow"))?;
                if byte_len > self.remaining_len() {
                    bail!(
                        "CString length exceeds remaining bytes: len={} remaining={} pos={}",
                        byte_len,
                        self.remaining_len(),
                        self.position()
                    );
                }
                let buf = self.read_bytes(byte_len)?;
                let mut units = Vec::with_capacity(len);
                for chunk in buf.chunks_exact(2) {
                    units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
                }
                Ok(String::from_utf16_lossy(&units))
            }
            _ => bail!("unknown CString encoding mode"),
        }
    }
}

pub(crate) fn scan_mfc_string_any(buf: &[u8], idx: &mut usize, max_len: usize) -> Result<bool> {
    let (len, mode) = match scan_mfc_string_len(buf, idx) {
        Some(pair) => pair,
        None => return Ok(false),
    };
    if len > max_len {
        return Ok(false);
    }
    let byte_len = if mode == 2 { len * 2 } else { len };
    if *idx + byte_len > buf.len() {
        return Ok(false);
    }
    *idx += byte_len;
    Ok(true)
}

pub(crate) fn scan_mfc_string_ascii(buf: &[u8], idx: &mut usize, max_len: usize) -> Result<bool> {
    let (len, mode) = match scan_mfc_string_len(buf, idx) {
        Some(pair) => pair,
        None => return Ok(false),
    };
    if len == 0 || len > max_len || mode != 1 {
        return Ok(false);
    }
    if *idx + len > buf.len() {
        return Ok(false);
    }
    if !buf[*idx..*idx + len].iter().all(|b| b.is_ascii_graphic()) {
        return Ok(false);
    }
    *idx += len;
    Ok(true)
}

pub(crate) fn scan_mfc_string_len(buf: &[u8], idx: &mut usize) -> Option<(usize, u8)> {
    if *idx >= buf.len() {
        return None;
    }
    let mut mode = 1u8;
    let first = buf[*idx];
    *idx += 1;
    if first != 0xFF {
        return Some((first as usize, mode));
    }
    if *idx + 2 > buf.len() {
        return None;
    }
    let len16 = u16::from_le_bytes([buf[*idx], buf[*idx + 1]]);
    *idx += 2;
    if len16 == 0xFFFE {
        mode = 2;
        if *idx >= buf.len() {
            return None;
        }
        let len8 = buf[*idx];
        *idx += 1;
        if len8 != 0xFF {
            return Some((len8 as usize, mode));
        }
        if *idx + 2 > buf.len() {
            return None;
        }
        let len16b = u16::from_le_bytes([buf[*idx], buf[*idx + 1]]);
        *idx += 2;
        if len16b != 0xFFFF {
            return Some((len16b as usize, mode));
        }
        if *idx + 4 > buf.len() {
            return None;
        }
        let len32 = u32::from_le_bytes([buf[*idx], buf[*idx + 1], buf[*idx + 2], buf[*idx + 3]]);
        *idx += 4;
        if len32 == 0xFFFF_FFFF {
            if *idx + 4 > buf.len() {
                return None;
            }
            *idx += 4;
            return Some((0, mode));
        }
        return Some((len32 as usize, mode));
    }
    if len16 != 0xFFFF {
        return Some((len16 as usize, mode));
    }
    if *idx + 4 > buf.len() {
        return None;
    }
    let len32 = u32::from_le_bytes([buf[*idx], buf[*idx + 1], buf[*idx + 2], buf[*idx + 3]]);
    *idx += 4;
    if len32 == 0xFFFF_FFFF {
        if *idx + 4 > buf.len() {
            return None;
        }
        *idx += 4;
        return Some((0, mode));
    }
    Some((len32 as usize, mode))
}
