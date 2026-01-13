# TASK-25-result.md

- **Task 编号与标题**：
  - TASK-25：一键演示链路（Import → Run(Mock) → Export Delivery），Results 必须 written（runLatest）

- **完成摘要**：
  - 在 `联合导入` 页面新增“一键：导入->采集->导出（演示）”按钮，串起 `comm_import_union_xlsx` → 映射/落盘 → `comm_run_start(Mock)` → `comm_run_latest` → `comm_run_stop` → `comm_export_delivery_xlsx(resultsSource=runLatest)` 的闭环。
  - UI 内置“流水线日志”，每一步成功/失败都可见；失败时显示 `stepName + error.kind + message`，不会静默失败。
  - 演示导出保证：交付版 `通讯地址表.xlsx` 三张冻结表必然存在，并且 `resultsStatus` 必须为 `written`（因为使用 runLatest 传入 results/stats）。

- **改动清单（文件路径 + 关键点）**：
  - Frontend
    - `src/comm/pages/ImportUnion.vue`
      - 新增：一键演示按钮 + 导出路径输入
      - 新增：流水线日志（每步 ok/error/info）
      - 新增：Demo run 配置（mock driver 按 `channelName` 关键字稳定触发 Timeout/DecodeError，用于演示）
  - Docs
    - `Tauri.CommMapping/Docs/Runbook/通讯采集一键演示.md`
      - 新增：环境准备/操作步骤/预期结果/失败排查（resultsStatus=missing）

- **完成证据（build/test）**：
  - `pnpm build`：
    ```text
    > vue-tsc --noEmit && vite build
    ✓ built in 3.55s
    ```
  - `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.39s
    ```
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`：
    ```text
    test result: ok. 26 passed; 0 failed
    ```

- **一键流水线 UI 日志示例（文字版）**：
  - 期望日志会按顺序出现（示例）：
    ```json
    [
      {"step":"import","status":"ok","message":"导入成功：points=5, profiles=2, warnings=0"},
      {"step":"map","status":"ok","message":"映射完成：outPoints=5, reused=5, created=0, skipped=0"},
      {"step":"save","status":"ok","message":"落盘完成：points=5, profiles=2"},
      {"step":"run_start","status":"ok","message":"runId=..."},
      {"step":"latest","status":"ok","message":"latest: results=5, qualities=Ok,Timeout,DecodeError"},
      {"step":"run_stop","status":"ok","message":"已停止"},
      {"step":"export","status":"ok","message":"outPath=C:\\temp\\通讯地址表.xlsx, resultsStatus=written"}
    ]
    ```
  - **预期 UI 结果**：至少出现 `OK/Timeout/DecodeError` 中任意两种（本演示用 mock 的 channelName 关键字稳定触发）。

- **导出 response 片段（resultsStatus 必须 written）**：
  ```json
  {
    "outPath": "C:\\temp\\通讯地址表.xlsx",
    "resultsStatus": "written",
    "resultsMessage": "resultsSource=runLatest: results provided by frontend"
  }
  ```

- **Runbook**：
  - `Tauri.CommMapping/Docs/Runbook/通讯采集一键演示.md`
  - 关键步骤：进入 `通讯采集 → 联合导入`，填写 `文件路径/导出路径`，点击 `一键：导入->采集->导出（演示）`，核对日志与导出文件 sheet。

- **验收自检**：
  - [x] command 不阻塞 UI：导入解析/导出写文件仍由后端 `spawn_blocking`；run start/stop/latest 维持后台/缓存语义
  - [x] mock 默认：一键演示明确使用 `driver=Mock`，不依赖真实 PLC/端口
  - [x] DTO 契约不破坏：未改动既有字段语义，仅前端增加演示逻辑
  - [x] 导出三表冻结不受影响：仍是交付版三张固定 sheet；Results 通过 runLatest 传入并保证 written

- **风险与未决项**：
  - 演示用的 mock 触发策略基于 `channelName` 关键字（timeout/decode）；该策略仅用于演示与排障，不建议直接作为真实配置落盘长期使用。

