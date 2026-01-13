# TASK-34-result.md（bridge-consumer check：验证 PlcImportBridge v1 可被消费）

## 1) 完成摘要

- 新增最小 “bridge consumer check” 能力（不接入 plc_core orchestrate）：读取 `PlcImportBridge v1` → 生成 `summary.json`，验证数据格式/字段/路径策略贯通。
- 新增 Tauri command：`comm_bridge_consume_check(bridgePath)`：输出到 `outputDir/bridge_check/<ts>/summary.json`，并返回 summary（便于 UI/脚本消费）。
- ImportUnion 页面增加按钮：`运行 bridge consumer check`，可直接生成并打开 summary.json。

## 2) 改动清单（文件路径 + 关键点）

- `Tauri.CommMapping/src-tauri/src/comm/bridge_plc_import.rs`
  - `consume_bridge_and_write_summary(bridgePath,outDir)`：生成 summary.json（totalPoints/byChannel/byQuality/first10）。
  - 新增单测：`consume_bridge_writes_summary_json`。
- `Tauri.CommMapping/src-tauri/src/comm/error.rs`
  - 新增结构化错误：`BridgeCheckError{kind,message,details}`。
- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
  - 新增 command：`comm_bridge_consume_check`（spawn_blocking）。
- `Tauri.CommMapping/src-tauri/src/lib.rs`
  - 注册 commands：`comm_bridge_consume_check`。
- `src/comm/api.ts`
  - 新增 `commBridgeConsumeCheck` + TS 类型（结构化错误 BridgeCheckError）。
- `src/comm/pages/ImportUnion.vue`
  - 新增按钮/展示：运行 consumer check 并显示 summary.json 路径。

## 3) build/test 证据

### 3.1 cargo test

```text
running 33 tests
...
test comm::bridge_plc_import::tests::consume_bridge_writes_summary_json ... ok
...
test result: ok. 33 passed; 0 failed
```

### 3.2 pnpm build（UI 有改动）

```text
> plc-code-forge@0.1.0 build C:\Program Files\Git\code\PLCCodeForge
> vue-tsc --noEmit && vite build

vite v6.4.1 building for production...
✓ 1458 modules transformed.
✓ built in 3.66s
```

## 4) summary.json 样例片段

样例文件（来自单测生成）：
`C:\Users\DELL\AppData\Local\Temp\plc-codeforge-bridge-consume-bc587ee0-1542-49e8-8172-a6cff6dbedce\bridge_check\t1\summary.json`

```json
{
  "totalPoints": 1,
  "byChannel": { "ch1": 1 },
  "byQuality": { "Ok": 1 },
  "first10": [
    { "name": "P1", "channelName": "ch1", "readArea": "Holding", "absoluteAddress": 1 }
  ]
}
```

## 5) 操作步骤（合并前 demo）

1) ImportUnion 页先运行 `一键演示（Wizard）`，确保生成 `CommIR 已导出：irPath`。
2) 点击 `导出 PLC Bridge（v1）`，页面提示 `PLC Bridge 已导出：...`。
3) 点击 `运行 bridge consumer check`，页面提示 `bridge_check 已生成：.../summary.json`，点击 `打开 summary.json` 查看统计。

## 6) 自检清单（逐条勾选）

- [x] 不依赖真实 PLC（mock 流程即可生成 bridge/summary）
- [x] 不接入 plc_core orchestrate（仅验证“数据可消费”）
- [x] summary.json 落盘到 `outputDir/bridge_check/<ts>/summary.json`
- [x] command 文件 IO 走 `spawn_blocking`（不阻塞 UI）

## 7) 风险与未决项

- 当前 summary 的 `byQuality` key 使用 `Quality` 的 Debug 文本（例如 `Ok/Timeout`）；若后续需要更严格枚举值（camelCase/大写），需在 v1 上新增字段或 bump v2。

