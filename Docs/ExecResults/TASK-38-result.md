# TASK-38-result.md（UnifiedImport v1 → plc_import_stub v1 + 映射文档）

## 1) 完成摘要

- 新增最小转换 command：`comm_unified_export_plc_import_stub_v1`，读取 `UnifiedImport v1`，导出 **plc_import_stub v1 JSON**（不引入 plc_core、不调用 orchestrate、不生成程序）。
- 输出默认落盘：`outputDir/plc_import_stub/plc_import.v1.<ts>.json`（路径由 TASK-32 的 `outputDir` 决定；可用 outPath 覆盖）。
- 确定性锁定：points 输出顺序 **严格按 UnifiedImport.points 原始顺序**（新增单测防回归）。
- 新增字段对齐说明文档：`Docs/Integration/plc_core_import_mapping.v1.md`（冻结 v1）。
- evidence `manifest.json` 增补：`outputs.plcImportStubPath/plcImportStubDigest`（sha256），并支持可选拷贝进 evidence 目录/zip。

## 2) 改动清单（文件路径 + 关键点）

- `src-tauri/src/comm/export_plc_import_stub.rs`
  - 定义并冻结 `PlcImportStubV1` schema（schemaVersion=1/specVersion=v1）。
  - `export_plc_import_stub_v1(unifiedImportPath,outPath)`：读取 unified → 校验 → 写 stub → 返回 digest/summary。
  - 单测：`unified_to_plc_import_stub_preserves_points_order`（锁定确定性）。
  - MVP 校验：若 `readArea` 为 Input/Discrete 则 fail-fast（提示 allowedValues），扩展需 bump specVersion。
- `src-tauri/src/comm/error.rs`
  - 新增结构化错误：`UnifiedPlcImportStubError{kind,message,details}`。
- `src-tauri/src/comm/tauri_api.rs`
  - 新增 DTO + command：`comm_unified_export_plc_import_stub_v1`（spawn_blocking，永不 reject）。
  - evidence：`CommEvidencePackRequest` 新增 `plcImportStubPath/copyPlcImportStub`；manifest.outputs 增加 digest/relPath 字段。
- `src-tauri/src/lib.rs`
  - 注册 command：`comm_unified_export_plc_import_stub_v1`。
- `src/comm/api.ts`
  - 新增 `commUnifiedExportPlcImportStubV1` + TS 类型（UnifiedPlcImportStubError/summary/response）。
- `src/comm/pages/ImportUnion.vue`
  - 新增按钮：`导出 PLC Import stub（v1）`，并展示/打开 outPath。
- `Docs/Integration/plc_core_import_mapping.v1.md`
  - 固化 UnifiedImport → ImportResult 语义映射说明（冻结 v1）。

## 3) build/test 证据

### 3.1 cargo build

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s
```

### 3.2 cargo test（包含确定性单测）

```text
running 39 tests
...
test comm::export_plc_import_stub::tests::unified_to_plc_import_stub_preserves_points_order ... ok
...
test result: ok. 39 passed; 0 failed
```

### 3.3 pnpm build（UI 有改动）

```text
> plc-code-forge@0.1.0 build C:\Program Files\Git\code\PLCCodeForge
> vue-tsc --noEmit && vite build
...
✓ built in 3.49s
```

## 4) 映射文档片段（字段对齐）

文件：`Docs/Integration/plc_core_import_mapping.v1.md`

```text
UnifiedImport.points[i].comm.addressSpec → points[i].comm.addressSpec
UnifiedImport.points[i].comm.dataType    → points[i].comm.dataType
UnifiedImport.points[i].comm.endian      → points[i].comm.endian
UnifiedImport.points[i].comm.scale       → points[i].comm.scale
UnifiedImport.points[i].verification.*   → points[i].verification.*
```

## 5) plc_import_stub v1 样例片段

文件（来自单测生成）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-plc-import-c8d29ac5-7625-410c-9010-aaec249ac4b6\plc_import.v1.test.json`

```json
{
  "schemaVersion": 1,
  "specVersion": "v1",
  "sourceUnifiedImportPath": "C:\\Users\\DELL\\AppData\\Local\\Temp\\...\\unified_import.v1.test.json",
  "points": [
    { "name": "A" },
    { "name": "B", "comm": { "channelName": "ch" }, "verification": { "quality": "Ok" } }
  ],
  "deviceGroups": [],
  "hardware": {},
  "statistics": { "points": 2, "commCovered": 1, "verified": 1 }
}
```

## 6) manifest.json outputs 片段（plcImportStubPath + digest）

文件（来自单测生成 evidence pack）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-evidence-ae09572e-e535-4a0e-8b5d-df26b2f931cf\deliveries\evidence\20260103T153924Z-1767454764437\manifest.json`

```json
{
  "plcImportStubPath": "C:\\...\\deliveries\\plc_import_stub\\plc_import.v1.test.json",
  "plcImportStubDigest": "sha256:a85013b01ae33035249dcea5268fb7adf72db9019d2b8c10c8a67a1a791ec0ec"
}
```

## 7) UI 操作步骤（ImportUnion 页）

1) 完成 TASK-37 的 `合并生成 UnifiedImport（v1）`
2) 点击 `导出 PLC Import stub（v1）` → 页面提示 `PLC Import stub 已导出：...`，可点击打开文件

## 8) 自检清单（逐条勾选）

- [x] 不引入 plc_core 依赖 / 不调用 orchestrate / 不生成程序
- [x] 输出路径默认在 `outputDir/plc_import_stub/`（可 outPath 覆盖）
- [x] points 输出顺序确定性（按 UnifiedImport.points 原始顺序），并有单测锁定
- [x] evidence manifest.outputs 增加 `plcImportStubPath/plcImportStubDigest`
- [x] command IO 使用 `spawn_blocking`（不阻塞 UI）
- [x] DTO 契约不破坏（仅新增可选字段/新增命令）

## 9) 风险与未决项

- `plc_import_stub.v1` 当前对 `readArea` 做 MVP 限制（Holding/Coil）；若后续扩展 Input/Discrete，必须 bump `specVersion=v2`（避免语义漂移）。
- 该 stub 结构是为后续接入 plc_core 的“接口适配层”准备，未来应以“只增可选字段”为演进策略。

