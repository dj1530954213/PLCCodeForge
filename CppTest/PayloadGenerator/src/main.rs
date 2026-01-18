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
struct ModbusSlaveSkeleton {
    base: DeviceBase,
    slave_desc: MfcString,

    #[brw(pad_after = 512)]
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
        id: 0xDEADBEEF,
        flag1: 1,
        flag2: 1,
        description: MfcString::new("SkeletonTest"),
    };

    let payload = ModbusSlaveSkeleton {
        base,
        slave_desc: MfcString::new("SafeMode"),
        safe_zone: (),
    };

    let mut file = std::fs::File::create("payload.bin")?;
    payload.write(&mut file)?;

    println!("Skeleton Payload generated! Size: {} bytes", file.metadata()?.len());
    Ok(())
}
