# TASK-COMM-UX-REALONLY-REFACTOR 结果

## 1) 目标与硬约束复述
- 点位表格：默认未编辑状态不再红；红色仅用于必填缺失/格式非法。
- 点位编辑 + 运行采集：合并到同一页面，同页切换不丢状态；运行中改点位提示“需重启生效”并提供一键重启。
- 禁止 Mock：仓库代码中不允许残留 `mock/Mock/demoPipeline/useMock/mockMode` 等字样与实现；只支持真实 Modbus TCP/RTU。
- 长任务边界：`run_start` 只 spawn；`run_latest` 只读缓存；`stop` 目标 < 1s。
- 契约：Tauri commands/DTO 语义不破坏（仅允许新增可选字段）。

## 2) 变更清单（关键文件）
| 文件 | 变更类型 | 关键点/理由 |
|---|---|---|
| `src-tauri/src/comm/adapters/driver/mock.rs` | 删除 | 彻底移除 Mock driver。 |
| `src/comm/services/demoPipeline.ts` | 删除 | 移除 demo pipeline（强制 real-only）。 |
| `src/comm/pages/Run.vue` | 删除 | 运行采集并入点位页；路由 `/comm/run` 重定向到 `/comm/points`。 |
| `src/comm/pages/Points.vue` | 修改 | 合并“点位与运行”；vxe-table 改为单击编辑；未编辑默认不红；错误红框；运行区可观测（runId/状态/日志/结果）。 |
| `src/comm/pages/ProjectWorkspace.vue` | 修改 | Tabs 收敛为“连接参数 / 点位与运行 / 导出 / 高级”。 |
| `src/router/index.ts` | 修改 | `/comm/run` -> redirect 到 `/comm/points`。 |
| `src/comm/pages/Connection.vue` | 修改 | 移除 demo 按钮/文案，避免误导。 |
| `src/comm/pages/ImportUnion.vue` | 重写 | 去掉 demo/wizard，仅保留“联合导入（高级）”与落盘到工程能力。 |
| `src/comm/api.ts` | 修改 | `CommDriverKind` 收敛为 `Tcp | Rtu485`；`CommRunError.details` 新增 `missingFields?`（可选字段）。 |
| `src-tauri/src/comm/error.rs` | 修改 | `CommRunErrorDetails` 新增 `missingFields?: [...]`（可选字段，保持兼容）。 |
| `src-tauri/src/comm/usecase/run_validation.rs` | 新增 | run_start 前配置强校验（纯校验，不做 IO/通讯）。 |
| `src-tauri/src/comm/tauri_api.rs` | 修改 | `comm_run_start_obs` 失败返回结构化 `ConfigError + missingFields`；driver 从 profiles 推断/校验协议匹配。 |
| `src-tauri/src/comm/usecase/evidence_pack.rs` | 修改 | 测试数据里移除 `driver:\"mock\"` 字样（满足 0-hit grep）。 |

## 3) UI 调整说明
### 3.1 合并页（点位与运行）
- 入口：工程工作区 Tab `点位与运行`（`/projects/:projectId/comm/points`）。
- 左侧：点位表格（vxe-table）支持区域选择、Ctrl+C/Ctrl+V、批量设置、向下填充。
- 右侧：运行采集（Start/Stop/Plan、状态 tag、runId、统计、结果表、最近 20 条调用日志）。

### 3.2 “未编辑红色”修复 + 校验策略
- 默认不高亮（未触碰/未保存时不红）。
- 单元格校验触发：
  - 用户编辑过该行（`edit-closed`）后：该行必填缺失/非法值会红框提示。
  - 点击保存/点击 Start 前：开启全量校验；失败会自动聚焦到出错单元格。
- 红色仅用于：
  - `hmiName` / `channelName` 为空
  - `scale` 不是有效数字

### 3.3 运行策略（不阻塞 UI + 变更提示）
- Start：先本地校验点位，再加载 profiles 并 build plan，最后调用 `comm_run_start_obs`（后端 spawn）。
- Poll：running 后每 1s 调 `comm_run_latest_obs` 刷新 results/stats。
- Stop：调用 `comm_run_stop_obs`（后端 1s 内停止目标）。
- running 时编辑点位：右侧出现“配置已变更需重启生效”提示，并提供“一键重启”。

## 4) Mock 删除证据（必须 0 命中）
命令：
```bash
rg -n "mock|Mock|demoPipeline|DemoPipeline|useMock|mockMode" src-tauri src
```
输出（0 命中）：
```text
exit_code=1
```

## 5) 真实 Modbus 联调证据（可复现）
### 5.1 后端真实 TCP 读（用本机 Modbus TCP 服务验证）
本次在本机启动一个 Modbus TCP 服务（端口 `15020`），并运行集成测试直连读取（真实 TCP 通讯链路）：
```text
running 1 test
tcp job[0] area=Holding startAddress=0 length=3
tcp raw job[0] = Ok(Registers([16256, 0, 0]))
test tcp_quality_ok_for_two_points_when_enabled ... ok
```

### 5.2 UI 配置样例（TCP）
在 `连接参数` 添加一条 TCP profile，并在 `点位与运行` 添加点位（channelName 必须匹配）：
- Profile（TCP）
  - channelName: `it-tcp`
  - deviceId: `1`
  - readArea: `Holding`
  - startAddress(UI 1-based): `1`
  - length: `10`
  - ip: `127.0.0.1`
  - port: `15020`
- Points（示例 2 行）
  - `IT_U16` / `UInt16` / `ABCD` / channelName=`it-tcp` / scale=`1`
  - `IT_F32` / `Float32` / `ABCD` / channelName=`it-tcp` / scale=`1`

预期 UI：Start 后显示 `runId` 与 `running`，右侧 results 表持续刷新；Stop 后 1s 内进入 `idle`。

## 6) build/test 证据
### 6.1 cargo fmt
```bash
cd src-tauri
cargo fmt
```

### 6.2 cargo test
```text
running 39 tests
...
test result: ok. 39 passed; 0 failed; ...
...
running 2 tests
test result: ok. 2 passed; 0 failed; ...
```

### 6.3 cargo build
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.60s
```

### 6.4 pnpm build
```text
> vue-tsc --noEmit && vite build
vite v6.4.1 building for production...
✓ built in 6.62s
```

## 7) 风险与后续建议
- 运行时计划一致性：当前 UI 在 Start 时强制先 build plan 并随请求发送，避免使用旧 plan；建议后续在后端也做“points/profiles 变化即重建 plan”的兜底。
- 大数据量点位：vxe-table 已具备较强编辑能力；后续可加虚拟滚动/分页与批量导入导出以提升体验。

