//! 通讯地址采集并生成模块：值解析（codec）。
//!
//! 目标（TASK-03）：把 Modbus 返回的原始数据解析为类型化值，并在失败时返回 `DecodeError`（不得 panic）。
//!
//! 当前约定（MVP）：
//! - `Bool` 仅从 Coil/Discrete（bit）读取，不从寄存器 bit 位读取。
//! - 16-bit 类型按 Modbus 寄存器常见约定处理（寄存器为 16-bit，大端）；`ByteOrder32` 仅对 32-bit 生效。

use super::model::{ByteOrder32, DataType};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum DecodedValue {
    Bool(bool),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
}

impl DecodedValue {
    pub fn to_value_display(&self, scale: f64) -> String {
        match self {
            DecodedValue::Bool(value) => {
                if *value {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            DecodedValue::Int16(value) => format!("{}", (*value as f64) * scale),
            DecodedValue::UInt16(value) => format!("{}", (*value as f64) * scale),
            DecodedValue::Int32(value) => format!("{}", (*value as f64) * scale),
            DecodedValue::UInt32(value) => format!("{}", (*value as f64) * scale),
            DecodedValue::Int64(value) => format!("{}", (*value as f64) * scale),
            DecodedValue::UInt64(value) => format!("{}", (*value as f64) * scale),
            DecodedValue::Float32(value) => format!("{}", (*value as f64) * scale),
            DecodedValue::Float64(value) => format!("{}", value * scale),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeError {
    #[error("insufficient registers: expected {expected} got {got}")]
    InsufficientRegisters { expected: usize, got: usize },

    #[error("insufficient bits: expected {expected} got {got}")]
    InsufficientBits { expected: usize, got: usize },

    #[error("unsupported data type for register decode: {0:?}")]
    UnsupportedRegisterDataType(DataType),

    #[error("unsupported data type for bit decode: {0:?}")]
    UnsupportedBitDataType(DataType),
}

pub fn decode_from_bits(data_type: DataType, bits: &[bool]) -> Result<DecodedValue, DecodeError> {
    match data_type {
        DataType::Bool => {
            if bits.is_empty() {
                return Err(DecodeError::InsufficientBits {
                    expected: 1,
                    got: 0,
                });
            }

            Ok(DecodedValue::Bool(bits[0]))
        }
        other => Err(DecodeError::UnsupportedBitDataType(other)),
    }
}

pub fn decode_from_registers(
    data_type: DataType,
    byte_order: ByteOrder32,
    registers: &[u16],
) -> Result<DecodedValue, DecodeError> {
    match data_type {
        DataType::Bool => Err(DecodeError::UnsupportedRegisterDataType(DataType::Bool)),
        DataType::Int16 => {
            let register = require_registers(registers, 1)?[0];
            Ok(DecodedValue::Int16(register as i16))
        }
        DataType::UInt16 => {
            let register = require_registers(registers, 1)?[0];
            Ok(DecodedValue::UInt16(register))
        }
        DataType::Int32 => {
            let bytes = read_u32_bytes(registers, byte_order)?;
            Ok(DecodedValue::Int32(i32::from_be_bytes(bytes)))
        }
        DataType::UInt32 => {
            let bytes = read_u32_bytes(registers, byte_order)?;
            Ok(DecodedValue::UInt32(u32::from_be_bytes(bytes)))
        }
        DataType::Int64 => {
            let bytes = read_u64_bytes(registers, byte_order)?;
            Ok(DecodedValue::Int64(i64::from_be_bytes(bytes)))
        }
        DataType::UInt64 => {
            let bytes = read_u64_bytes(registers, byte_order)?;
            Ok(DecodedValue::UInt64(u64::from_be_bytes(bytes)))
        }
        DataType::Float32 => {
            let bytes = read_u32_bytes(registers, byte_order)?;
            Ok(DecodedValue::Float32(f32::from_be_bytes(bytes)))
        }
        DataType::Float64 => {
            let bytes = read_u64_bytes(registers, byte_order)?;
            Ok(DecodedValue::Float64(f64::from_be_bytes(bytes)))
        }
        DataType::Unknown => Err(DecodeError::UnsupportedRegisterDataType(DataType::Unknown)),
    }
}

fn require_registers(registers: &[u16], expected: usize) -> Result<&[u16], DecodeError> {
    if registers.len() < expected {
        return Err(DecodeError::InsufficientRegisters {
            expected,
            got: registers.len(),
        });
    }

    Ok(&registers[..expected])
}

fn read_u32_bytes(registers: &[u16], byte_order: ByteOrder32) -> Result<[u8; 4], DecodeError> {
    let registers = require_registers(registers, 2)?;

    let first = registers[0].to_be_bytes();
    let second = registers[1].to_be_bytes();
    let raw = [first[0], first[1], second[0], second[1]];

    Ok(match byte_order {
        ByteOrder32::ABCD => raw,
        ByteOrder32::BADC => [raw[1], raw[0], raw[3], raw[2]],
        ByteOrder32::CDAB => [raw[2], raw[3], raw[0], raw[1]],
        ByteOrder32::DCBA => [raw[3], raw[2], raw[1], raw[0]],
        ByteOrder32::Unknown => raw,
    })
}

fn read_u64_bytes(registers: &[u16], byte_order: ByteOrder32) -> Result<[u8; 8], DecodeError> {
    let registers = require_registers(registers, 4)?;

    let r0 = registers[0].to_be_bytes();
    let r1 = registers[1].to_be_bytes();
    let r2 = registers[2].to_be_bytes();
    let r3 = registers[3].to_be_bytes();
    let raw = [r0[0], r0[1], r1[0], r1[1], r2[0], r2[1], r3[0], r3[1]];

    // For 64-bit types, we apply the same byte order pattern as 32-bit
    // but extend it to 8 bytes (treating it as two 32-bit words)
    Ok(match byte_order {
        ByteOrder32::ABCD => raw,
        ByteOrder32::BADC => [
            raw[1], raw[0], raw[3], raw[2], raw[5], raw[4], raw[7], raw[6],
        ],
        ByteOrder32::CDAB => [
            raw[4], raw[5], raw[6], raw[7], raw[0], raw[1], raw[2], raw[3],
        ],
        ByteOrder32::DCBA => [
            raw[7], raw[6], raw[5], raw[4], raw[3], raw[2], raw[1], raw[0],
        ],
        ByteOrder32::Unknown => raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_from_registers_returns_error_instead_of_panicking() {
        let error = decode_from_registers(DataType::UInt32, ByteOrder32::ABCD, &[]).unwrap_err();
        assert_eq!(
            error,
            DecodeError::InsufficientRegisters {
                expected: 2,
                got: 0
            }
        );
    }

    #[test]
    fn decode_vectors_cover_data_types_and_byte_orders() {
        struct RegCase {
            name: &'static str,
            data_type: DataType,
            byte_order: ByteOrder32,
            registers: Vec<u16>,
            expected: DecodedValue,
        }

        let cases: Vec<RegCase> = vec![
            // UInt32: 0x11223344 under 4 byte orders
            RegCase {
                name: "u32-ABCD",
                data_type: DataType::UInt32,
                byte_order: ByteOrder32::ABCD,
                registers: vec![0x1122, 0x3344],
                expected: DecodedValue::UInt32(0x11223344),
            },
            RegCase {
                name: "u32-BADC",
                data_type: DataType::UInt32,
                byte_order: ByteOrder32::BADC,
                registers: vec![0x2211, 0x4433],
                expected: DecodedValue::UInt32(0x11223344),
            },
            RegCase {
                name: "u32-CDAB",
                data_type: DataType::UInt32,
                byte_order: ByteOrder32::CDAB,
                registers: vec![0x3344, 0x1122],
                expected: DecodedValue::UInt32(0x11223344),
            },
            RegCase {
                name: "u32-DCBA",
                data_type: DataType::UInt32,
                byte_order: ByteOrder32::DCBA,
                registers: vec![0x4433, 0x2211],
                expected: DecodedValue::UInt32(0x11223344),
            },
            // Float32: 1.0f under 4 byte orders (0x3F800000)
            RegCase {
                name: "f32-ABCD",
                data_type: DataType::Float32,
                byte_order: ByteOrder32::ABCD,
                registers: vec![0x3F80, 0x0000],
                expected: DecodedValue::Float32(1.0),
            },
            RegCase {
                name: "f32-BADC",
                data_type: DataType::Float32,
                byte_order: ByteOrder32::BADC,
                registers: vec![0x803F, 0x0000],
                expected: DecodedValue::Float32(1.0),
            },
            RegCase {
                name: "f32-CDAB",
                data_type: DataType::Float32,
                byte_order: ByteOrder32::CDAB,
                registers: vec![0x0000, 0x3F80],
                expected: DecodedValue::Float32(1.0),
            },
            RegCase {
                name: "f32-DCBA",
                data_type: DataType::Float32,
                byte_order: ByteOrder32::DCBA,
                registers: vec![0x0000, 0x803F],
                expected: DecodedValue::Float32(1.0),
            },
            // Int32: -123456789 under 2 byte orders (0xF8A432EB)
            RegCase {
                name: "i32-ABCD",
                data_type: DataType::Int32,
                byte_order: ByteOrder32::ABCD,
                registers: vec![0xF8A4, 0x32EB],
                expected: DecodedValue::Int32(-123456789),
            },
            RegCase {
                name: "i32-CDAB",
                data_type: DataType::Int32,
                byte_order: ByteOrder32::CDAB,
                registers: vec![0x32EB, 0xF8A4],
                expected: DecodedValue::Int32(-123456789),
            },
            // 16-bit types
            RegCase {
                name: "i16",
                data_type: DataType::Int16,
                byte_order: ByteOrder32::ABCD,
                registers: vec![0xCFC7],
                expected: DecodedValue::Int16(-12345),
            },
            RegCase {
                name: "u16",
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                registers: vec![0xD431],
                expected: DecodedValue::UInt16(54321),
            },
        ];

        for case in cases {
            let got =
                decode_from_registers(case.data_type, case.byte_order, &case.registers).unwrap();
            assert_eq!(got, case.expected, "case {}", case.name);
        }

        let bool_value = decode_from_bits(DataType::Bool, &[true]).unwrap();
        assert_eq!(bool_value, DecodedValue::Bool(true));
    }
}
