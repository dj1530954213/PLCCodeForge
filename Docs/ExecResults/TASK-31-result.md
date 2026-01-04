# TASK-31-result.md（导出统一中间数据 IR：CommIR v1 + 纳入 evidence manifest）

## 1) 完成摘要

- 新增后端命令：`comm_export_ir_v1`，导出 **CommIR v1** 到磁盘 JSON（冻结字段，仅允许新增可选字段）。
- Wizard 流水线在 `export` 后自动执行 `comm_export_ir_v1`；即使流水线失败也会 best-effort 导出 IR（保证可追溯产物存在）。
- evidence 证据包的 `manifest.json` 增补：`outputs.irPath` + `outputs.irDigest`（sha256），并可选将 IR 拷贝进 evidence 目录/zip。
- ImportUnion 页面展示 `irPath`，并提供 `打开 IR` 入口。

## 2) 改动清单（文件路径 + 关键点）

- `src-tauri/src/comm/export_ir.rs`
  - 定义冻结的 `CommIrV1` schema（schemaVersion=1/specVersion=v1），包含 sources/mapping/verification/decisionsSummary/conflicts。
  - `export_comm_ir_v1(...) -> { irPath, irDigest, summary }`：生成 JSON + sha256 + 原子写入文件。
- `src-tauri/src/comm/tauri_api.rs`
  - 新增 command：`comm_export_ir_v1(request)`（spawn_blocking，不阻塞 UI）。
  - evidence：`CommEvidencePackRequest` 新增可选 `irPath/copyIr`；manifest.outputs 增加 `irPath/irDigest`。
- `src-tauri/src/lib.rs`
  - 注册 commands：`comm_export_ir_v1`。
- `src/comm/api.ts`
  - 新增 `commExportIrV1` 调用与 TS 类型（CommIrResultsSource / CommIrExportSummary / CommExportIrV1Request/Response）。
  - evidence request 类型新增 `irPath/copyIr`。
- `src/comm/services/demoPipeline.ts`
  - Wizard 在 `export` 后自动调用 `commExportIrV1`；失败场景也会 finally best-effort 导出 IR。
- `src/comm/services/evidencePack.ts`
  - 透传 `irPath` 到 `commEvidencePackCreate`，默认 `copyIr=true`。
- `src/comm/pages/ImportUnion.vue`
  - Wizard 结束后展示 `CommIR 已导出：irPath`，并提供 `打开 IR` 按钮。

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
test comm::export_ir::tests::export_ir_v1_writes_file_and_contains_required_fields ... ok
test comm::tauri_api::tests::evidence_pack_manifest_includes_ir_path_and_digest_and_output_dir ... ok
...
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

Running tests\comm_it.rs (...\comm_it-....exe)
running 2 tests
test rtu_quality_ok_for_one_point_when_enabled ... ok
test tcp_quality_ok_for_two_points_when_enabled ... ok
test result: ok. 2 passed; 0 failed
```

## 4) CommIR v1 样例片段（字段冻结）

示例来自单测运行时生成的文件：
`C:\\Users\\DELL\\AppData\\Local\\Temp\\plc-codeforge-ir-0499c468-7aff-4ead-a890-2aca6c3911f2\\ir\\comm_ir.v1.20260103T131104Z-1767445864397.json`

```json
{
  "schemaVersion": 1,
  "specVersion": "v1",
  "generatedAtUtc": "2026-01-03T13:11:04.397633700Z",
  "sources": {
    "unionXlsxPath": "C:\\temp\\union.xlsx",
    "resultsSource": "runLatest"
  },
  "mapping": {
    "points": [
      {
        "pointKey": "00000000-0000-0000-0000-000000000001",
        "hmiName": "P1",
        "channelName": "ch1",
        "dataType": "UInt16",
        "endian": "ABCD",
        "scale": 1.0,
        "rw": "R",
        "addressSpec": {
          "readArea": "Holding",
          "absoluteAddress": 102,
          "unitLength": 1,
          "profileStartAddress": 100,
          "profileLength": 10,
          "offsetFromProfileStart": 2,
          "jobStartAddress": 102,
          "jobLength": 1,
          "addressBase": "zero"
        }
      }
    ],
    "profiles": [
      {
        "protocolType": "TCP",
        "channelName": "ch1",
        "deviceId": 1,
        "readArea": "Holding",
        "startAddress": 100,
        "length": 10,
        "ip": "127.0.0.1",
        "port": 502,
        "timeoutMs": 1000,
        "retryCount": 0,
        "pollIntervalMs": 500
      }
    ]
  },
  "verification": {
    "results": [
      {
        "pointKey": "00000000-0000-0000-0000-000000000001",
        "valueDisplay": "123",
        "quality": "Ok",
        "timestamp": "2026-01-03T13:11:04.397626900Z",
        "durationMs": 1,
        "message": ""
      }
    ],
    "stats": { "total": 1, "ok": 1, "timeout": 0, "commError": 0, "decodeError": 0, "configError": 0 }
  }
}
```

## 5) evidence manifest 片段（outputs.irPath + irDigest）

示例来自单测运行时生成的文件：
`C:\\Users\\DELL\\AppData\\Local\\Temp\\plc-codeforge-evidence-70e2016f-1dae-4f76-82e4-579f77bbee9b\\deliveries\\evidence\\20260103T131104Z-1767445864403\\manifest.json`

```json
{
  "outputs": {
    "outputDir": "C:\\Users\\DELL\\AppData\\Local\\Temp\\plc-codeforge-evidence-70e2016f-1dae-4f76-82e4-579f77bbee9b\\deliveries",
    "irPath": "C:\\Users\\DELL\\AppData\\Local\\Temp\\plc-codeforge-evidence-70e2016f-1dae-4f76-82e4-579f77bbee9b\\deliveries\\ir\\comm_ir.v1.test.json",
    "irDigest": "sha256:17f0e4e48f3e80360f1a0dc75bbee4116ea3cb973ef3d18309d6460105ef8d8c",
    "headersDigest": "sha256:d20dc8fd2298cca0b0868a92d7d6ed0944b646d4bd1f8b4fd1d27b110118e22f"
  }
}
```

## 6) 自检清单（逐条勾选）

- [x] CommIR v1 字段冻结说明已写入代码注释与本结果文件（后续仅允许新增可选字段）
- [x] Wizard 成功/失败都会 best-effort 导出 IR（不 silent fail）
- [x] IR 写文件在后端 `spawn_blocking`（不阻塞 UI）
- [x] evidence manifest 输出包含 `outputs.irPath` 与 `outputs.irDigest`
- [x] UI 可观测：ImportUnion 页面可看到 `irPath` 并可点击打开

## 7) 风险与未决项

- 当前 `mapping.profiles` 为脱敏快照（无密码字段）；若未来引入敏感字段，需要在 IR/manifest 中明确脱敏策略（只能新增字段）。
- IR 的 addressSpec 以内部 0-based 为准；若后续需支持 1-based/40001 风格，必须 bump specVersion=v2。
