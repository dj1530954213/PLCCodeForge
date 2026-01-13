# TASK-09-result.md

- **Task 编号与标题**：
  - TASK-09：tauri_api.rs + lib.rs 注册 commands（契约冻结）

- **完成摘要**：
  - 在 `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs` 定义并暴露通讯采集模块的 Tauri commands（profiles/points/plan/run/export），并将对外 DTO 视为冻结契约（只允许新增可选字段）。
  - 在 `Tauri.CommMapping/src-tauri/src/lib.rs` 通过 `tauri::Builder::manage(CommState::new())` 注入全局状态，并注册 commands 到 `invoke_handler`。
  - start/stop/latest 语义满足“command 不阻塞 UI”硬约束（已写入 `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs` 顶部注释）：
    - `comm_run_start` 只 spawn 后台任务（不在 command 内循环采集）
    - `comm_run_latest` 只读缓存（不触发采集）
    - `comm_run_stop` 1s 内生效（MVP；由 engine 单测锁死）

- **改动清单**：
  - `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
    - 新增并暴露 commands（统一 `comm_` 前缀）：
      - `comm_profiles_load/save`
      - `comm_points_load/save`
      - `comm_plan_build -> PlanV1`（返回 `schemaVersion: 1` + `jobs`）
      - `comm_run_start -> { runId }`
      - `comm_run_stop -> ()`（无返回值；错误返回 string message）
      - `comm_run_latest -> { results, stats, updatedAtUtc }`
      - `comm_export_xlsx -> { outPath, headers }`（headers 同时提供 `tcp/rtu/params` 与 `tcpSheet/rtu485Sheet/paramsSheet`）
    - 新增 `CommState`：内存态（profiles/points/plan）+ `CommRunEngine` + drivers（mock/tcp/rtu），通过 `tauri::State` 管理
  - `Tauri.CommMapping/src-tauri/src/lib.rs`
    - 将上述 commands 注册到 `invoke_handler`

- **完成证据**：
  - `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml` 输出片段：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.38s
    ```
  - `Tauri.CommMapping/src-tauri/src/lib.rs` 注册 commands 片段：
    ```rust
    .invoke_handler(tauri::generate_handler![
        greet,
        comm_ping,
        comm_profiles_save,
        comm_profiles_load,
        comm_points_save,
        comm_points_load,
        comm_plan_build,
        comm_run_start,
        comm_run_latest,
        comm_run_stop,
        comm_export_xlsx,
    ])
    ```
  - invoke 示例（request/response JSON 片段；mock 可用）：
    - `comm_run_start`：
      - request：
        ```json
        {
          "request": {
            "driver": "Mock",
            "profiles": {
              "schemaVersion": 1,
              "profiles": [
                {
                  "protocolType": "TCP",
                  "channelName": "tcp-ok",
                  "deviceId": 1,
                  "readArea": "Holding",
                  "startAddress": 0,
                  "length": 10,
                  "ip": "127.0.0.1",
                  "port": 502,
                  "timeoutMs": 1000,
                  "retryCount": 0,
                  "pollIntervalMs": 500
                }
              ]
            },
            "points": {
              "schemaVersion": 1,
              "points": [
                {
                  "pointKey": "00000000-0000-0000-0000-000000000001",
                  "hmiName": "P1",
                  "dataType": "UInt16",
                  "byteOrder": "ABCD",
                  "channelName": "tcp-ok",
                  "scale": 1.0
                }
              ]
            }
          }
        }
        ```
      - response：
        ```json
        { "runId": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" }
        ```
    - `comm_run_latest`：
      - request：
        ```json
        { "runId": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" }
        ```
      - response：
        ```json
        {
          "results": [
            {
              "pointKey": "00000000-0000-0000-0000-000000000001",
              "valueDisplay": "0",
              "quality": "Ok",
              "timestamp": "2026-01-02T00:00:00Z",
              "durationMs": 1,
              "errorMessage": ""
            }
          ],
          "stats": { "total": 1, "ok": 1, "timeout": 0, "commError": 0, "decodeError": 0, "configError": 0 },
          "updatedAtUtc": "2026-01-02T00:00:00Z"
        }
        ```

- **验收自检**：
  - [x] commands 已在 `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs` 定义并在 `Tauri.CommMapping/src-tauri/src/lib.rs` 注册。
  - [x] `comm_run_start` spawn 后台任务；`comm_run_latest` 只读缓存；`comm_run_stop` 1s 内生效（MVP）。
  - [x] 每个 command 返回 `Result<_, String>`，错误用 string message 表达，禁止 panic。
  - [x] DTO 契约冻结说明已写入 `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs` 顶部注释。
  - [x] `Tauri.CommMapping/Docs/ExecResults/TASK-09-result.md` 已归档（本文件）。

- **风险/未决项**：
  - 契约已冻结：后续如需变更字段名/语义，必须通过新增 DTO/command（版本化）来演进。

