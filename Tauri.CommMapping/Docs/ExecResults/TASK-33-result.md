# TASK-33-result.md（CommIR v1 → PlcImportBridge v1 桥接导出：仅映射+校验）

## 1) 完成摘要

- 新增后端桥接导出：`comm_bridge_to_plc_import_v1`，把 `CommIR v1` 映射为 **PlcImportBridge v1 JSON**（不接入 plc_core、不做程序生成）。
- 新增 fail-fast 校验：schemaVersion/specVersion、pointKey 唯一、hmiName 非空、readArea 仅允许 Holding/Coil（MVP）。
- evidence `manifest.json` 增补：`outputs.plcBridgePath` / `outputs.plcBridgeDigest`（sha256），并支持可选拷贝到 evidence 目录/zip。
- 前端 ImportUnion 页面新增按钮：`导出 PLC Bridge（v1）`，默认使用最新 `irPath`，导出到 `outputDir/bridge/` 并展示路径。

## 2) 改动清单（文件路径 + 关键点）

- `Tauri.CommMapping/src-tauri/src/comm/bridge_plc_import.rs`
  - 定义并冻结 `PlcImportBridgeV1` schema（schemaVersion=1/specVersion=v1）。
  - `export_plc_import_bridge_v1(irPath,outPath)`：读取 CommIR → 校验 → 映射 points/verification/statistics → 原子写文件。
- `Tauri.CommMapping/src-tauri/src/comm/error.rs`
  - 新增结构化错误：`PlcBridgeError{kind,message,details}`（用于校验失败可展示）。
- `Tauri.CommMapping/src-tauri/src/comm/path_resolver.rs`
  - 新增 bridge 相关路径：`default_plc_bridge_path` / `bridge_check_dir`。
- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
  - 新增 commands：`comm_bridge_to_plc_import_v1`（spawn_blocking，永不 reject）。
  - evidence：`CommEvidencePackRequest` 新增可选 `plcBridgePath/copyPlcBridge`；manifest.outputs 增加 `plcBridgePath/plcBridgeDigest` 等字段。
- `Tauri.CommMapping/src-tauri/src/lib.rs`
  - 注册 commands：`comm_bridge_to_plc_import_v1`。
- `src/comm/api.ts`
  - 新增 `commBridgeToPlcImportV1` + TS 类型（含结构化错误 PlcBridgeError）。
  - evidence request 增加 `plcBridgePath/copyPlcBridge`。
- `src/comm/services/evidencePack.ts`
  - 透传 `plcBridgePath` 到 `commEvidencePackCreate`。
- `src/comm/pages/ImportUnion.vue`
  - 新增按钮/展示：`导出 PLC Bridge（v1）`（使用最新 irPath）。

## 3) build/test 证据

### 3.1 cargo test

```text
running 33 tests
...
test comm::bridge_plc_import::tests::bridge_export_writes_file_and_contains_points_and_stats ... ok
test comm::bridge_plc_import::tests::bridge_export_fails_on_duplicate_point_key ... ok
...
test comm::tauri_api::tests::evidence_pack_manifest_includes_ir_path_and_digest_and_output_dir ... ok

test result: ok. 33 passed; 0 failed
```

### 3.2 pnpm build

```text
> plc-code-forge@0.1.0 build C:\Program Files\Git\code\PLCCodeForge
> vue-tsc --noEmit && vite build

vite v6.4.1 building for production...
✓ 1458 modules transformed.
✓ built in 3.66s
```

## 4) PlcImportBridge v1 JSON 样例片段

样例文件（来自单测生成）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-bridge-4dd29bd2-67ea-48d6-a700-a98718e30c80\bridge\plc_import_bridge.v1.test.json`

```json
{
  "schemaVersion": 1,
  "specVersion": "v1",
  "sourceIrPath": "C:\\Users\\DELL\\AppData\\Local\\Temp\\...\\comm_ir.v1....json",
  "sourceIrDigest": "sha256:0f8812c4fd6e19ac9e03a5e1a9b58ae000f52c39d957e50d6a09bf4e3fe2b5ec",
  "points": [
    {
      "name": "P1",
      "comm": {
        "channelName": "ch1",
        "addressSpec": { "readArea": "Holding", "absoluteAddress": 1, "addressBase": "zero" },
        "dataType": "UInt16",
        "endian": "ABCD",
        "scale": 1.0,
        "rw": "R"
      },
      "verification": { "quality": "Ok", "valueDisplay": "1", "message": "" }
    }
  ],
  "statistics": { "total": 1, "ok": 1, "timeout": 0, "commError": 0, "decodeError": 0, "configError": 0 }
}
```

## 5) evidence manifest 片段（outputs.plcBridgePath + plcBridgeDigest）

样例文件（来自单测生成）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-evidence-abadd593-43aa-4ad2-8b8a-43b8d33217e0\deliveries\evidence\20260103T135704Z-1767448624497\manifest.json`

```json
{
  "outputs": {
    "plcBridgePath": "C:\\Users\\DELL\\AppData\\Local\\Temp\\...\\deliveries\\bridge\\plc_import_bridge.v1.test.json",
    "plcBridgeDigest": "sha256:a85013b01ae33035249dcea5268fb7adf72db9019d2b8c10c8a67a1a791ec0ec"
  }
}
```

## 6) 校验失败示例（duplicate pointKey → 结构化错误）

样例文件（来自单测生成）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-bridge-dup-83bcb26c-2fd0-404e-8774-a1083d596fa4\bridge_error.json`

```json
{
  "kind": "CommIrValidationError",
  "message": "duplicate pointKey detected in CommIR",
  "details": {
    "irPath": "C:\\Users\\DELL\\AppData\\Local\\Temp\\...\\ir.json",
    "pointKey": "00000000-0000-0000-0000-00000000002a",
    "field": "pointKey"
  }
}
```

## 7) UI 操作步骤（ImportUnion 页）

1) 先运行 `一键演示（Wizard）`，页面提示 `CommIR 已导出：...`（获得 `irPath`）
2) 点击 `导出 PLC Bridge（v1）`
3) 页面提示 `PLC Bridge 已导出：...`，可点击 `打开 Bridge` 查看生成的 `plc_import_bridge.v1.*.json`

## 8) 自检清单（逐条勾选）

- [x] 仅桥接映射，不引入/不依赖 plc_core
- [x] command 内文件 IO 走 `spawn_blocking`（不阻塞 UI）
- [x] fail-fast：schema/spec、pointKey 唯一、hmiName 非空、readArea 限定 Holding/Coil
- [x] evidence manifest.outputs 增加 `plcBridgePath/plcBridgeDigest`（可选拷贝）
- [x] UI 可操作：ImportUnion 页面可导出 bridge 并看到 outPath

## 9) 风险与未决项

- 当前桥接 schema 以 `name=hmiName` 为主键语义；若 plc_core 最终需要额外稳定键（例如 channel+device+addr），只能在 v1 上新增可选字段或 bump v2。
- 当前强制 readArea 仅允许 Holding/Coil（MVP）；未来支持 Input/Discrete 需放宽校验并 bump specVersion（避免静默改变语义）。
