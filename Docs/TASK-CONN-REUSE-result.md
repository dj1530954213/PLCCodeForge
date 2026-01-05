# TASK-CONN-REUSE-result.md

## 目标回顾
- 同一个 `runId` 的轮询周期内：同一个 `connectionKey` 必须复用同一条 Modbus 连接，不允许每轮重新 connect/disconnect。
- 失败重连：连接断开/超时/IO 错误时，允许本轮标记错误，并触发重连（同一轮对同一 `connectionKey` 最多重连 1 次）。
- Stop：发出 stop 后 1s 内轮询线程必须结束；不能卡在 connect 或 read 上（需要 timeout + abort/cancel）。

## 关键改动（代码落点）
### 1) 引入 ConnectionKey + ConnectionManager（run 内独享）
- `src-tauri/src/comm/adapters/driver/mod.rs`
  - 新增 `ConnectionKey`（Hash/Eq）：
    - TCP：`ip + port + unitId`
    - RTU：`serialPort + baud + parity + dataBits + stopBits + slaveId`
  - `CommDriver` 拆为三段职责：
    - `connection_key(profile) -> ConnectionKey`
    - `connect(profile) -> ConnectedClient`
    - `read_with_client(client, job) -> RawReadData`
- `src-tauri/src/comm/adapters/driver/connection_manager.rs`
  - 新增 `ConnectionManager`（**每个 runId 在后台 task 内独享**）
  - `ensure_connected()`：缺失则 connect 并缓存；已存在则直接复用
  - `invalidate()`：移除坏连接，下次触发重连

### 2) Driver 改为“connect 一次，多次读”
- `src-tauri/src/comm/adapters/driver/modbus_tcp.rs`
  - `connect()`：只负责 `tcp::connect_slave(...)`
  - `read_with_client()`：复用同一个 `Context` 多次读
- `src-tauri/src/comm/adapters/driver/modbus_rtu.rs`
  - `connect()`：只负责 `SerialStream::open + rtu::attach_slave(...)`
  - `read_with_client()`：复用同一个 `Context` 多次读

### 3) Engine 轮询结构：select(stop, tick) + do_poll_once() 复用连接
- `src-tauri/src/comm/usecase/engine.rs`
  - `CommRunEngine::start_run` 的后台 task：
    - 只创建 **一次** `ConnectionManager::new(run_id)`，贯穿整个 run 生命周期
    - 循环结构变为：
      - `select! { stop => break, tick => do_poll_once() }`
  - `execute_plan_once_with_manager(...)`：
    - 按 `ConnectionKey` 对 plan jobs 分组
    - 每个 group 每轮只 `ensure_connected()` 一次（从第二轮开始会走 reuse）
  - `read_with_retry(...)`：
    - 所有 `connect/read/sleep` 都同时受 `timeout` 和 `stop` 控制
    - 同一轮同一 `ConnectionKey` 最多触发 1 次 `invalidate + reconnect`

## 关键日志（验收要求）
日志输出位置：`ConnectionManager` 内部 `eprintln!`（stderr），你在运行 `cargo tauri dev` 的终端能看到。

### 预期日志形态
**第一次轮询（connect 只出现一次）**
```
[comm][conn] runId=<RUN_ID> connect key=tcp://127.0.0.1:502?unitId=1
```

**后续轮询（只出现 reuse，不再重复 connect）**
```
[comm][conn] runId=<RUN_ID> reuse connection key=tcp://127.0.0.1:502?unitId=1
```

**故障时（invalidate + reconnect）**
```
[comm][conn] runId=<RUN_ID> invalidate key=tcp://127.0.0.1:502?unitId=1 reason=read failed; will reconnect once
[comm][conn] runId=<RUN_ID> connect key=tcp://127.0.0.1:502?unitId=1
```

## 最小复现步骤（10 秒只 connect 1 次）
1. 启动开发模式：在 `src-tauri/` 目录运行 `cargo tauri dev`。
2. App 内配置一个 TCP 连接（同一 ip/port/unitId）并设置 `pollIntervalMs=500`（或 1000）。
3. 点位页点击“开始运行”，等待约 10 秒。
4. 观察启动 `cargo tauri dev` 的终端日志：
   - **只出现 1 次** `connect key=...`
   - 随后每个 tick 只出现 `reuse connection key=...`

## Stop 为何能 < 1s 生效（说明）
- 轮询主循环：`src-tauri/src/comm/usecase/engine.rs` 的 `CommRunEngine::start_run` 内部使用 `tokio::select!` 同时监听：
  - `stop_rx.changed()`（stop 立刻打断下一次 tick）
  - `ticker.tick()`（触发一次 poll）
- connect/read/retry-sleep 都受两层保护：
  - `tokio::time::timeout(timeout, ...)`
  - `tokio::select! { stop => return None, ... => ... }`
- 因为 stop 分支会 drop 掉正在进行的 connect/read future，所以不会卡在“等待超时结束”，也不会把 stop 延迟到下一轮。

## 构建/测试
- `cargo fmt --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo build --manifest-path src-tauri/Cargo.toml`

本次执行输出片段（Windows / PowerShell）：

```text
> cargo test --manifest-path src-tauri/Cargo.toml
running 41 tests
...
test result: ok. 41 passed; 0 failed
...
running 2 tests
test result: ok. 2 passed; 0 failed
```

```text
> cargo build --manifest-path src-tauri/Cargo.toml
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.67s
```
