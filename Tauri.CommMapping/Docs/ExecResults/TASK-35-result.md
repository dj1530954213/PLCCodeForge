# TASK-35-result.md（PlcImportBridge v1 稳定化：schema + 版本策略 + golden tests）

## 1) 完成摘要

- 新增两份 JSON Schema（冻结资产）：`CommIR v1` 与 `PlcImportBridge v1`，用于长期对齐与验收。
- 新增桥接输出 golden fixtures + 单测锁定（忽略可变字段：timestamp/path），避免后续迭代破坏三模块合并前提。
- 明确并写死输出确定性规则：`PlcImportBridge.points` 顺序保持为 `CommIR.mapping.points` 的原始顺序（冻结）。
- 增补 v1 演进策略文档：v1 仅允许新增可选字段；破坏性变更必须 bump `specVersion=v2`（尤其 readArea 扩展）。

## 2) 改动清单（文件路径 + 关键点）

- `Tauri.CommMapping/Docs/Schemas/comm_ir.v1.schema.json`
  - CommIR v1 的最小可用 JSON Schema（字段/必填/枚举）。
- `Tauri.CommMapping/Docs/Schemas/plc_import_bridge.v1.schema.json`
  - PlcImportBridge v1 的最小可用 JSON Schema（字段/必填/枚举；readArea 限定 Holding/Coil）。
- `Tauri.CommMapping/Docs/Schemas/README.md`
  - 增加“演进策略”小节（v1 兼容承诺 + 何时 bump v2）。
- `Tauri.CommMapping/src-tauri/src/comm/fixtures/comm_ir.sample.v1.json`
  - golden 输入：3 点位（Holding + Coil），结果包含 Ok + Timeout。
- `Tauri.CommMapping/src-tauri/src/comm/fixtures/plc_import_bridge.expected.v1.json`
  - golden 期望输出（字段与数值固定）。
- `Tauri.CommMapping/src-tauri/src/comm/bridge_plc_import.rs`
  - 增加 Unknown dataType/endian 的 fail-fast 校验（bridge v1 不允许 Unknown）。
  - 增加 golden test：`bridge_export_matches_golden_fixture_ignoring_generated_at_and_source_path`。
  - 写死确定性说明注释：points 顺序冻结为 CommIR 原始顺序。

## 3) build/test 证据

### 3.1 cargo test（包含 golden test）

```text
running 36 tests
...
test comm::bridge_plc_import::tests::bridge_export_matches_golden_fixture_ignoring_generated_at_and_source_path ... ok
...
test result: ok. 36 passed; 0 failed
```

## 4) Schema 文件（路径 + 关键枚举片段）

- `Tauri.CommMapping/Docs/Schemas/comm_ir.v1.schema.json`
  - `sources.resultsSource`：`appdata|runLatest`
  - `dataType`：`Bool|Int16|UInt16|Int32|UInt32|Float32|Unknown`
  - `byteOrder32`：`ABCD|BADC|CDAB|DCBA|Unknown`
  - `registerArea`：`Holding|Input|Coil|Discrete`
- `Tauri.CommMapping/Docs/Schemas/plc_import_bridge.v1.schema.json`
  - `specVersion`：固定 `"v1"`
  - `addressSpec.readArea`（MVP）：`Holding|Coil`
  - `dataType`：`Bool|Int16|UInt16|Int32|UInt32|Float32`
  - `endian`：`ABCD|BADC|CDAB|DCBA`
  - `quality`：`Ok|Timeout|CommError|DecodeError|ConfigError`

## 5) Fixtures（文件名 + expected 片段）

- `Tauri.CommMapping/src-tauri/src/comm/fixtures/comm_ir.sample.v1.json`
- `Tauri.CommMapping/src-tauri/src/comm/fixtures/plc_import_bridge.expected.v1.json`

expected 片段（points 顺序固定）：

```json
{
  "points": [
    { "name": "HOLD_U16_OK", "verification": { "quality": "Ok" } },
    { "name": "COIL_BOOL_TIMEOUT", "verification": { "quality": "Timeout" } },
    { "name": "HOLD_F32_OK", "verification": { "quality": "Ok" } }
  ],
  "statistics": { "total": 3, "ok": 2, "timeout": 1 }
}
```

## 6) 输出确定性规则（写死）

- `PlcImportBridge.points`：严格按 `CommIR.mapping.points` 原始顺序输出（不做排序、不依赖 HashMap 遍历）。
- golden 对比忽略：`generatedAtUtc`、`sourceIrPath`（这两项天然可变）；其余字段严格对齐（含 `sourceIrDigest` 与统计计数）。

## 7) 自检清单（逐条勾选）

- [x] 新增 `Tauri.CommMapping/Docs/Schemas/comm_ir.v1.schema.json`
- [x] 新增 `Tauri.CommMapping/Docs/Schemas/plc_import_bridge.v1.schema.json`
- [x] 新增 fixtures + golden test 锁定输出
- [x] points 输出顺序规则已写死并被 golden 覆盖
- [x] 演进策略已写入文档：v1 仅加可选字段；readArea 扩展必须 bump specVersion

## 8) 风险与未决项

- JSON Schema 当前为“最小可用”（覆盖必填/枚举/关键字段）；若后续需要更严格的数值范围（例如地址上限/长度上限），建议以 v1 增补约束（不改变语义）。
- 若未来要把 `Unknown` 从 CommIR v1 中彻底禁止，需要：1) IR 导出侧增加校验/告警策略；2) 在 v2 中收紧 schema（避免对既有数据产生破坏性影响）。

