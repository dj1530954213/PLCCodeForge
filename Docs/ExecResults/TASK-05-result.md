# TASK-05-result.md

- **Task 编号与标题**：
  - TASK-05：driver/mock.rs（MVP 必须）

- **完成摘要**：
  - 在 `src-tauri/src/comm/driver/mod.rs` 定义最小 Driver 抽象：`CommDriver` + `RawReadData` + `DriverError`，为 mock/真实驱动统一调用形态。
  - 在 `src-tauri/src/comm/driver/mock.rs` 实现 `MockDriver`：可稳定制造 `OK/Timeout/DecodeError` 三种场景（通过 `channelName` 触发）。
  - 为满足验收证据，`src-tauri/src/comm/engine.rs` 增加最小 `execute_plan_once`（一次性执行 plan 并产出 `SampleResult + RunStats`），并加单测验证 stats 变化。

- **改动清单**：
  - `src-tauri/src/comm/driver/mod.rs`
    - 新增：`RawReadData`（Registers/Coils）
    - 新增：`DriverError`（Timeout/Comm）
    - 新增：`CommDriver` trait（read 返回 boxed future，无需引入额外 async-trait 依赖）
  - `src-tauri/src/comm/driver/mock.rs`
    - 新增：`MockDriver`（按 `channelName` 规则返回 OK/Timeout/短数据）
  - `src-tauri/src/comm/engine.rs`
    - 新增：`execute_plan_once`（用于 mock 验收；后续 TASK-08 将演进为后台 run）
    - 新增单测：`mock_driver_produces_ok_timeout_and_decode_error_stats`
  - `Docs/ExecResults/TASK-05-result.md`
    - 新建任务结果归档文件（本文件）

- **关键实现说明**：
  - Mock 触发规则（便于前端/测试快速构造）：  
    - `channelName` 包含 `timeout` → `DriverError::Timeout` → quality 记为 `Timeout`
    - `channelName` 包含 `decode` → 返回短数据（比请求少 1）→ 上层 decode 失败 → quality 记为 `DecodeError`
    - 其他 → 返回确定性假数据（寄存器按 startAddress 递增；线圈按奇偶交替）

- **完成证据**：
  - `cargo test --manifest-path src-tauri/Cargo.toml` 输出片段：
    ```text
    running 7 tests
    ...
    test comm::engine::tests::mock_driver_produces_ok_timeout_and_decode_error_stats ... ok
    test result: ok. 7 passed; 0 failed;
    ```

- **验收自检**：
  - [x] `driver/mock.rs` 能制造 OK/Timeout/DecodeError（通过 channelName 触发）。
  - [x] 默认可在无真实 PLC 环境下运行（mock 不依赖端口/外部服务）。
  - [x] 单测验证 mock 场景下 `RunStats` 中 ok/timeout/decodeError 计数正确变化。
  - [x] `cargo test` 通过并输出测试名。
  - [x] `Docs/ExecResults/TASK-05-result.md` 已归档。

- **风险/未决项**：
  - 当前 `execute_plan_once` 是“一次性执行”能力；后台 run 的 start/stop/latest/stats（以及 stop < 1s、latest 只读缓存）将在 TASK-08 完整实现与验收。

- **下一步建议**：
  - 进入 TASK-06：实现 `driver/modbus_tcp.rs`（基于 `tokio-modbus`，并对接 `CommDriver`）。
