# TASK-13-result.md

- **Task 编号与标题**：
  - TASK-13：实现 AppData/<app-name>/comm/ 持久化（schemaVersion=1）

- **完成摘要**：
  - 所有 profiles/points/plan/last_results 的 JSON 读写统一落盘到 `AppData/<app-name>/comm/`，并强制顶层 `schemaVersion: 1`。
  - 固定文件名（硬约束）已实现为 Rust 常量单一真源：
    - `profiles.v1.json`
    - `points.v1.json`
    - `plan.v1.json`
    - `last_results.v1.json`
  - commands 已接入落盘：
    - `comm_profiles_save/load` ⇄ `profiles.v1.json`
    - `comm_points_save/load` ⇄ `points.v1.json`
    - `comm_plan_build` → `plan.v1.json`
    - `comm_run_stop` → `last_results.v1.json`（后台写入，避免阻塞）
  - 满足硬约束：`comm_run_latest` 保持“只读缓存”（不做文件 I/O）。

- **改动清单**：
  - `Tauri.CommMapping/src-tauri/src/comm/storage.rs`
    - 新增：固定文件名常量 `PROFILES_FILE_NAME/POINTS_FILE_NAME/PLAN_FILE_NAME/LAST_RESULTS_FILE_NAME`
    - 新增：`PlanV1` / `LastResultsV1`（顶层 `schemaVersion:1`）
    - 新增：`write_json_atomic`（tmp + rename）避免写入中断导致 JSON 半截
    - 新增：单测打印落盘 JSON（便于验收贴片段）
  - `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
    - `comm_base_dir()`：通过 `app.path().app_data_dir()` 获取 AppData 并拼接 `comm/`
    - `comm_profiles_save/load`、`comm_points_save/load`：接入落盘读写（目录不存在自动创建）
    - `comm_plan_build`：生成 plan 后落盘 `plan.v1.json`
    - `comm_run_stop`：后台落盘 `last_results.v1.json`

- **完成证据**：
  - `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml` 输出片段：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.44s
    ```
  - 实际落盘路径说明（代码片段）：`Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`：
    ```rust
    fn comm_base_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        Ok(storage::comm_dir(app_data_dir)) // => AppData/<app-name>/comm/
    }
    ```
  - 目录与文件名清单（常量单一真源）：`Tauri.CommMapping/src-tauri/src/comm/storage.rs`
    - `STORAGE_DIR_NAME = "comm"`
    - `PROFILES_FILE_NAME = "profiles.v1.json"`
    - `POINTS_FILE_NAME = "points.v1.json"`
    - `PLAN_FILE_NAME = "plan.v1.json"`
    - `LAST_RESULTS_FILE_NAME = "last_results.v1.json"`
  - `profiles.v1.json` 示例内容片段（含 `schemaVersion`；来自单测 `--nocapture` 输出）：
    ```json
    {
      "schemaVersion": 1,
      "profiles": [
        {
          "protocolType": "TCP",
          "channelName": "tcp-1",
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
    }
    ```
  - `points.v1.json` 示例内容片段（含 `schemaVersion`；来自单测 `--nocapture` 输出）：
    ```json
    {
      "schemaVersion": 1,
      "points": [
        {
          "pointKey": "00000000-0000-0000-0000-000000000001",
          "hmiName": "P1",
          "dataType": "UInt16",
          "byteOrder": "ABCD",
          "channelName": "tcp-1",
          "scale": 1.0
        }
      ]
    }
    ```

- **验收自检**：
  - [x] 落盘目录固定：`AppData/<app-name>/comm/`（Tauri 2：`app.path().app_data_dir()`）
  - [x] 固定文件名完全匹配：`profiles.v1.json / points.v1.json / plan.v1.json / last_results.v1.json`
  - [x] 每个文件顶层 `schemaVersion: 1`
  - [x] 目录不存在会自动创建（`create_dir_all`）
  - [x] 写入使用 tmp + rename 策略（避免崩溃损坏半截 JSON）
  - [x] commands 与 TASK-09 对齐：`comm_profiles_save/load`、`comm_points_save/load`（不存在不 panic）
  - [x] `Tauri.CommMapping/Docs/ExecResults/TASK-13-result.md` 已归档（本文件）

- **风险/未决项**：
  - 当前 `last_results.v1.json` 在 `comm_run_stop` 时落盘（后台写入）；若需要“运行中持续落盘”，应做节流/合并写入以避免频繁 I/O。

