# TASK-POINTS-UX-REFINE：点位编辑表格重构（地址表达 + 运行集成 + Excel 级编辑体验）

## 完成摘要
- 点位页改为 **RevoGrid** 数据网格（MIT），替代传统表格组件，实现可用的单元格编辑、键盘导航、复制粘贴与选区操作。
- 点位表格对用户侧“通道名称”改为 **Modbus 地址表达**（40001 风格为主），并在前端将输入映射到后端 `addressOffset`（相对 profile.startAddress 的 0-based 偏移），保持后端 plan/driver 不变。
- “运行采集”融合进点位表格：运行后每行展示 `valueDisplay/quality/errorMessage/timestamp/durationMs`，并提供轮询频率、stats、runId、错误面板与最近调用日志。
- UI 导航收敛：移除左侧菜单，仅保留工程工作区 Tabs + 顶部“工程列表”入口。

## 调研与选型
见：`Tauri.CommMapping/Docs/comm-grid-selection.md`

## 改动清单（关键文件）
- `src/comm/pages/Points.vue`
  - RevoGrid 落地：`range + clipboard + editors`
  - 点位列：`变量名称(HMI)`/`Modbus 地址`/`DataType`/`ByteOrder`/`Scale` + 运行列
  - 运行集成：`comm_run_start_obs` + `comm_run_latest_obs` 轮询合并（按 `pointKey`）+ `comm_run_stop_obs`
  - 编辑-运行策略：运行中修改点位 -> 提示“需重启生效”（提供一键重启）
- `src/comm/components/revogrid/TextEditor.vue`
- `src/comm/components/revogrid/SelectEditor.vue`
- `src/comm/components/revogrid/NumberEditor.vue`
  - RevoGrid Vue 编辑器封装（text/select/number）
- `src/App.vue`
  - 移除左侧目录式菜单，顶部按钮跳转工程列表
- `vite.config.ts`
  - dev server 端口调整为 `61420`（避免占用冲突）
- `Tauri.CommMapping/src-tauri/tauri.conf.json`
  - `build.devUrl` 同步到 `http://localhost:61420`
- `src/main.ts`
  - 移除 `vxe-table` 注册（已不再使用）
- `package.json` / `pnpm-lock.yaml`
  - 依赖：保留 `@revolist/vue3-datagrid` / `@revolist/revogrid`；移除 `vxe-table`/`xe-utils`

## 关键实现说明

### 1) 40001 风格地址解析/展示规则（前端）
- 入口：`src/comm/pages/Points.vue` 的 `parseModbusAddress()` / `formatModbusAddress()`
- 解析（MVP 支持 4xxxx，同时兼容 0/1/3/4 前缀风格）：
  - `40001` -> `{ area: Holding, start0: 0 }`
  - `30001` -> `{ area: Input, start0: 0 }`
  - `10001` -> `{ area: Discrete, start0: 0 }`
  - `00001`/`1` -> `{ area: Coil, start0: 0 }`
- 映射到后端点位结构（保持原有结构化输入，不改 DTO）：
  - profile 已包含：`readArea + startAddress(0-based) + length`
  - point 保存：`addressOffset = start0 - profile.startAddress`（0-based）
  - 若地址单元格为空：`addressOffset=None`，保持旧行为（按 points 顺序从 profile.startAddress 自动顺排）
- 校验提示（行内红边框，仅错误才标红）：
  - 非纯数字、area 与 profile.readArea 不匹配、越界（start/len）、dtype 与 area 不匹配等都会给出 message。

### 2) 运行结果合并到表格（按 pointKey）
- Start：`comm_run_start_obs({ profiles, points, plan })`（后端 spawn，不阻塞 UI）
- Poll：按 `pollMs` 每 N ms 调用 `comm_run_latest_obs(runId)`，将 results 映射为 `{ [pointKey]: SampleResult }` 并回填到表格运行列。
- Stop：`comm_run_stop_obs(runId)`，清理轮询定时器。

### 3) 运行中编辑点位的生效策略
- 运行中允许继续编辑（表格可编辑）。
- 任意编辑会触发 `pointsRevision` 递增；若与 `runPointsRevision` 不同，显示提示按钮：`配置已变更：重启使其生效`。
- 点击重启：`stop -> start`，让新配置在新 run 中生效（MVP 方案，避免热更新复杂度）。

## 验收证据

### pnpm build
```bash
pnpm build
```
关键输出片段：
```
vite v6.4.1 building for production...
✓ 1492 modules transformed.
✓ built in 4.07s
```

### cargo build/test
```bash
cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
cargo test  --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
```
关键输出片段：
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.51s
running 39 tests
test result: ok. 39 passed; 0 failed
```

### cargo tauri dev（端口与启动验证）
必须从 `Tauri.CommMapping/src-tauri/` 目录运行（在 `Tauri.CommMapping/src-tauri/src` 下会因找不到 tauri.conf 而失败）。
```bash
cd Tauri.CommMapping/src-tauri
cargo tauri dev
```
关键输出片段：
```
VITE v6.4.1  ready in 299 ms
Local: http://localhost:61420/
Running `target\\debug\\tauri-app.exe`
```

### 真实 Modbus TCP 跑通（非 mock）
在本机启动一个临时 `pymodbus` TCP server（1502 端口），然后运行后端集成测试（真实走 Modbus TCP 协议栈）。

命令片段（示例）：
```powershell
$env:COMM_IT_ENABLE='1'
$env:COMM_IT_TCP_HOST='127.0.0.1'
$env:COMM_IT_TCP_PORT='1502'
$env:COMM_IT_TCP_UNITID='1'
cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml tcp_quality_ok_for_two_points_when_enabled -- --nocapture
```

关键输出片段：
```
tcp job[0] area=Holding startAddress=0 length=3
tcp raw job[0] = Ok(Registers([16256, 0, 0]))
test tcp_quality_ok_for_two_points_when_enabled ... ok
```

## UI 操作步骤（现场联调）
1) 进入工程：`工程列表 -> 打开工程 -> 点位与运行`
2) 连接参数页配置 Modbus TCP/RTU，并保存 profiles
3) 点位与运行页：
   - 选择连接（下拉）
   - 表格里录入 `变量名称(HMI)` 与 `Modbus 地址(40001...)`
   - 点击“开始运行”，观察每行 `quality/valueDisplay/timestamp/durationMs`
   - 若编辑点位：出现“配置已变更：重启使其生效”，点击重启后生效

## 风险与未决项
- RevoGrid 为 Web Component：若用户机器上 dev server 端口冲突，需同步修改 `vite.config.ts` 与 `Tauri.CommMapping/src-tauri/tauri.conf.json` 的 `devUrl`。
- 目前 UI 以“选择连接（channel）-> 编辑该连接下点位 -> 运行”方式收敛；若一个工程确实需要多连接并行运行，后续可再扩展为“多连接聚合视图/并行 run”。

