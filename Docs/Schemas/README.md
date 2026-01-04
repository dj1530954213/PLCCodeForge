# Schemas（稳定契约）

本目录用于存放“可交付/可融合”的 JSON Schema（冻结清单）。

## 1) CommIR v1

- Schema：`Docs/Schemas/comm_ir.v1.schema.json`
- 产物：由后端 `comm_export_ir_v1` 导出到 `outputDir/ir/comm_ir.v1.<ts>.json`

## 2) PlcImportBridge v1

- Schema：`Docs/Schemas/plc_import_bridge.v1.schema.json`
- 产物：由后端 `comm_bridge_to_plc_import_v1` 导出到 `outputDir/bridge/plc_import_bridge.v1.<ts>.json`

## 3) 演进策略（必须遵守）

### 3.1 v1 的兼容承诺

- `schemaVersion=1/specVersion=v1` 一旦对外使用，即视为稳定契约。
- v1 **只允许新增可选字段**，不得改名/删字段/改语义。

### 3.2 何时 bump v2

以下变更均视为破坏性变更，必须 bump `specVersion=v2`（同时提供 v1 兼容策略或迁移说明）：

- `readArea` 语义扩展（例如从 Holding/Coil 扩展到 Input/Discrete）。
- 地址基准语义变化（例如允许 1-based/40001 风格输入影响内部 addressBase）。
- 字段重命名/删除/类型变化/枚举值含义变化。

