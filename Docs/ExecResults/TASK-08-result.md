# TASK-08-result.md

- **Task 编号与标题**：
  - TASK-08：engine.rs（start/stop/latest/stats，非阻塞）

- **完成摘要**：
  - 在 `src-tauri/src/comm/engine.rs` 落地后台采集引擎 `CommRunEngine`：`start_run` spawn 后台任务、`latest` 只读缓存、`stop_run` 在 1 秒内等待退出。
  - 结果按 `pointKey` 关联与存储，并在对外读取时按 `points` 输入顺序返回（保证 UI 行稳定对齐）。
  - 补齐单测：mock 环境下验证 stop < 1s 且 latest 返回结果顺序与 points 行一致。

- **改动清单**：
  - `src-tauri/src/comm/engine.rs`
    - 新增：`CommRunEngine`（run registry：start/stop/latest）
    - 新增：后台任务使用 `watch` 停止信号 + `tokio::select!`，可在 stop 时中断正在进行的采集（drop future）
    - latest 缓存：`LatestSnapshot { results, stats }`（parking_lot Mutex）
    - 新增单测：`run_engine_stop_within_1s_and_latest_is_ordered_by_points`
  - `Docs/ExecResults/TASK-08-result.md`
    - 新建任务结果归档文件（本文件）

- **关键实现说明**：
  - `start_run`：仅创建 runId + spawn 后台循环，不在调用线程内做采集循环（满足“不阻塞 command”的硬约束）。
  - `stop_run`：发送 stop 信号并在 `timeout(1s)` 内等待 join handle 结束；后台循环用 `select` 监听 stop，从而使 stop 能尽快生效（mock 下已验证 < 1s）。
  - `latest`：纯读取内存缓存，不触发采集；结果以 points 顺序返回，内部关联使用 `pointKey`，避免变量名变更导致错位。

- **完成证据**：
  - `cargo test --manifest-path src-tauri/Cargo.toml` 输出片段：
    ```text
    test comm::engine::tests::run_engine_stop_within_1s_and_latest_is_ordered_by_points ... ok
    test result: ok. 10 passed; 0 failed;
    ```

- **验收自检**：
  - [x] `start_run` spawn 后台任务，不在调用线程内循环采集。
  - [x] `stop_run` 通过 stop 信号触发后台退出，并在 1 秒内完成（mock 单测覆盖）。
  - [x] `latest` 只读缓存，不触发采集；结果按 points 顺序返回并用 `pointKey` 对齐。
  - [x] `cargo test` 通过并输出测试名。
  - [x] `Docs/ExecResults/TASK-08-result.md` 已归档。

- **风险/未决项**：
  - 目前后台循环使用单一 `poll_interval_ms`；后续若需要“每个通道不同轮询周期”，可在不破坏现有契约的前提下扩展为可选字段/更细粒度调度。

- **下一步建议**：
  - 进入 TASK-09：在 `src-tauri/src/comm/tauri_api.rs` 定义并注册 commands（profiles/points/plan/run/export），冻结对外 DTO（后续只加可选字段）。
