# TASK-24-result.md

- **Task 编号与标题**：
  - TASK-24：交付导出：可选 Results sheet 的“来源与缺失策略”拍板 + 实现（保证现场可解释）

- **完成摘要**：
  - 为 `comm_export_delivery_xlsx` 增加 `resultsSource`（`appdata`/`runLatest`）并拍板缺失策略：导出始终成功，是否写入 Results sheet 由 `resultsStatus` 明确说明（`written/missing/skipped`）。
  - `includeResults=true & source=appdata`：后端在 `spawn_blocking` 中读取 `AppData/<app>/comm/last_results.v1.json`；不存在/读取失败则 `resultsStatus=missing`，但三张冻结表仍正常导出。
  - `includeResults=true & source=runLatest`：前端先调用 `comm_run_latest(runId)` 并把 `results/stats` 作为参数传入导出；缺失/空则 `resultsStatus=missing`。
  - 前端 Export 页增加 `resultsSource` 选择与 `runId` 输入，并在导出后稳定展示 `resultsStatus/resultsMessage`。

- **改动清单（文件路径 + 关键点）**：
  - Rust
    - `src-tauri/src/comm/tauri_api.rs`
      - 扩展 DTO（仅新增字段，向后兼容）：
        - `CommExportDeliveryXlsxRequest` 新增：`resultsSource?`、`results?`、`stats?`
        - `CommExportDeliveryXlsxResponse` 新增：`resultsStatus?`、`resultsMessage?`
      - `comm_export_delivery_xlsx`：在 `spawn_blocking` 内实现来源/缺失策略，避免阻塞 UI
  - Frontend
    - `src/comm/api.ts`
      - 新增类型：`DeliveryResultsSource` / `DeliveryResultsStatus`
      - 扩展 `commExportDeliveryXlsx` 入参与返回值字段（仅新增可选字段）
    - `src/comm/pages/Export.vue`
      - 新增：Results 来源选择（appdata/runLatest）+ runId 输入
      - 导出后展示：`resultsStatus/resultsMessage`

- **完成证据（build/test）**：
  - `cargo build --manifest-path src-tauri/Cargo.toml`：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.46s
    ```
  - `cargo test --manifest-path src-tauri/Cargo.toml`：
    ```text
    test result: ok. 26 passed; 0 failed
    ```
  - `pnpm build`：
    ```text
    > vue-tsc --noEmit && vite build
    ✓ built in 3.61s
    ```

- **导出 response 示例（2 组）**：
  - 1) includeResults=true & resultsSource=appdata，但 `last_results.v1.json` 缺失：
    ```json
    {
      "outPath": "C:\\temp\\通讯地址表.xlsx",
      "resultsStatus": "missing",
      "resultsMessage": "resultsSource=appdata: last_results.v1.json not found; Results sheet skipped"
    }
    ```
  - 2) includeResults=true & resultsSource=runLatest，前端传入 latest results：
    ```json
    {
      "outPath": "C:\\temp\\通讯地址表.xlsx",
      "resultsStatus": "written",
      "resultsMessage": "resultsSource=runLatest: results provided by frontend"
    }
    ```

- **UI 展示说明（Export 页）**：
  - 勾选 `附加 Results（可选）` 后可选择：
    - `appdata（默认：last_results.v1.json）`：无需填 runId
    - `runLatest（从 runId 的 latest 获取）`：填入 runId（UUID）后导出
  - 导出完成后页面会显示：`Results: written/missing/skipped - <message>`

- **验收自检**：
  - [x] 三张交付表冻结列名/顺序未改（Results sheet 仍为可选附加，不计入冻结）
  - [x] DTO 仅新增可选字段：`resultsSource/results/resultsStatus/resultsMessage`，旧调用方不受影响
  - [x] `spawn_blocking` 保持：导出写文件 + AppData 读取均不阻塞 UI
  - [x] 缺失策略可解释：includeResults=true 但缺结果时导出仍成功，response 明确 `resultsStatus=missing`

- **风险与未决项**：
  - `runLatest` 来源会把 results 从前端传到后端（IPC 体积与点位数相关）；若现场点位量很大，可考虑改为“后端按 runId 从引擎缓存读取”作为 v2（仍需保持 command 非阻塞）。

