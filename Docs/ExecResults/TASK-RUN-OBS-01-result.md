# TASK-RUN-OBS-01-result：Run 页可观测性硬化（修复“点击无反应”）

## 1) 完成摘要
- Run 页点击 Start/Stop 立即有可见状态（starting/running/stopping/error）、可见 `runId`、可见错误面板与最近 20 条调用日志。
- 新增 run 相关“可观测 command”：
  - `comm_run_start_obs` / `comm_run_latest_obs` / `comm_run_stop_obs`
  - 返回 `{ ok, value?/runId?, error? }`，`error.kind/message/details` 为结构化对象，避免前端拿到字符串难展示。
- 保持运行边界不变：采集循环仍在后端后台任务；`latest` 只读缓存；`stop` 目标 <1s（已有单测覆盖）。

## 2) 改动清单（文件路径 + 关键点）
### Rust
- `src-tauri/src/comm/error.rs`：新增 `CommRunErrorKind/CommRunError/CommRunErrorDetails`。
- `src-tauri/src/comm/tauri_api.rs`：
  - 新增 DTO：`CommRunStartObsResponse` / `CommRunLatestObsResponse` / `CommRunStopObsResponse`。
  - 新增 commands：`comm_run_start_obs` / `comm_run_latest_obs` / `comm_run_stop_obs`（用于 UI 稳定展示）。
  - 保留原 commands：`comm_run_start` / `comm_run_latest` / `comm_run_stop`（兼容旧调用）。
- `src-tauri/src/lib.rs`：注册新增 commands。

### Frontend
- `src/comm/api.ts`：新增 `commRunStartObs/commRunLatestObs/commRunStopObs` 与类型定义。
- `src/comm/pages/Run.vue`：改用 `*_obs` commands；UI 增加状态、错误面板、调用日志、轮询 latest。

## 3) 验收证据
### 3.1 构建/测试输出片段
#### `cargo test --manifest-path src-tauri/Cargo.toml`
```text
running 42 tests
test result: ok. 42 passed; 0 failed; ...
running 2 tests
test result: ok. 2 passed; 0 failed; ...
```

#### `cargo build --manifest-path src-tauri/Cargo.toml`
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s
```

#### `pnpm build`
```text
vite v6.4.1 building for production...
✓ 1767 modules transformed.
✓ built in 4.68s
```

### 3.2 UI 演示步骤（mock 环境）
1. 工程工作区 → “连接参数”点击 `加载 Demo（mock）` 并保存。
2. 工程工作区 → “点位配置”点击 `加载 Demo（mock）` 并保存。
3. 工程工作区 → “运行采集”：
   - driver 选择 `Mock（默认）`
   - 点击 `Start`：
     - 立刻看到 `starting -> running` 状态与 `runId=...`
     - 每 1s 轮询 `latest`，在“最近调用日志”中持续出现 `run_latest` 记录
     - 统计区出现至少两种质量计数（例如 OK / Timeout / DecodeError）
4. 点击 `Stop`：
   - 状态变为 `stopping -> idle`
   - 日志中出现 `run_stop stopped`

### 3.3 故意制造错误（示例）
- 场景：不配置 points（或清空 points 保存后）直接点 `Start`
  - 页面出现 `ConfigError: points 为空...` 错误面板
  - 日志追加 `run_start` 的 error 记录（无静默失败）

### 3.4 结构化错误示例（obs command 返回对象）
`comm_run_latest_obs`（传入不存在的 runId）返回示例（关键字段）：
```json
{
  "ok": false,
  "error": {
    "kind": "RunNotFound",
    "message": "run not found",
    "details": { "runId": "00000000-0000-0000-0000-000000000000" }
  }
}
```

## 4) 风险与未决项
- 当前 `CommRunErrorKind` 仅做 MVP 分类（ConfigError/RunNotFound/InternalError），后续若要更精确（如 PlanBuildError/DriverError）应增量扩展枚举（不破坏已有值）。
- Export 页仍使用 legacy `comm_run_latest`（非 obs）；如需全站统一结构化错误，可逐步迁移到 `*_obs`。

## 5) 回滚点说明（如何恢复到旧结构）
- 前端：将 `src/comm/pages/Run.vue` 调回使用 `commRunStart/commRunLatest/commRunStop` 即可回滚到旧调用方式。
- 后端：保留原 `comm_run_*` commands 未改签名；新增的 `*_obs` 可直接不再使用。

