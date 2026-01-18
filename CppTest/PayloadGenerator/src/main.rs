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
    // 头部我们必须写，否则 MFC 认不出对象名字
    name: MfcString,

    // ID 设为 0，防止被当成长度
    id: u32,

    // Flags 设为 0
    flag1: u8,
    flag2: u8,

    description: MfcString,
}

#[binrw::binwrite]
#[brw(little)]
struct ModbusSlaveSkeleton {
    base: DeviceBase,

    // Body 第一字段: 描述字符串 (写个短的)
    slave_desc: MfcString,

    // 暴力全零填充
    #[brw(pad_after = 1024)]
    safe_zone: (),
}

fn encode_gbk_or_ascii(value: &str) -> Vec<u8> {
    match GBK.encode(value, EncoderTrap::Strict) {
        Ok(bytes) => bytes,
        Err(_) => value.as_bytes().to_vec(),
    }
}

fn main() -> BinResult<()> {
    let base = DeviceBase {
        name: MfcString::new("TCPIO_1_1_192_168_1_100"),
        id: 0,
        flag1: 0,
        flag2: 0,
        description: MfcString::new("Skeleton"),
    };

    let payload = ModbusSlaveSkeleton {
        base,
        slave_desc: MfcString::new("Safe"),
        safe_zone: (),
    };

    let mut file = std::fs::File::create("payload.bin")?;
    payload.write(&mut file)?;

    println!("Skeleton Payload Generated! Size: {} bytes", file.metadata()?.len());
    Ok(())
}
