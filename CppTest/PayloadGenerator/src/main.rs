use binrw::{BinResult, BinWrite, Endian};
use binrw::io::{Seek, Write};
use encoding::{Encoding, EncoderTrap};
use encoding::all::GBK;

#[derive(Debug, Clone)]
struct MfcString {
    value: String,
}

impl MfcString {
    fn new(value: impl Into<String>) -> Self {
        Self { value: value.into() }
    }
}

impl BinWrite for MfcString {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<()> {
        let bytes = encode_gbk_or_ascii(&self.value);
        let len = bytes.len();
        if len < 0xFF {
            writer.write_all(&[len as u8])?;
        } else {
            writer.write_all(&[0xFF])?;
            let len16 = u16::try_from(len).unwrap_or(u16::MAX);
            len16.write_options(writer, endian, ())?;
        }
        writer.write_all(&bytes)?;
        Ok(())
    }
}

#[binrw::binwrite]
#[brw(little)]
struct DeviceBase {
    name: MfcString,
    id: u32,
    flag1: u8,
    flag2: u8,
    description: MfcString,
}

#[binrw::binwrite]
#[brw(little)]
struct ModbusSlaveConfig {
    base: DeviceBase,

    // --- 真实业务数据 ---
    description: MfcString,
    enabled: u8,
    ip_address: u32, // C0 A8 01 64 (192.168.1.100)
    port: u32,       // 502
    timeout: u32,    // 2000
    retry_count: u32,// 3
    unit_id: u32,    // 1

    padding: [u8; 4], // Padding

    mapping_count: u16,     // 0
    version_reserved: u32,  // 0
    order_count: u32,       // 0
    channel_count: u32,     // 0
    extra_blob_len: u16,    // 0

    // 保留安全缓冲垫，确保万无一失
    #[brw(pad_after = 128)]
    tail_padding: (),
}

fn encode_gbk_or_ascii(value: &str) -> Vec<u8> {
    match GBK.encode(value, EncoderTrap::Strict) {
        Ok(bytes) => bytes,
        Err(_) => value.as_bytes().to_vec(),
    }
}

fn main() -> BinResult<()> {
    // 构造头部：注意 Name 必须唯一，不要和现有的重复
    let base = DeviceBase {
        name: MfcString::new("TCPIO_1_1_192_168_1_254"),
        id: 0x9999,
        flag1: 1,
        flag2: 1,
        description: MfcString::new("RUST_NODE"),
    };

    let config = ModbusSlaveConfig {
        base,
        description: MfcString::new("Inject Success!"),
        enabled: 1,
        ip_address: 0xC0A80164,
        port: 502,
        timeout: 2000,
        retry_count: 5,
        unit_id: 1,
        padding: [0u8; 4],

        mapping_count: 0,
        version_reserved: 0,
        order_count: 0,
        channel_count: 0,
        extra_blob_len: 0,
        tail_padding: (),
    };

    let mut file = std::fs::File::create("payload.bin")?;
    config.write(&mut file)?;

    println!("Payload Ready! Size: {} bytes", file.metadata()?.len());
    Ok(())
}
