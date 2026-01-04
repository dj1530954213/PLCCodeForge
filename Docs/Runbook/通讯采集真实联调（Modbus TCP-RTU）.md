# 通讯采集 Wizard 真实联调 Runbook（Modbus TCP/RTU）

## 目的

- 通过 `联合 xlsx 导入` 页的一键 Wizard，在 `driver=modbus_tcp/modbus_rtu` 下跑通：`start -> latest -> stop -> export`
- 产出可回传的 `evidence.zip`（含 `manifest.json`/日志/导出响应）

## 前置准备

- 已能运行前端/后端构建：`pnpm build`、`cargo build --manifest-path src-tauri/Cargo.toml`
- 有符合 `联合xlsx输入规范.v1` 的联合点表 xlsx（默认 sheet：`联合点表`）
- 真实联调需要你准备 Modbus 服务（TCP 或 485 RTU）并提供连接参数

## 操作步骤（UI）

1. 打开页面：`联合 xlsx 导入`（路由：`/comm/import-union`）
2. 填写：
   - 联合 xlsx 文件路径
   - `strict`（推荐开）
   - Sheet 名（默认 `联合点表`）
   - 地址基准（v1 默认 `one`：Excel 1-based → 内部 0-based）
3. 点击：`导入并生成通讯点位`
   - 预期：点位/Profiles 落盘到 `AppData/<app>/comm/points.v1.json`、`profiles.v1.json`
4. （真实联调必做）切到 `连接` 页面补齐 Profile 参数并保存：
   - TCP：`ip/port/deviceId(read unitId)/readArea/startAddress/length/timeoutMs/retryCount/pollIntervalMs`
   - 485：`serialPort/baudRate/parity/dataBits/stopBits/deviceId/readArea/startAddress/length/timeoutMs/retryCount/pollIntervalMs`
5. 回到 `联合 xlsx 导入` 页顶部 `一键演示（Wizard）+ 证据包` 区：
   - 选择驱动：
     - `mock（默认）`
     - `modbus_tcp（真实联调）`
     - `modbus_rtu（485，真实联调）`
   - 填写导出路径（建议文件名：`通讯地址表.xlsx`）
   - 点击 `一键演示（Wizard）`
6. 观察 `流水线日志（验收用）`：
   - 至少应出现：`run_start`、`latest`、`run_stop`、`export`
   - 若 `resultsStatus=written`：表示 Results sheet 写入成功（来源为 runLatest）
7. 点击 `导出证据包`
   - 记录 `evidenceDir/zipPath`
   - 页面会展示 `manifest 摘要`（driver/points/results/conflicts/headersDigest 等）

## 预期结果（验收口径）

- Wizard 结束提示：`ok=true` 且 `resultsStatus=written`
- 导出的 xlsx 必含三张冻结 sheet：
  - `TCP通讯地址表`（5列冻结）
  - `485通讯地址表`（5列冻结）
  - `通讯参数`（14列冻结）
- evidence 包（目录或 zip）包含：
  - `pipeline_log.json`
  - `export_response.json`
  - `manifest.json`
  - `conflict_report.json`（若存在冲突）
  - `通讯地址表.xlsx` 拷贝（若 copy 成功）

## 常见失败排查

- `resultsStatus=missing`
  - 说明：`runLatest` 未提供 results（run 未启动/无结果/提前 stop）
  - 处理：检查驱动选择、Profile 参数、点位通道/设备号、网络/串口可达性
- `driver=modbus_tcp` 但 points/profiles 混有 `485`
  - 说明：本 MVP 一次 run 只能选一种 driver；混用会导致部分点位 `ConfigError/CommError`
  - 处理：拆分点位或分别跑 TCP/485 两次 Wizard
- stop 不生效
  - 查看 `pipeline_log.json` 的 `run_stop` step 与耗时；确认后端满足 “<1s 生效” 约束

