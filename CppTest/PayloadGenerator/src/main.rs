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
struct DeviceBase {
    name: MfcString,
    id: u32,
    flag1: u8,
    flag2: u8,
    description: MfcString,
}

#[binrw::binwrite]
#[brw(little)]
#[derive(Debug)]
struct ModbusSlaveV026 {
    base: DeviceBase,
    description: MfcString,
    enabled: u8,
    ip_address: u32,
    port: u32,
    timeout: u32,
    retry_count: u32,
    unit_id: u32,
    flags: [u8; 4],
    mapping_count: u16,
    mappings: Vec<u8>,
    order_count: u32,
    orders: Vec<u8>,
    channel_count: u32,
    channels: Vec<u8>,
    extra_len: u16,
    extra_data: Vec<u8>,
}

fn main() -> BinResult<()> {
    let mappings: Vec<u8> = vec![];
    let orders: Vec<u8> = vec![];
    let channels: Vec<u8> = vec![];
    let extra_data: Vec<u8> = vec![];

    let payload = ModbusSlaveV026 {
        base: DeviceBase {
            name: MfcString::new("TCPIO_1_1_192_168_1_222"),
            id: 0,
            flag1: 1,
            flag2: 1,
            description: MfcString::new("Rust_Gen"),
        },
        description: MfcString::new("V026_Strict"),
        enabled: 1,
        ip_address: 0xC0A801DE,
        port: 502,
        timeout: 2000,
        retry_count: 3,
        unit_id: 1,
        flags: [0, 0, 0, 0],
        mapping_count: mappings.len() as u16,
        mappings,
        order_count: orders.len() as u32,
        orders,
        channel_count: channels.len() as u32,
        channels,
        extra_len: extra_data.len() as u16,
        extra_data,
    };

    let mut file = File::create("payload.bin")?;
    payload.write(&mut file)?;
    println!(
        "Payload (V026 Strict) Generated. Size: {} bytes",
        file.metadata()?.len()
    );
    Ok(())
}
