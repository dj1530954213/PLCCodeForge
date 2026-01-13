# TASK-03-result.md

- **Task 编号与标题**：
  - TASK-03：codec（≥10 测试向量）

- **完成摘要**：
  - 在 `Tauri.CommMapping/src-tauri/src/comm/codec.rs` 实现解析器（bytes/registers/bits → typed value），支持 `Bool/Int16/UInt16/Int32/UInt32/Float32`。
  - 支持 32-bit 字节序 `ABCD/BADC/CDAB/DCBA`（按 Modbus 常见“字节序/字交换”含义），并在单测中锁死行为。
  - 解析失败返回 `DecodeError`（不 panic），并补充“空输入返回 DecodeError”的单测。

- **改动清单**：
  - `Tauri.CommMapping/src-tauri/src/comm/codec.rs`
    - 新增：`DecodedValue`（Bool/Int16/UInt16/Int32/UInt32/Float32）
    - 新增：`DecodeError`（寄存器不足/bit 不足/不支持的输入形态）
    - 新增：`decode_from_registers(DataType, ByteOrder32, &[u16])`
    - 新增：`decode_from_bits(DataType, &[bool])`
    - 新增：`DecodedValue::to_value_display(scale)`（用于后续 `valueDisplay`）
    - 新增单测：`decode_vectors_cover_data_types_and_byte_orders`（≥10 向量）与 `decode_from_registers_returns_error_instead_of_panicking`
  - `Tauri.CommMapping/Docs/ExecResults/TASK-03-result.md`
    - 新建任务结果归档文件（本文件）

- **关键实现说明**：
  - 32-bit 解析流程：把两个寄存器按 Modbus 常见大端拆成 4 个原始字节，然后按 `ByteOrder32` 把“原始字节序”重排回标准 `ABCD`（高位在前），再用 `from_be_bytes` 解码为 `i32/u32/f32`。
  - `Bool` 仅支持从 bit（Coil/Discrete）读取；对寄存器输入的 `Bool` 会返回 `UnsupportedRegisterDataType`（与执行要求一致：MVP 不做寄存器 bit 位）。

- **测试向量（>=10 组，写入单测并在此列出）**：
  - UInt32：目标值 `0x11223344`
    - `u32-ABCD`：registers `[0x1122, 0x3344]` → `0x11223344`
    - `u32-BADC`：registers `[0x2211, 0x4433]` → `0x11223344`
    - `u32-CDAB`：registers `[0x3344, 0x1122]` → `0x11223344`
    - `u32-DCBA`：registers `[0x4433, 0x2211]` → `0x11223344`
  - Float32：目标值 `1.0`（bits `0x3F800000`）
    - `f32-ABCD`：registers `[0x3F80, 0x0000]` → `1.0`
    - `f32-BADC`：registers `[0x803F, 0x0000]` → `1.0`
    - `f32-CDAB`：registers `[0x0000, 0x3F80]` → `1.0`
    - `f32-DCBA`：registers `[0x0000, 0x803F]` → `1.0`
  - Int32：目标值 `-123456789`（hex `0xF8A432EB`）
    - `i32-ABCD`：registers `[0xF8A4, 0x32EB]` → `-123456789`
    - `i32-CDAB`：registers `[0x32EB, 0xF8A4]` → `-123456789`
  - 16-bit
    - `i16`：registers `[0xCFC7]` → `-12345`
    - `u16`：registers `[0xD431]` → `54321`
  - Bool（bit）
    - bits `[true]` → `true`

- **完成证据**：
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml` 输出片段：
    ```text
    running 4 tests
    test comm::codec::tests::decode_vectors_cover_data_types_and_byte_orders ... ok
    test comm::codec::tests::decode_from_registers_returns_error_instead_of_panicking ... ok
    test comm::model::tests::points_v1_json_roundtrip_includes_schema_version_and_point_key ... ok
    test comm::model::tests::profiles_v1_json_roundtrip ... ok
    ```

- **验收自检**：
  - [x] 支持 `Bool/Int16/UInt16/Int32/UInt32/Float32`。
  - [x] 支持 `ABCD/BADC/CDAB/DCBA` 并有覆盖向量。
  - [x] 解析失败返回 `DecodeError`，无 panic（含单测）。
  - [x] `cargo test` 通过并输出测试名。
  - [x] `Tauri.CommMapping/Docs/ExecResults/TASK-03-result.md` 已归档。

- **风险/未决项**：
  - 16-bit 类型当前未引入单独的“16-bit 字节序”概念；MVP 按 Modbus 寄存器常见大端处理（如后续现场需要可再扩展为可选字段/可选策略）。

- **下一步建议**：
  - 进入 TASK-04：实现 `plan.rs`（按 `channelName` 分组、按 points 顺序做地址映射/聚合/分批，并提供排序稳定性单测）。
