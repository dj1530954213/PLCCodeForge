# TASK-27-result.md

- **Task 编号与标题**：
  - TASK-27：一键演示抽成可复用 Wizard + evidence.zip（现场可回传证据包）

- **完成摘要**：
  - 将“一键：导入→采集→导出”的页面内逻辑抽离为可复用服务：`src/comm/services/demoPipeline.ts`（步骤数组驱动、进度回调、可取消）。
  - 新增证据包产出能力：`src/comm/services/evidencePack.ts` + 后端 command `comm_evidence_pack_create`，输出到 `AppData/<app>/comm/evidence/<ts>/` 并生成 `evidence.zip`。
  - ImportUnion 页面改为调用 Wizard 服务，并提供：`一键演示（Wizard）` / `取消` / `导出证据包` 按钮，UI 始终可见流水线日志与证据输出位置。

- **改动清单（文件路径 + 关键点）**：
  - Frontend
    - `src/comm/services/demoPipeline.ts`
      - 步骤 runner：import/load/map/save/run_start/latest/run_stop/export + cleanup
      - `onProgress` 回调输出日志；支持 `AbortController` 取消并自动 stop run
    - `src/comm/services/evidencePack.ts`
      - 组装 evidence request，自动附带 `conflict_report.json`（仅当 conflicts>0）
    - `src/comm/pages/ImportUnion.vue`
      - UI 入口：一键演示（Wizard）+ 证据包按钮；日志显示；冲突二次确认（checkbox）
    - `src/comm/api.ts`
      - 新增：`commEvidencePackCreate()` + Request/Response 类型
  - Rust
    - `src-tauri/src/comm/tauri_api.rs`
      - 新增 command：`comm_evidence_pack_create(request)`（spawn_blocking 写文件 + zip）
      - 输出目录：`AppData/<identifier>/comm/evidence/<ts>/...`
    - `src-tauri/src/lib.rs`
      - 注册 command：`comm_evidence_pack_create`
    - `src-tauri/Cargo.toml`
      - 新增依赖：`zip = "2.4.2"`

- **完成证据（build/test）**：
  - `pnpm build`：
    ```text
    vite v6.4.1 building for production...
    ✓ built in 3.51s
    ```
  - `cargo test --manifest-path src-tauri/Cargo.toml`：
    ```text
    test result: ok. 26 passed; 0 failed
    ```

- **pipeline_log.json 片段（至少 6 步，示例字段与实际一致）**：
  ```json
  [
    {"tsUtc":"2026-01-03T00:00:00.000Z","step":"import","status":"info","message":"开始：comm_import_union_xlsx"},
    {"tsUtc":"2026-01-03T00:00:01.000Z","step":"import","status":"ok","message":"导入成功：points=5, profiles=2, warnings=0"},
    {"tsUtc":"2026-01-03T00:00:01.100Z","step":"map","status":"ok","message":"映射完成：outPoints=5, reused=5, created=0, skipped=0, mapperWarnings=0"},
    {"tsUtc":"2026-01-03T00:00:01.200Z","step":"run_start","status":"ok","message":"runId=..."},
    {"tsUtc":"2026-01-03T00:00:03.200Z","step":"latest","status":"ok","message":"latest: results=5, qualities=Ok,Timeout,DecodeError"},
    {"tsUtc":"2026-01-03T00:00:03.400Z","step":"export","status":"ok","message":"outPath=C:\\\\temp\\\\通讯地址表.xlsx, resultsStatus=written"}
  ]
  ```

- **evidence 输出目录树（文字版）**：
  - `AppData/<app-name>/comm/evidence/<ts>/`
    - `pipeline_log.json`
    - `export_response.json`
    - `conflict_report.json`（仅当 conflicts>0 时生成）
    - `通讯地址表.xlsx`（从 outPath 复制一份，确保可回传）
    - `evidence.zip`（打包上述文件）

- **自检**：
  - [x] 一键逻辑已从页面抽离为 `demoPipeline.ts`，UI 仅负责展示与按钮状态
  - [x] 支持取消：`AbortController.abort()` 触发后自动 stop run，并在日志中可见 cleanup
  - [x] 证据包落点固定：`AppData/<app>/comm/evidence/<ts>/...`
  - [x] 不改变既有对外 DTO 契约：新增 command/类型为增量，不破坏既有字段语义

- **风险与未决项**：
  - 若导出 outPath 指向不可读/不可复制的位置，证据包会记录 warning 且跳过 xlsx 拷贝；建议现场使用 DocumentDir 或 `C:\temp\`。

