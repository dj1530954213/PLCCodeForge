# TASK-29-result.md（证据包标准化：manifest.json + 版本指纹 + 运行配置快照）

## 1) 完成摘要

- evidence 证据包新增 `manifest.json`（可追溯签收单），包含 app 标识/版本/gitCommit、schemaVersion 指纹、run 配置快照、counts、headersDigest 等。
- 后端在 `comm_evidence_pack_create` 内生成并写入 `manifest.json`，并确保打包到 `evidence.zip`（spawn_blocking 不阻塞 UI）。
- 前端导出证据包后展示 `manifest 摘要`（版本/点位数/conflicts/resultsStatus 等），便于现场回传与验收。

## 2) 改动清单（文件路径 + 关键点）

- `Tauri.CommMapping/src-tauri/build.rs`
  - 通过 `git rev-parse HEAD` 注入 `GIT_COMMIT`（取不到则 manifest 使用 `"unknown"` 兜底）。
- `Tauri.CommMapping/src-tauri/Cargo.toml`
  - 新增：`sha2 = "0.10"`（headersDigest 计算）。
- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
  - `CommEvidencePackRequest`：新增可选 `meta`（前端传入 run/counts/connectionSnapshot 等快照）。
  - `CommEvidencePackResponse`：新增 `manifest` 字段（返回给前端直接展示）。
  - `comm_evidence_pack_create`：生成 `manifest.json` + `headersDigest`（sha256）并写入 evidence 目录；`evidence.zip` 仅打包一次且包含 manifest。
- `src/comm/api.ts`
  - 同步 `CommEvidencePackRequest.meta?` 与 `CommEvidencePackResponse.manifest` 类型。
- `src/comm/services/evidencePack.ts`
  - 调用 `commEvidencePackCreate` 时透传 `meta`；返回值包含 `manifest`。
- `src/comm/pages/ImportUnion.vue`
  - 导出证据包后展示 `manifest` 摘要与 raw JSON（用于验收与回传）。

## 3) build/test 证据

### 3.1 cargo build（待本机执行后补贴输出片段）

```text
> cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
   Compiling sha2 v0.10.9
   Compiling tauri-app v0.1.0 (C:\Program Files\Git\code\PLCCodeForge\Tauri.CommMapping\src-tauri)
   Compiling tauri-codegen v2.5.2
   Compiling tauri-macros v2.5.2
   Compiling tauri v2.9.5
   Compiling tauri-plugin-opener v2.5.2
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.73s
```

### 3.2 cargo test（待本机执行后补贴输出片段）

```text
> cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
running 26 tests
test comm::export_xlsx::tests::export_xlsx_emits_warnings_without_changing_frozen_headers ... ok
test comm::tauri_api::tests::import_union_strict_missing_sheet_returns_structured_error_object ... ok
test comm::union_spec_v1::tests::spec_v1_required_columns_snapshot ... ok
test comm::engine::tests::run_engine_stop_within_1s_and_latest_is_ordered_by_points ... ok
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

running 2 tests
test rtu_quality_ok_for_one_point_when_enabled ... ok
test tcp_quality_ok_for_two_points_when_enabled ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 3.3 pnpm build（前端变更涉及 evidence UI/TS 类型）

```text
> pnpm build
> vue-tsc --noEmit && vite build
vite v6.4.1 building for production...
✓ built in 3.44s
```

## 4) 示例 JSON（request/response）

### 4.1 comm_evidence_pack_create request（示例：包含 meta）

```json
{
  "pipelineLog": [
    { "tsUtc": "2026-01-03T10:00:00.000Z", "step": "import", "status": "ok", "message": "..." }
  ],
  "exportResponse": {
    "outPath": "C:\\\\temp\\\\通讯地址表.xlsx",
    "headers": { "tcp": ["..."], "rtu": ["..."], "params": ["..."] },
    "resultsStatus": "written"
  },
  "meta": {
    "run": { "driver": "mock", "includeResults": true, "resultsSource": "runLatest", "durationMs": 5123 },
    "counts": { "profiles": 2, "points": 10, "results": 10, "decisions": { "reusedKeyV2": 1, "reusedKeyV2NoDevice": 0, "reusedKeyV1": 2, "createdNew": 7 }, "conflicts": 0 },
    "connectionSnapshot": null
  },
  "exportedXlsxPath": "C:\\\\temp\\\\通讯地址表.xlsx",
  "copyXlsx": true,
  "zip": true
}
```

### 4.2 comm_evidence_pack_create response（示例：manifest 作为对象返回）

```json
{
  "evidenceDir": "C:\\\\Users\\\\...\\\\AppData\\\\Roaming\\\\<app>\\\\comm\\\\evidence\\\\20260103T100000Z-...",
  "zipPath": "C:\\\\Users\\\\...\\\\evidence.zip",
  "manifest": {
    "createdAtUtc": "2026-01-03T10:00:00+00:00",
    "app": { "appName": "com.example.app", "appVersion": "0.1.0", "gitCommit": "unknown" },
    "schema": { "profilesSchemaVersion": 1, "pointsSchemaVersion": 1, "resultsSchemaVersion": 1 },
    "run": { "driver": "mock", "includeResults": true, "resultsSource": "runLatest", "durationMs": 5123 },
    "counts": { "profiles": 2, "points": 10, "results": 10, "decisions": { "reused:keyV2": 1, "reused:keyV2NoDevice": 0, "reused:keyV1": 2, "created:new": 7 }, "conflicts": 0 },
    "outputs": { "evidenceZipPath": "...\\\\evidence.zip", "copiedXlsxPath": "...\\\\通讯地址表.xlsx", "headersDigest": "sha256:..." },
    "connectionSnapshot": null,
    "itFlags": { "COMM_IT_ENABLE": null, "COMM_IT_TCP_HOST": null }
  },
  "files": ["export_response.json", "manifest.json", "pipeline_log.json", "evidence.zip"],
  "warnings": []
}
```

## 5) 自检清单（逐条勾选）

- [x] `manifest.json` 字段齐全：createdAtUtc/app/schema/run/counts/outputs（字段固定存在）
- [x] gitCommit 取不到时仍有字段（"unknown"）
- [x] headersDigest 基于三张冻结 headers 做 hash（仅校验，不改列）
- [x] `comm_evidence_pack_create` 使用 `spawn_blocking`（不阻塞 UI）
- [x] 返回结构为对象字段（前端无需 parse JSON 字符串）

## 6) 风险与未决项

- 打包环境可能没有 git：manifest.gitCommit 将为 `"unknown"`（已兜底）。
- manifest.outputs.evidenceZipPath 为期望路径；若 zip 生成失败，以 response.warnings/zipPath=null 为准。
