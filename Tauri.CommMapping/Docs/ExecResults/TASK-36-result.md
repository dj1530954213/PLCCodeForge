# TASK-36-result.md（Bridge → ImportResultStub v1 + 最小验证器 + manifest 对齐）

## 1) 完成摘要

- 新增后端导出：`comm_bridge_export_importresult_stub_v1`，把 `PlcImportBridge v1` 转换为 **ImportResultStub v1 JSON**（不接入 plc_core、不渲染模板、不生成程序）。
- Stub 产物默认落盘：`outputDir/bridge_importresult_stub/import_result_stub.v1.<ts>.json`。
- 增加最小验证器（fail-fast）：points.name 唯一、name/channelName 非空、schema/spec 严格匹配。
- evidence `manifest.json` 增补：`outputs.importResultStubPath` / `outputs.importResultStubDigest`（sha256），并支持可选拷贝进 evidence 目录/zip。
- ImportUnion 页面新增按钮：`导出 ImportResultStub（v1）`，并展示/打开 outPath。

## 2) 改动清单（文件路径 + 关键点）

- `Tauri.CommMapping/src-tauri/src/comm/bridge_importresult_stub.rs`
  - 定义并冻结 `ImportResultStubV1` schema（schemaVersion=1/specVersion=v1）。
  - `export_import_result_stub_v1(bridgePath,outPath)`：读取 bridge → 校验 → 写 stub → 返回 digest/summary。
  - 新增单测：`export_stub_writes_file_and_has_required_sections` / `export_stub_fails_on_duplicate_name`。
- `Tauri.CommMapping/src-tauri/src/comm/error.rs`
  - 新增结构化错误：`ImportResultStubError{kind,message,details}`。
- `Tauri.CommMapping/src-tauri/src/comm/path_resolver.rs`
  - 新增默认路径：`default_importresult_stub_path`（输出到 `outputDir/bridge_importresult_stub/`）。
- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
  - 新增 command：`comm_bridge_export_importresult_stub_v1`（spawn_blocking，永不 reject）。
  - evidence：`CommEvidencePackRequest` 新增可选 `importResultStubPath/copyImportResultStub`；manifest.outputs 增加 `importResultStubPath/importResultStubDigest` 等字段。
- `Tauri.CommMapping/src-tauri/src/lib.rs`
  - 注册 commands：`comm_bridge_export_importresult_stub_v1`。
- `src/comm/api.ts`
  - 新增 `commBridgeExportImportResultStubV1` + TS 类型（含结构化错误 ImportResultStubError）。
  - evidence request 增加 `importResultStubPath/copyImportResultStub`。
- `src/comm/services/evidencePack.ts`
  - 透传 `importResultStubPath` 到 `commEvidencePackCreate`。
- `src/comm/pages/ImportUnion.vue`
  - 新增按钮/展示：`导出 ImportResultStub（v1）` 并展示 outPath（可打开）。

## 3) build/test 证据

### 3.1 cargo test

```text
running 36 tests
...
test comm::bridge_importresult_stub::tests::export_stub_writes_file_and_has_required_sections ... ok
test comm::bridge_importresult_stub::tests::export_stub_fails_on_duplicate_name ... ok
...
test result: ok. 36 passed; 0 failed
```

### 3.2 pnpm build（UI 有改动）

```text
> plc-code-forge@0.1.0 build C:\Program Files\Git\code\PLCCodeForge
> vue-tsc --noEmit && vite build

vite v6.4.1 building for production...
✓ 1458 modules transformed.
✓ built in 4.64s
```

## 4) ImportResultStub v1 样例片段（points/deviceGroups/hardware/statistics）

样例文件（来自单测生成）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-stub-d1f5b800-3370-412d-bb34-6df382cb2031\stub\import_result_stub.v1.test.json`

```json
{
  "schemaVersion": 1,
  "specVersion": "v1",
  "sourceBridgePath": "C:\\Users\\DELL\\AppData\\Local\\Temp\\...\\plc_import_bridge.v1.test.json",
  "sourceBridgeDigest": "sha256:323b04c87166def30381ee9989c39d10a427097bbef8e97e1e0464646d22fd1e",
  "points": [{ "name": "P1", "comm": { "channelName": "ch1" }, "verification": { "quality": "Ok" } }],
  "deviceGroups": [],
  "hardware": {},
  "statistics": { "total": 1, "ok": 1, "timeout": 0, "commError": 0, "decodeError": 0, "configError": 0 }
}
```

## 5) manifest.json outputs 片段（importResultStubPath + digest）

样例文件（来自单测生成）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-evidence-18c4d079-9663-44ca-8820-e830ed0726be\deliveries\evidence\20260103T144409Z-1767451449461\manifest.json`

```json
{
  "outputs": {
    "importResultStubPath": "C:\\Users\\DELL\\AppData\\Local\\Temp\\...\\deliveries\\bridge_importresult_stub\\import_result_stub.v1.test.json",
    "importResultStubDigest": "sha256:a85013b01ae33035249dcea5268fb7adf72db9019d2b8c10c8a67a1a791ec0ec"
  }
}
```

## 6) 校验失败示例（duplicate name → 结构化错误）

样例文件（来自单测生成）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-stub-dup-0cc64d62-0cc4-4770-9857-7648679a5b49\stub_error.json`

```json
{
  "kind": "ImportResultStubValidationError",
  "message": "duplicate points.name detected",
  "details": { "name": "DUP", "field": "points.name" }
}
```

## 7) UI 操作步骤（ImportUnion 页）

1) 先运行 `一键演示（Wizard）` → 导出 IR
2) 点击 `导出 PLC Bridge（v1）`
3) 点击 `导出 ImportResultStub（v1）` → 页面提示 outPath，可点击打开
4) （可选）点击 `导出证据包`，manifest.outputs 会包含 importResultStubPath/importResultStubDigest

## 8) 自检清单（逐条勾选）

- [x] 不接入 plc_core orchestrate / 不渲染模板 / 不生成程序
- [x] 默认输出到 `outputDir/bridge_importresult_stub/`
- [x] fail-fast：points.name 唯一、字段非空、schema/spec 严格
- [x] evidence manifest.outputs 增加 `importResultStubPath/importResultStubDigest`
- [x] command 文件 IO 走 `spawn_blocking`（不阻塞 UI）

## 9) 风险与未决项

- 当前 `deviceGroups/hardware` 为占位空结构；后续与“联合表/硬件信息”合并时只能新增可选字段或 bump v2，避免破坏 v1 消费方。

