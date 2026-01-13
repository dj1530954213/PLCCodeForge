# TASK-37-result.md（联合表 × 通讯采集 Stub → UnifiedImport v1 + merge_report）

## 1) 完成摘要

- 新增合并 command：`comm_merge_import_sources_v1`，把 **联合 xlsx（工程设计源）** 与 **ImportResultStub v1（通讯采集核对源）** 合并为 `UnifiedImport v1`。
- 合并规则拍板并固化：
  - 主集合以 union xlsx 为准（权威设计源）
  - stub 中同名点位覆盖 `comm + verification`
  - stub 中 unmatched 点位不并入 points，仅输出 warnings
  - union xlsx 内 `points.name(HMI)` 重复 => fail-fast（避免歧义合并）
- 输出两个落盘产物（同一时间戳目录下）：
  - `outputDir/unified_import/unified_import.v1.<ts>.json`
  - `outputDir/unified_import/merge_report.v1.<ts>.json`
- evidence `manifest.json` 增补：`outputs.unifiedImportPath/unifiedImportDigest` 与 `outputs.mergeReportPath/mergeReportDigest`（sha256），并支持可选拷贝进 evidence 目录/zip。
- ImportUnion 页面新增入口：点击 `合并生成 UnifiedImport（v1）` 可生成并展示 outPath + reportPath。

## 2) 改动清单（文件路径 + 关键点）

- `Tauri.CommMapping/src-tauri/src/comm/merge_unified_import.rs`
  - 定义冻结 `UnifiedImportV1` / `MergeReportV1` / `MergeImportSourcesSummary`。
  - `merge_import_sources_v1(...)`：strict 解析 union xlsx → 校验 name 唯一 → 合并 stub → 写 unified + report（原子写）。
  - 单测：`merge_applies_stub_comm_and_verification_for_matched_points` / `merge_keeps_union_as_authoritative_and_reports_unmatched_stub_points_as_warning`。
- `Tauri.CommMapping/src-tauri/src/comm/error.rs`
  - 新增结构化错误：`MergeImportSourcesError{kind,message,details}`。
- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
  - 新增 DTO + command：`comm_merge_import_sources_v1`（spawn_blocking，永不 reject）。
  - evidence：`CommEvidencePackRequest` 新增 `unifiedImportPath/mergeReportPath` 与 copy 开关；manifest.outputs 增加 digest/relPath 字段。
- `Tauri.CommMapping/src-tauri/src/lib.rs`
  - 注册 command：`comm_merge_import_sources_v1`。
  - 增加 `#![recursion_limit = \"256\"]`（解决 manifest json! 宏展开递归限制）。
- `Tauri.CommMapping/src-tauri/src/comm/path_resolver.rs`
  - 已包含默认路径：`default_unified_import_path` / `default_merge_report_path`（用于本任务落盘目录策略）。
- `src/comm/api.ts`
  - 新增 `commMergeImportSourcesV1` 与 TS 类型（MergeImportSourcesError/Summary/Response）。
- `src/comm/pages/ImportUnion.vue`
  - 新增按钮：`合并生成 UnifiedImport（v1）`；展示 outPath/reportPath + warnings。
- `src/comm/services/evidencePack.ts`
  - 透传 `unifiedImportPath/mergeReportPath` 到 `commEvidencePackCreate`。

## 3) build/test 证据

### 3.1 cargo build

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s
```

### 3.2 cargo test（包含新增 merge 单测）

```text
running 39 tests
...
test comm::merge_unified_import::tests::merge_keeps_union_as_authoritative_and_reports_unmatched_stub_points_as_warning ... ok
test comm::merge_unified_import::tests::merge_applies_stub_comm_and_verification_for_matched_points ... ok
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

## 4) 示例 JSON（合并请求/响应 + UnifiedImport/merge_report）

### 4.1 invoke：comm_merge_import_sources_v1（request/response）

request（示例）：

