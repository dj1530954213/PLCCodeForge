# TASK-07-result.md

- **Task 编号与标题**：
  - TASK-07：modbus_rtu.rs（真实 RTU 读）

- **完成摘要**：
  - 在 `Tauri.CommMapping/src-tauri/src/comm/driver/modbus_rtu.rs` 实现 `ModbusRtuDriver`，基于 `tokio-serial` + `tokio-modbus` 进行 Modbus RTU（485）读取，并适配 `CommDriver` 抽象。
  - 支持读取 area：Holding/Input/Coil/Discrete；串口参数（baud/parity/dataBits/stopBits）由 `ConnectionProfile::Rtu485` 提供并映射到 tokio-serial。
  - 增加 env-gated 的 RTU 读测试入口：默认 skip；你准备好真实串口/设备后可开启联调。

- **改动清单**：
  - `Tauri.CommMapping/src-tauri/src/comm/driver/modbus_rtu.rs`
    - 新增：`ModbusRtuDriver`（实现 `CommDriver`）
    - 串口参数映射：`SerialParity` → `tokio_serial::Parity`；dataBits/stopBits 做范围校验
    - 错误映射：串口打开/读失败 → `DriverError::Comm { message }`
    - 新增 env-gated 单测：`it_can_read_holding_registers_over_rtu_when_enabled`
  - `Tauri.CommMapping/Docs/ExecResults/TASK-07-result.md`
    - 新建任务结果归档文件（本文件）

- **关键实现说明**：
  - 串口打开：`SerialStream::open(tokio_serial::new(port, baud).parity(...).data_bits(...).stop_bits(...))`
  - RTU attach：`rtu::attach_slave(port, Slave(slave_id))`
  - 联调测试 gating（冻结口径）：仅当 `COMM_IT_ENABLE=1` 且设置 `COMM_IT_RTU_*` 环境变量时才会真正打开串口；否则测试直接返回（skip）。

- **完成证据**：
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml` 输出片段：
    ```text
    test comm::driver::modbus_rtu::tests::it_can_read_holding_registers_over_rtu_when_enabled ... ok
    test result: ok. 9 passed; 0 failed;
    ```

- **验收自检**：
  - [x] 基于 `tokio-serial` + `tokio-modbus` 实现 Modbus RTU 读取。
  - [x] 串口参数映射口径存在（parity/dataBits/stopBits）。
  - [x] 存在可被 `COMM_IT_ENABLE=1` 打开的联调用例入口；默认 skip。
  - [x] `cargo test` 通过并输出测试名。
  - [x] `Tauri.CommMapping/Docs/ExecResults/TASK-07-result.md` 已归档。

- **风险/未决项**：
  - 真实 RTU 联调依赖硬件环境（USB-RS485、正确的 COM 口与从站配置）；当前测试默认 skip，需你提供现场/实验环境后再执行验证（对应 TASK-14）。

- **下一步建议**：
  - 进入 TASK-08：把 `engine` 演进为后台 run（start/stop/latest/stats），满足 stop < 1s、latest 只读缓存、按 pointKey 关联等硬约束。
