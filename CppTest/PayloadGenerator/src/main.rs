use binrw::{binrw, BinResult, BinWrite}; // 关键：导入 BinWrite trait
use encoding::all::GBK;
use encoding::{EncoderTrap, Encoding};
use std::fs::File;
use std::io::{Seek, Write};

// ============================================================================
// 1. MFC String (保持不变)
// ============================================================================
#[derive(Debug, Clone, Default)]
struct MfcString(String);

impl MfcString {
    fn new(s: &str) -> Self {
        Self(s.to_string())
    }
}

// 实现 BinWrite trait
impl BinWrite for MfcString {
    type Args<'a> = ();

    fn write_options<W: Seek + Write>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        _: Self::Args<'_>,
    ) -> BinResult<()> {
        let bytes = GBK
            .encode(&self.0, EncoderTrap::Strict)
            .unwrap_or_else(|_| self.0.as_bytes().to_vec());
        let len = bytes.len();

        if len < 0xFF {
            (len as u8).write_options(writer, endian, ())?;
        } else if len < 0xFFFE {
            0xFFu8.write_options(writer, endian, ())?;
            (len as u16).write_options(writer, endian, ())?;
        } else {
            0xFFu8.write_options(writer, endian, ())?;
            0xFFFFu16.write_options(writer, endian, ())?;
            (len as u32).write_options(writer, endian, ())?;
        }

        writer.write_all(&bytes)?;
        Ok(())
    }
}

// ============================================================================
// 2. Modbus Slave V026 (修复填充)
// ============================================================================
#[binrw::binwrite]
#[brw(little)]
#[derive(Debug)]
struct ModbusSlaveV026 {
    // --- 字符串区 (共 40 字节) ---
    // (你当前的字符串长度正好凑齐 40 字节，所以我们只要补齐后面的 71 字节二进制块即可)
    #[bw(map = |_:&()| MfcString::new(""))] s0: (),
    #[bw(map = |_:&()| MfcString::new(""))] s1: (),
    #[bw(map = |_:&()| MfcString::new(""))] s2: (),
    #[bw(map = |_:&()| MfcString::new(""))] s3: (),
    #[bw(map = |_:&()| MfcString::new(""))] s4: (),
    #[bw(map = |_:&()| MfcString::new(""))] s5: (),
    #[bw(map = |_:&()| MfcString::new(""))] s6: (),
    #[bw(map = |_:&()| MfcString::new(""))] s7: (),

    str_enabled: MfcString,
    str_ip_a: MfcString,
    str_ip_b: MfcString,
    str_unit_id: MfcString,
    str_port: MfcString,
    str_param13: MfcString,

    // --- 固定二进制块 (必须凑齐 71 字节) ---
    
    // Magic 1-10 (40 bytes)
    magic1: u32,
    magic2: u32,
    magic3: u32,
    magic4: u32,
    magic5: u32,
    magic6: u32,
    magic7: u32,
    magic8: u32,
    magic9: u32,
    magic10: u32,

    // Magic 11-12 (8 bytes)
    magic11: u32,
    magic12: u32,

    // 🚨 关键修正点 🚨
    // 之前被 Codex 错误改成了 magic_pad: u16 (2 bytes)
    // 必须改回 [u8; 11] (11 bytes) 才能补齐那丢失的 9 字节
    magic_pad: [u8; 11], 

    // Counts & Extra (12 bytes)
    mapping_count: u16,
    order_count: u32,
    channel_count: u32,
    extra_len: u16,
}

fn main() -> BinResult<()> {
    let payload = ModbusSlaveV026 {
        s0:(), s1:(), s2:(), s3:(), s4:(), s5:(), s6:(), s7:(),

        str_enabled: MfcString::new("1"),
        str_ip_a: MfcString::new("192.168.1.100"), // 13 bytes (+1 len)
        str_ip_b: MfcString::new("0.0.0.0"),       // 7 bytes (+1 len)
        str_unit_id: MfcString::new("1"),
        str_port: MfcString::new("502"),
        str_param13: MfcString::new("0"),

        magic1: 0,
        magic2: 1,
        magic3: 0xFFFFFFFF,
        magic4: 0,
        magic5: 0xFF000000,
        magic6: 0,
        magic7: 0,
        magic8: 0xFFFFFFFF,
        magic9: 0,
        magic10: 0xFFFFFFFF,
        magic11: 0,
        magic12: 0,
        
        // 恢复填充
        magic_pad: [0u8; 11],

        mapping_count: 0,
        order_count: 0,
        channel_count: 0,
        extra_len: 0,
    };

    let mut file = File::create("payload.bin")?;
    payload.write(&mut file)?;

    let size = file.metadata()?.len();
    println!("Payload Generated. Size: {} bytes (Target: 111 bytes)", size);
    
    if size == 111 {
        println!("✅ PERFECT MATCH! Size is exactly 111 bytes.");
    } else {
        println!("⚠️ Size mismatch: expected 111, got {}. (Missing {} bytes)", size, 111 - size);
    }
    Ok(())
}