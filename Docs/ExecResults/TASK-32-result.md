# TASK-32-result.md（统一交付目录 outputDir：XLSX / IR / evidence 同一出口）

## 1) 完成摘要

- 新增 `outputDir` 交付目录配置（落盘到 `AppData/<app>/comm/config.v1.json`，schemaVersion=1）。
- 导出路径策略兼容旧行为：
  - 用户手填 `outPath`：仍按指定路径导出（不破坏历史语义）。
  - `outPath` 留空：自动导出到 `outputDir/xlsx/通讯地址表.<ts>.xlsx`。
- IR 与 evidence 输出统一到 `outputDir`：
  - `outputDir/ir/comm_ir.v1.<ts>.json`
  - `outputDir/evidence/<ts>/.../evidence.zip`
- evidence `manifest.json` 的 `outputs` 增补 `outputDir` 与相对路径字段（尽量相对 outputDir 记录）。

## 2) 改动清单（文件路径 + 关键点）

- `src-tauri/src/comm/storage.rs`
  - 新增 `config.v1.json`：`CommConfigV1{schemaVersion, outputDir}` + `save_config/load_config` + `default_output_dir()`
- `src-tauri/src/comm/path_resolver.rs`
  - 集中封装：`resolve_output_dir` / `default_delivery_xlsx_path` / `evidence_dir` / `ir_dir` / `rel_if_under`
- `src-tauri/src/comm/tauri_api.rs`
  - 新增 commands：`comm_config_load` / `comm_config_save`
  - `comm_export_xlsx`：改为 async + spawn_blocking；`outPath` 允许留空走 outputDir 默认路径
  - `comm_export_delivery_xlsx`：`outPath` 允许留空走 outputDir 默认路径
  - `comm_export_ir_v1`：输出到 `outputDir/ir/`
  - `comm_evidence_pack_create`：输出到 `outputDir/evidence/`；manifest.outputs 增补 outputDir/相对路径
- `src/comm/api.ts`
  - 新增 `commConfigLoad/commConfigSave` 与 `CommConfigV1` 类型
- `src/comm/pages/ImportUnion.vue`
  - 增加 outputDir 配置入口（文本输入+保存）；Wizard 导出路径可留空
- `src/comm/pages/Export.vue`
  - 导出路径可留空（后端按 outputDir 自动生成）
- `Docs/Runbook/通讯采集一键演示.md`
  - 更新：outputDir/IR/evidence 的最新路径策略与 UI 步骤

## 3) build/test 证据

### 3.1 pnpm build

```text
> plc-code-forge@0.1.0 build C:\Program Files\Git\code\PLCCodeForge
> vue-tsc --noEmit && vite build

vite v6.4.1 building for production...
✓ 1458 modules transformed.
✓ built in 3.90s
```

### 3.2 cargo test

```text
Running unittests src\lib.rs (...\tauri_app_lib-....exe)
running 30 tests
...
test comm::path_resolver::tests::resolve_output_dir_defaults_to_deliveries_when_missing_config ... ok
test comm::path_resolver::tests::resolve_output_dir_uses_config_absolute_or_relative ... ok
test comm::tauri_api::tests::evidence_pack_manifest_includes_ir_path_and_digest_and_output_dir ... ok
...
test result: ok. 30 passed; 0 failed
```

## 4) config.v1.json 示例

```json
{
  "schemaVersion": 1,
  "outputDir": "C:\\Users\\DELL\\AppData\\Local\\Temp\\plc-codeforge-evidence-70e2016f-1dae-4f76-82e4-579f77bbee9b\\deliveries"
}
```

## 5) 目录树示例（outPath 留空时）

```text
outputDir/
  evidence/
    20260103T131104Z-1767445864403/
      通讯地址表.test.xlsx
      comm_ir.v1.test.json
      evidence.zip
      export_response.json
      manifest.json
      pipeline_log.json
  ir/
    comm_ir.v1.test.json
  xlsx/
    通讯地址表.test.xlsx
```

## 6) manifest.json 片段（outputs.outputDir + 相对路径）

```json
{
  "outputs": {
    "outputDir": "C:\\Users\\DELL\\AppData\\Local\\Temp\\plc-codeforge-evidence-70e2016f-1dae-4f76-82e4-579f77bbee9b\\deliveries",
    "xlsxPathRel": "xlsx\\通讯地址表.test.xlsx",
    "irPathRel": "ir\\comm_ir.v1.test.json",
    "evidenceZipPathRel": "evidence\\20260103T131104Z-1767445864403\\evidence.zip"
  }
}
```

## 7) 自检清单（逐条勾选）

- [x] `config.v1.json` 顶层 schemaVersion=1，字段含 outputDir
- [x] outPath 手填仍生效（兼容旧流程）
- [x] outPath 留空时自动导出到 outputDir/xlsx（包含时间戳）
- [x] IR 与 evidence 输出在 outputDir/ir 与 outputDir/evidence
- [x] 所有文件 IO 仍在 spawn_blocking（不阻塞 UI）
- [x] manifest.outputs 记录 outputDir + 相对路径字段（尽量相对）

## 8) 风险与未决项

- outputDir 若配置到无权限目录，会导致导出失败；当前会以结构化错误/日志体现（需现场选择可写目录）。
- 若未来三模块合并需要更严格的“相对路径必填”口径，建议在 manifest 里固定字段并做测试快照锁定。