```json
{
  "request": {
    "unionXlsxPath": "C:\\temp\\联合点表.xlsx",
    "importResultStubPath": "C:\\temp\\import_result_stub.v1.json",
    "outPath": ""
  }
}
```

response（示例，ok=true）：

```json
{
  "outPath": "C:\\...\\deliveries\\unified_import\\unified_import.v1.20260103T...json",
  "reportPath": "C:\\...\\deliveries\\unified_import\\merge_report.v1.20260103T...json",
  "summary": { "matched": 10, "unmatchedStub": 1, "unifiedImportDigest": "sha256:..." },
  "warnings": [{ "code": "UNMATCHED_STUB_POINT", "hmiName": "EXTRA" }],
  "ok": true
}
```

### 4.2 UnifiedImport v1 片段（来自单测生成）

文件：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-merge-out-6c2a193f-a671-4713-af43-f7fba609d6ab\unified_import.v1.json`

```json
{
  "schemaVersion": 1,
  "specVersion": "v1",
  "points": [
    {
      "name": "P1",
      "design": { "channelName": "tcp-1", "protocolType": "TCP" },
      "comm": { "channelName": "tcp-1", "addressSpec": { "readArea": "Holding" } },
      "verification": { "quality": "Ok", "valueDisplay": "123" }
    }
  ],
  "deviceGroups": [],
  "hardware": {},
  "statistics": { "matched": 1, "unmatchedStub": 1 }
}
```

### 4.3 merge_report v1 片段（来自单测生成）

文件：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-merge-out-6c2a193f-a671-4713-af43-f7fba609d6ab\merge_report.v1.json`

```json
{
  "matchedCount": 1,
  "unmatchedStubPoints": ["EXTRA"],
  "overriddenCount": 1,
  "conflicts": []
}
```

## 5) manifest.json outputs 片段（unifiedImport + mergeReport）

文件（来自单测生成 evidence pack）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-evidence-ae09572e-e535-4a0e-8b5d-df26b2f931cf\deliveries\evidence\20260103T153924Z-1767454764437\manifest.json`

```json
{
  "unifiedImportPath": "C:\\...\\deliveries\\unified_import\\unified_import.v1.test.json",
  "unifiedImportDigest": "sha256:a85013b01ae33035249dcea5268fb7adf72db9019d2b8c10c8a67a1a791ec0ec",
  "mergeReportPath": "C:\\...\\deliveries\\unified_import\\merge_report.v1.test.json",
  "mergeReportDigest": "sha256:d112654ce6821815a7df0ff353f8d31a4ef927f6fc17969ddab7fefab0317c48"
}
```

## 6) UI 操作步骤（ImportUnion 页）

1) 运行 `一键演示（Wizard）` → 导出 IR
2) 点击 `导出 PLC Bridge（v1）`
3) 点击 `导出 ImportResultStub（v1）`
4) 点击 `合并生成 UnifiedImport（v1）` → 页面显示 `UnifiedImport 已生成：... | report=...`

## 7) 自检清单（逐条勾选）

- [x] union xlsx 为主集合；stub unmatched 不并入 points（仅 warnings）
- [x] union xlsx 内 points.name 重复 => fail-fast（避免歧义）
- [x] 输出 `UnifiedImport v1` + `merge_report v1`，路径在 `outputDir/unified_import/`
- [x] command 全部 `spawn_blocking`（不阻塞 UI）
- [x] evidence manifest.outputs 增加 `unifiedImportPath/mergeReportPath` + sha256 digest（可拷贝入 evidence）
- [x] 不修改既有冻结 DTO/headers（仅新增可选字段/新增命令）

## 8) 风险与未决项

- 当前 `UnifiedImport.points[].design` 仅为 MVP 最小集（deviceGroups/hardware 仍为空占位）；后续补齐联合表解析字段时需要 **只增可选字段** 或 bump `specVersion=v2`。
- 合并主键按 HMI 变量名称（name）唯一：若现场存在同名点位设计，将需要在联合表侧先做规范化（或未来 v2 引入更强主键）。

