use binrw::{binrw, BinResult, BinWrite};
use std::fs::File;
use std::io::{Seek, Write};
use encoding::{Encoding, EncoderTrap};
use encoding::all::GBK;

// ============================================================================
// 1. MFC String (保持不变)
// ============================================================================
#[derive(Debug, Clone, Default)]
struct MfcString(String);

impl MfcString {
    fn new(s: &str) -> Self { Self(s.to_string()) }
}

impl BinWrite for MfcString {
    type Args<'a> = ();
    fn write_options<W: Seek + Write>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        _: Self::Args<'_>,
    ) -> BinResult<()> {
        let bytes = GBK.encode(&self.0, EncoderTrap::Strict)
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
// 2. Modbus Slave V026 (基于 17 Dwords 实锤结构)
// ============================================================================
#[binrw::binwrite]
#[brw(little)]
#[derive(Debug)]
struct ModbusSlaveV026 {
    // [Part 1] 14 Strings (8 empty + 6 data)
    #[bw(map = |_:&()| MfcString::new(""))] _s0: (),
    #[bw(map = |_:&()| MfcString::new(""))] _s1: (),
    #[bw(map = |_:&()| MfcString::new(""))] _s2: (),
    #[bw(map = |_:&()| MfcString::new(""))] _s3: (),
    #[bw(map = |_:&()| MfcString::new(""))] _s4: (),
    #[bw(map = |_:&()| MfcString::new(""))] _s5: (),
    #[bw(map = |_:&()| MfcString::new(""))] _s6: (),
    #[bw(map = |_:&()| MfcString::new(""))] _s7: (),

    // Config Strings
    #[bw(map = |s| MfcString::new(s))] str_enabled: String,   // "1"
    #[bw(map = |s| MfcString::new(s))] str_ip_a: String,      // "192.168.1.100"
    #[bw(map = |s| MfcString::new(s))] str_ip_b: String,      // "0.0.0.0"
    #[bw(map = |s| MfcString::new(s))] str_unit_id: String,   // "1"
    #[bw(map = |s| MfcString::new(s))] str_port: String,      // "502"
    #[bw(map = |s| MfcString::new(s))] str_param13: String,   // "0"

    // [Part 2] Tail Block (17 u32 + 1 u8)
    // 这是最稳健的写法，不用担心偏移量
    tail_dwords: [u32; 17],
    tail_end: u8,
}

fn main() -> BinResult<()> {
    let payload = ModbusSlaveV026 {
        _s0:(), _s1:(), _s2:(), _s3:(), _s4:(), _s5:(), _s6:(), _s7:(),

        str_enabled: "1".to_string(),
        str_ip_a: "192.168.1.100".to_string(),
        str_ip_b: "0.0.0.0".to_string(),
        str_unit_id: "1".to_string(),
        str_port: "502".to_string(),
        str_param13: "0".to_string(),

        // 直接复刻 empty_slave.bin 的二进制指纹
        tail_dwords: [
            0x00000000, 0x00000001, 0xFFFFFFFF, 0x00000000, 0xFF000000,
            0x00000000, 0x00000000, 0xFFFFFFFF, 0x00000000, 0xFFFFFFFF,
            0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000
        ],
        tail_end: 0x00,
    };

    let mut file = File::create("payload.bin")?;
    payload.write(&mut file)?;
    
    println!("Payload Generated. Size: {} bytes", file.metadata()?.len());
    Ok(())
}