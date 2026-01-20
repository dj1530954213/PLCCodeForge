use binrw::{BinResult, BinWrite};
use encoding::all::GBK;
use encoding::{EncoderTrap, Encoding};
use std::fs::File;
use std::io::{Seek, Write};

#[derive(Debug, Clone, Default)]
struct MfcString(String);

impl MfcString {
    fn new(s: &str) -> Self {
        Self(s.to_string())
    }
}

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

#[binrw::binwrite]
#[brw(little)]
#[derive(Debug)]
struct ModbusSlaveV026 {
    s0: MfcString,
    s1: MfcString,
    s2: MfcString,
    s3: MfcString,
    s4: MfcString,
    s5: MfcString,
    s6: MfcString,
    s7: MfcString,

    str_enabled: MfcString,
    str_ip_a: MfcString,
    str_ip_b: MfcString,
    str_unit_id: MfcString,
    str_port: MfcString,
    str_param13: MfcString,

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

    padding1: u32,
    padding2: u32,
    tail_flag: u8,
}

fn main() -> BinResult<()> {
    let payload = ModbusSlaveV026 {
        s0: MfcString::new(""),
        s1: MfcString::new(""),
        s2: MfcString::new(""),
        s3: MfcString::new(""),
        s4: MfcString::new(""),
        s5: MfcString::new(""),
        s6: MfcString::new(""),
        s7: MfcString::new(""),

        str_enabled: MfcString::new("1"),
        str_ip_a: MfcString::new("192.168.1.100"),
        str_ip_b: MfcString::new("0.0.0.0"),
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
        padding1: 0,
        padding2: 0,
        tail_flag: 0,
    };

    let mut file = File::create("payload.bin")?;
    payload.write(&mut file)?;
    println!(
        "Payload (V026 String-Based) Generated. Size: {} bytes",
        file.metadata()?.len()
    );
    Ok(())
}
