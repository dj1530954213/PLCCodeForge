# TASK-19-result.md

- **Task 编号与标题**：
  - TASK-19：联合 xlsx strict 错误：从“reject 的 JSON 字符串”统一为结构化错误 + 前端稳定展示

- **完成摘要**：
  - `comm_import_union_xlsx` 改为**永不 reject**：无论 strict/loose 成功或失败都返回 `{ ok, error?, diagnostics?, points, profiles, warnings }`（仅新增可选字段，不改旧字段含义）。
  - strict 校验失败时，返回**结构化** `error.kind/message/details`（不再使用“字符串里塞 JSON”）。
  - 新增前端页面 `联合导入`：可直接展示 `error.kind/message/details` 与 `diagnostics`，用于现场快速定位“缺 Sheet/缺列/非法枚举/行号”等问题。

- **改动清单（文件路径 + 关键点）**：
  - Rust
    - `Tauri.CommMapping/src-tauri/src/comm/error.rs`
      - 新增：`ImportUnionErrorKind/ImportUnionError/ImportUnionErrorDetails`（稳定错误分类）。
    - `Tauri.CommMapping/src-tauri/src/comm/import_union_xlsx.rs`
      - 新增：`ImportUnionXlsxError::to_import_error()`（结构化错误转换）
      - 新增：`ImportUnionXlsxError::diagnostics()`（提取 diagnostics）
      - 移除：`to_json_string()` 路径（避免“JSON 字符串约定”）
    - `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
      - `CommImportUnionXlsxResponse` 新增可选字段：`ok?`、`error?`
      - `comm_import_union_xlsx(path, options?)`：spawn_blocking 保持；失败返回 `ok=false + error`
      - 新增单测：覆盖 strict 缺 sheet/缺列/非法枚举的结构化返回
    - `Tauri.CommMapping/src-tauri/src/comm/mod.rs`
      - 导出：`pub mod error;`
  - Frontend
    - `src/comm/api.ts`
      - 新增：`ImportUnionErrorKind/ImportUnionError/ImportUnionThrownError`
      - `commImportUnionXlsx()`：`ok=false` 时 throw 结构化错误对象（携带 diagnostics，便于 UI 展示）
    - `src/comm/pages/ImportUnion.vue`
      - 新增：联合 xlsx 导入入口（strict 开关 + 展示结构化错误/diagnostics）
    - `src/router/index.ts` / `src/App.vue`
      - 新增路由与菜单：`/comm/import-union`（联合导入）

- **完成证据（build/test）**：
  - `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.46s
    ```
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`（新增用例名可见）：
    ```text
    test comm::tauri_api::tests::import_union_strict_missing_sheet_returns_structured_error_object ... ok
    test comm::tauri_api::tests::import_union_strict_missing_columns_returns_missing_columns_details ... ok
    test comm::tauri_api::tests::import_union_strict_invalid_enum_returns_row_column_raw_and_allowed_values ... ok
    ```
  - `pnpm build`：
    ```text
    vite v6.4.1 building for production...
    ✓ built in 3.44s
    ```

- **strict=true 失败示例（前端拿到对象，不是字符串）**：
  - 调用（示意）：
    ```json
    {
      "path": "C:\\\\data\\\\union.xlsx",
      "options": { "strict": true, "sheetName": "联合点表", "addressBase": "one" }
    }
    ```
  - 后端返回（示例片段，永不 reject）：
    ```json
    {
      "ok": false,
      "error": {
        "kind": "UnionXlsxInvalidSheet",
        "message": "strict: sheet not found: '联合点表', available: [\"OtherSheet\"]",
        "details": { "sheetName": "联合点表", "detectedSheets": ["OtherSheet"], "addressBaseUsed": "one" }
      },
      "diagnostics": {
        "detectedSheets": ["OtherSheet"],
        "detectedColumns": [],
        "usedSheet": "联合点表",
        "strict": true,
        "addressBaseUsed": "one",
        "rowsScanned": 0
      }
    }
    ```
  - 前端展示位置：
    - 菜单：`联合导入`（`/comm/import-union`）
    - 页面直接展示：`error.kind / error.message / error.details` + `diagnostics.*`（JSON pretty print）

- **验收自检**：
  - [x] strict=true 失败不再依赖“字符串里塞 JSON”
  - [x] 错误分类稳定：`ImportUnionErrorKind`（仅增量，不复用 ConfigError/InvalidArgument）
  - [x] command 不阻塞 UI：解析仍在 `spawn_blocking` 中
  - [x] strict=false（loose）导入行为不变（成功仍返回 points/profiles/warnings）
  - [x] 兼容性：旧字段保持不变，仅新增 `ok?`、`error?` 可选字段
  - [x] 单测覆盖：缺 sheet / 缺必填列 / 非法枚举值（含行号等细节）

- **风险与未决项**：
  - 当前 `comm_import_union_xlsx` 在失败时仍返回空的 `points/profiles`（schemaVersion=1），调用方必须以 `ok`/`error` 判断是否成功。
  - 若未来要把该“结构化错误包装”推广到其它 commands（例如 export_xlsx），建议统一一套通用 envelope，但需谨慎评估冻结契约影响。

