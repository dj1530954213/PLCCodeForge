# TASK-06-result.md

- **Task 编号与标题**：
  - TASK-06：modbus_tcp.rs（真实 TCP 读）

- **完成摘要**：
  - 在 `Tauri.CommMapping/src-tauri/src/comm/driver/modbus_tcp.rs` 实现 `ModbusTcpDriver`，基于 `tokio-modbus` 读取 Holding/Input/Coil/Discrete，并适配 `CommDriver` 抽象。
  - Driver 仅负责“真实读段 + 明确错误返回”；timeout/retry 由上层 `engine` 控制（当前 `execute_plan_once` 已使用 `tokio::time::timeout` 包裹）。
  - 增加 env-gated 的 TCP 读测试入口：默认 skip，不影响 CI/离线环境；你提供服务后可开启联调。

- **改动清单**：
  - `Tauri.CommMapping/src-tauri/src/comm/driver/modbus_tcp.rs`
    - 新增：`ModbusTcpDriver`（实现 `CommDriver`）
    - 支持读取 area：`Holding/Input/Coil/Discrete`
    - 错误映射：连接/读失败 → `DriverError::Comm { message }`
    - 新增 env-gated 单测：`it_can_read_holding_registers_when_enabled`
  - `Tauri.CommMapping/Docs/ExecResults/TASK-06-result.md`
    - 新建任务结果归档文件（本文件）

- **关键实现说明**：
  - 连接方式：`tcp::connect_slave(socket_addr, Slave(unit_id))`。
  - 读取方式按 `ReadJob.readArea` 分派到 tokio-modbus 对应 API：`read_holding_registers/read_input_registers/read_coils/read_discrete_inputs`。
  - 联调测试 gating（冻结口径）：仅当 `COMM_IT_ENABLE=1` 且设置 `COMM_IT_TCP_HOST/COMM_IT_TCP_PORT/COMM_IT_TCP_UNITID` 时才会真正发起连接；否则测试直接返回（skip）。

- **完成证据**：
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml` 输出片段：
    ```text
    test comm::driver::modbus_tcp::tests::it_can_read_holding_registers_when_enabled ... ok
    test result: ok. 8 passed; 0 failed;
    ```

- **验收自检**：
  - [x] 基于 `tokio-modbus` 实现 Modbus TCP 读取（至少 Holding/Coil；本次同时覆盖 Input/Discrete）。
  - [x] Driver 返回明确错误类型，不在 driver 内实现 retry（交由 engine）。
  - [x] 存在可被 `COMM_IT_ENABLE=1` 打开的联调用例入口；默认 skip。
  - [x] `cargo test` 通过并输出测试名。
  - [x] `Tauri.CommMapping/Docs/ExecResults/TASK-06-result.md` 已归档。

- **风险/未决项**：
  - 当前 driver 每次 job 读取都会新建一次 TCP 连接；后续如需性能优化，可在 engine 层做连接复用/连接池（不影响本阶段正确性）。

- **下一步建议**：
  - 进入 TASK-07：实现 `driver/modbus_rtu.rs`（tokio-serial + tokio-modbus），并同样加 env-gated 入口用于你介入联调。
