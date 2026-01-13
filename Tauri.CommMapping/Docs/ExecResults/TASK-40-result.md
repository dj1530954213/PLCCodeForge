# TASK-40-result.md（Evidence Pack 增强：全量打包 + verify 命令 + 回归 Runbook）

## 1) 完成摘要

- Evidence Pack 增强：支持把 `unified_import/merge_report/plc_import_stub` 等关键产物拷贝进 evidence 目录，并生成 `evidence_summary.v1.json`（用于签收/回归摘要）。
- 新增回归校验命令：`comm_evidence_verify_v1(path)`（支持 evidence 目录或 `evidence.zip`），返回结构化 `checks/errors`，用于现场快速定位：缺文件 / digest mismatch / schema mismatch / points 顺序不一致。
- 前端（ImportUnion）增加稳定展示入口：Evidence Verify（v1）区块可直接展示校验结果对象。
- 新增 Runbook：`Tauri.CommMapping/Docs/Runbook/evidence_pack_v1.md`（生成/验证/交付复现）。

## 2) 改动清单（文件路径 + 关键点）

- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
  - 增强 `comm_evidence_pack_create`：生成 `evidence_summary.v1.json` 并纳入 zip；manifest.inputs 增补 `unionXlsxPath/unionXlsxDigest/parsedColumnsUsed`。
  - 新增 command：`comm_evidence_verify_v1`（spawn_blocking，支持 dir/zip；结构化 errors）。
  - 新增单测：`evidence_verify_v1_reports_ok_and_detects_digest_mismatch_after_tamper`（含 dir/zip 校验与篡改检测）。
- `Tauri.CommMapping/src-tauri/src/lib.rs`
  - 注册 command：`comm_evidence_verify_v1`。
- `src/comm/api.ts`
  - TS 类型新增：`CommEvidenceVerifyV1Response/EvidenceVerify*`。
  - 新增封装：`commEvidenceVerifyV1(path)`；`CommEvidencePackRequest` 增补 `unionXlsxPath/parsedColumnsUsed`。
- `src/comm/services/evidencePack.ts`
  - evidence pack 请求透传 `unionXlsxPath/parsedColumnsUsed`。
- `src/comm/pages/ImportUnion.vue`
  - 证据包导出后自动填充 `evidenceVerifyPath`。
  - 新增 UI 区块：Evidence Verify (v1)（展示 `checks/errors`）。
- `Tauri.CommMapping/Docs/Runbook/evidence_pack_v1.md`
  - 新增：证据包生成/验证/交付复现说明。

## 3) build/test 证据

### 3.1 cargo build

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.54s
```

### 3.2 cargo test（新增 verify 单测）

```text
running 41 tests
...
test comm::tauri_api::tests::evidence_verify_v1_reports_ok_and_detects_digest_mismatch_after_tamper ... ok
test result: ok. 41 passed; 0 failed
```

### 3.3 pnpm build（UI 有改动）

```text
> plc-code-forge@0.1.0 build C:\Program Files\Git\code\PLCCodeForge
> vue-tsc --noEmit && vite build
...
✓ built in 3.63s
```

## 4) evidence 目录结构树（示例）

```text
outputDir/evidence/<ts>/
  manifest.json
  evidence_summary.v1.json
  pipeline_log.json
  export_response.json
  conflict_report.json               (optional)
  通讯地址表.<ts>.xlsx                 (copied, optional)
  comm_ir.v1.<ts>.json               (copied, optional)
  unified_import.v1.<ts>.json        (copied, optional)
  merge_report.v1.<ts>.json          (copied, optional)
  plc_import.v1.<ts>.json            (copied, optional)
  evidence.zip                       (optional)
```

## 5) comm_evidence_verify_v1 成功输出片段（dir/zip）

（来自单测 `cargo test ... -- --nocapture` 打印，展示结构化 checks）

```json
{
  "ok": true,
  "checks": [
    { "name": "manifest.json:present", "ok": true },
    { "name": "evidence_summary.v1.json:schema", "ok": true },
    { "name": "unifiedImport:digest", "ok": true },
    { "name": "plcImportStub:digest", "ok": true },
    { "name": "pointsOrder:unified_vs_plc_stub", "ok": true }
  ],
  "errors": []
}
```

## 6) 篡改后 verify 失败输出片段（digest mismatch）

```json
{
  "ok": false,
  "errors": [
    {
      "kind": "DigestMismatch",
      "message": "unifiedImport digest mismatch",
      "details": {
        "fileName": "unified_import.v1.test.json",
        "expected": "sha256:0c3a837e8810fc6666c15c37e685560c2cd57d394ad5492bfd5f347c4e2c7a78",
        "actual": "sha256:261c6ae621161c881ff223bbc1e5e15d331c6cdd3a6c729c8b3ae4a209eac197"
      }
    }
  ]
}
```

## 7) Runbook（生成/验证步骤）

文件：`Tauri.CommMapping/Docs/Runbook/evidence_pack_v1.md`

关键步骤（摘录）：

```text
ImportUnion 页面：
  1) 一键演示（Wizard）
  2) 导出证据包（生成 evidenceDir / evidence.zip）
  3) Evidence Verify (v1) 输入路径 -> Verify -> ok=true
```

## 8) 自检清单（逐条勾选）

- [x] evidence 目录生成 `evidence_summary.v1.json`，并打入 zip
- [x] `comm_evidence_verify_v1` 返回结构化 `checks/errors`（不依赖“字符串 JSON”约定）
- [x] verify 支持 dir 与 zip 两种输入
- [x] IO/解压在 `spawn_blocking` 执行（不阻塞 UI）
- [x] 前端可稳定展示错误对象（kind/message/details）

## 9) 风险与未决项

- 当前 verify 主要校验 JSON 产物的 sha256 digest；XLSX 本体未做 digest 校验（仍保留 `headersDigest` 用于冻结 headers 对齐）。
- zip 目前采用“仅写入 basename”的策略；若未来 zip 结构调整，需要同步更新 verify 的 entry 查找规则。
