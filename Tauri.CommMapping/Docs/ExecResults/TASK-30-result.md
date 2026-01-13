# TASK-30-result.md（Wizard 支持真实驱动联调 + evidence 纳入联调配置快照）

## 1) 完成摘要

- Wizard 增加驱动选择：`mock`（默认）/ `modbus_tcp` / `modbus_rtu(485)`，并把选择透传到 `comm_run_start`。
- 非 mock 场景仍遵守：`start -> latest -> stop -> export`，Results 来源固定 `runLatest`（用于 `resultsStatus=written` 口径）。
- evidence manifest 增补 `connectionSnapshot`（脱敏连接快照：TCP ip/port/unitId；RTU 串口/波特率/校验/位数/站号等），便于现场回传定位。
- 新增联调 Runbook：`Tauri.CommMapping/Docs/Runbook/通讯采集真实联调（Modbus TCP-RTU）.md`。

## 2) 改动清单（文件路径 + 关键点）

- `src/comm/pages/ImportUnion.vue`
  - Wizard 区新增驱动下拉：`Mock/Tcp/Rtu485`。
  - Wizard 结束态统一可导出证据包（即使 ok=false，也能拿到 logs + exportResponse）。
  - 导出证据包 meta：包含 `run.driver/includeResults/resultsSource/durationMs`、counts、`connectionSnapshot`。
- `src/comm/services/demoPipeline.ts`
  - 新增 `params.driver`；mock 用 demo 配置保证至少两种 quality（OK/Timeout/DecodeError）；真实驱动使用落盘 profiles/points。
  - pipeline 在失败时 best-effort 仍尝试 export，保证有 `exportResponse`（用于 evidence）。
- `Tauri.CommMapping/Docs/Runbook/通讯采集真实联调（Modbus TCP-RTU）.md`
  - 真实联调准备/配置/操作步骤/预期结果/排查路径。

## 3) build/test 证据

### 3.1 pnpm build（待本机执行后补贴输出片段）

```text
> pnpm build
> vue-tsc --noEmit && vite build
vite v6.4.1 building for production...
✓ built in 3.44s
```

### 3.2 cargo build/test（如 Rust 无额外改动，可复用 TASK-29 的输出；但此处仍需贴一次）

```text
> cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
   Compiling sha2 v0.10.9
   Compiling tauri-app v0.1.0 (C:\Program Files\Git\code\PLCCodeForge\Tauri.CommMapping\src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.73s

> cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
running 26 tests
test comm::engine::tests::run_engine_stop_within_1s_and_latest_is_ordered_by_points ... ok
test comm::export_xlsx::tests::export_xlsx_emits_warnings_without_changing_frozen_headers ... ok
test comm::tauri_api::tests::import_union_strict_invalid_enum_returns_row_column_raw_and_allowed_values ... ok
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

## 4) 示例（UI 操作步骤 + 预期结果）

### 4.1 Wizard（真实驱动）操作步骤

1. 打开 `联合 xlsx 导入`（`/comm/import-union`）
2. 点击 `导入并生成通讯点位`（生成并落盘 points/profiles）
3. 到 `连接` 页面补齐 TCP/485 Profile 参数并保存
4. 回到 ImportUnion 页面顶部 Wizard 区：
   - 驱动选择：`modbus_tcp` 或 `modbus_rtu`
   - 点击：`一键演示（Wizard）`
5. 点击：`导出证据包`

### 4.2 预期 UI 结果

- Wizard 结束提示：`ok=true`，`resultsStatus=written`
- `流水线日志` 中包含：`run_start`/`latest`/`run_stop`/`export`
- `manifest 摘要` 显示：driver/points/results/conflicts/headersDigest

## 5) 示例 JSON（meta.connectionSnapshot）

### 5.1 TCP 示例

```json
{
  "connectionSnapshot": {
    "protocol": "TCP",
    "channels": [
      { "channelName": "CH1", "deviceId": 1, "readArea": "Holding", "startAddress": 0, "length": 100, "ip": "192.168.1.10", "port": 502 }
    ]
  }
}
```

### 5.2 485 示例

```json
{
  "connectionSnapshot": {
    "protocol": "485",
    "channels": [
      { "channelName": "COM1", "deviceId": 1, "readArea": "Holding", "startAddress": 0, "length": 100, "serialPort": "COM3", "baudRate": 9600, "parity": "Even", "dataBits": 8, "stopBits": 1 }
    ]
  }
}
```

## 6) 自检清单（逐条勾选）

- [x] 驱动选择仅影响 `comm_run_start(driver=...)`，其他 command 仍不阻塞 UI（长 IO 在后端 spawn_blocking）
- [x] 默认仍为 mock（不依赖真实 PLC/端口）
- [x] driver!=mock 时 pipeline 仍是 `start -> latest -> stop -> export`
- [x] evidence manifest 记录 connectionSnapshot（脱敏）+ itFlags（ENV）
- [x] 失败可观测：UI 日志包含 step + error.kind/message，不 silent

## 7) 风险与未决项

- 真实联调需要你提供可达的 Modbus TCP/RTU 服务与参数；本结果文件中的“联调跑通日志片段”需在现场补跑后追加。
- 本 MVP 一次 run 只选择一种 driver；若 points/profiles 混有 TCP/485，可能出现部分点位 `ConfigError/CommError`（建议拆分点表分别跑）。
