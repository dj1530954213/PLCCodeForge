# TASK-18-result.md

- **Task 编号与标题**：
  - TASK-18：冻结“联合 xlsx（IO+设备表合并表）”输入规范 + strict 校验模式（避免列名/Sheet 轻微变化导致静默跳过）

- **完成摘要**：
  - 新增联合 xlsx 输入规范文档（冻结 v1）：固定 Sheet/列名/地址基准/枚举值与去重策略。
  - 在后端导入命令中增加 `options`（向后兼容），并实现 `strict=true` 的硬失败校验与结构化 diagnostics 返回。
  - 新增 Rust 单测覆盖 strict 失败与宽松模式 warnings 行为。

- **改动清单（文件路径 + 关键点）**：
  - `Docs/通讯数据采集验证/联合xlsx输入规范.v1.md`
    - 冻结 v1：目标 Sheet 名、必填列、可选列、地址基准（默认 1-based）、去重策略、strict 行为。
  - `src-tauri/src/comm/import_union_xlsx.rs`
    - 新增/补齐：`ImportUnionOptions`（strict/sheetName/addressBase）、`ImportUnionDiagnostics`、`ImportUnionXlsxError`（MissingSheet/MissingRequiredColumns/InvalidRequiredValue）。
    - `strict=true`：sheet/必填列/必填枚举值非法 → 直接返回错误（不 panic，错误可序列化为 JSON 字符串）。
    - `strict=false`：保持宽松导入（尽量导入 + warnings）。
    - 新增单测：覆盖 strict 缺 sheet/缺列/非法枚举值，以及 strict=false 同输入返回 warnings。
  - `src-tauri/src/comm/tauri_api.rs`
    - 扩展 command：`comm_import_union_xlsx(path, options?) -> { points, profiles, warnings, diagnostics }`（仅新增可选字段/参数，向后兼容）。
    - 解析仍在 `spawn_blocking` 中执行，避免阻塞 UI；strict 错误通过 `to_json_string()` 返回结构化信息。
  - `src/comm/api.ts`
    - 增加 `ImportUnionOptions/ImportUnionDiagnostics` 类型，并支持 `commImportUnionXlsx(path, options?)`。

- **完成证据（build/test）**：
  - `cargo build --manifest-path src-tauri/Cargo.toml`：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.56s
    ```
  - `cargo test --manifest-path src-tauri/Cargo.toml`（新增用例名可见）：
    ```text
    test comm::import_union_xlsx::tests::strict_missing_sheet_fails_with_available_sheet_list ... ok
    test comm::import_union_xlsx::tests::strict_missing_required_columns_fails_with_missing_list ... ok
    test comm::import_union_xlsx::tests::strict_invalid_data_type_fails_with_row_index ... ok
    test comm::import_union_xlsx::tests::loose_mode_returns_warnings_instead_of_failing ... ok
    ```
  - `pnpm build`（本任务改动了 `src/comm/api.ts`）：
    ```text
    vite v6.4.1 building for production...
    ✓ built in 3.44s
    ```

- **示例 JSON（invoke 请求/响应片段）**：
  - strict=true（示例：缺少目标 sheet → 硬失败）
    - request：
      ```json
      {
        "path": "C:\\\\data\\\\union.xlsx",
        "options": { "strict": true, "sheetName": "联合点表", "addressBase": "one" }
      }
      ```
    - response（Tauri reject 的 error string，内容为 JSON）：
      ```json
      {
        "kind": "MissingSheet",
        "message": "strict: sheet not found: '联合点表', available: [\"OtherSheet\"]",
        "diagnostics": {
          "detectedSheets": ["OtherSheet"],
          "detectedColumns": [],
          "usedSheet": "联合点表",
          "strict": true,
          "addressBaseUsed": "one",
          "rowsScanned": 0
        },
        "detectedSheets": ["OtherSheet"]
      }
      ```
  - strict=true（示例：必填枚举值非法 → 硬失败，包含行号）
    - request：
      ```json
      {
        "path": "C:\\\\data\\\\union.xlsx",
        "options": { "strict": true, "sheetName": "联合点表" }
      }
      ```
    - response（error string JSON）：
      ```json
      {
        "kind": "InvalidRequiredValue",
        "message": "strict: invalid value at row 2 column '数据类型': 'BADTYPE', allowed: [\"Bool\", \"Int16\", \"UInt16\", \"Int32\", \"UInt32\", \"Float32\"]",
        "rowIndex": 2,
        "columnName": "数据类型",
        "rawValue": "BADTYPE",
        "allowedValues": ["Bool", "Int16", "UInt16", "Int32", "UInt32", "Float32"],
        "diagnostics": {
          "detectedSheets": ["联合点表"],
          "detectedColumns": ["变量名称（HMI）", "数据类型", "字节序", "通道名称", "协议类型", "设备标识"],
          "usedSheet": "联合点表",
          "strict": true,
          "addressBaseUsed": "one",
          "rowsScanned": 1
        }
      }
      ```
  - strict=false（同类输入：尽量导入 + warnings，不失败）
    - request：
      ```json
      {
        "path": "C:\\\\data\\\\union.xlsx",
        "options": { "strict": false, "sheetName": "联合点表" }
      }
      ```
    - response（片段）：
      ```json
      {
        "points": { "schemaVersion": 1, "points": [] },
        "profiles": { "schemaVersion": 1, "profiles": [] },
        "warnings": [
          { "code": "ROW_DATATYPE_UNKNOWN_SKIP", "message": "row 2: dataType unknown; skipped", "hmiName": "TEMP_1" }
        ],
        "diagnostics": {
          "detectedSheets": ["联合点表"],
          "detectedColumns": ["变量名称（HMI）", "数据类型", "字节序", "通道名称", "协议类型", "设备标识"],
          "usedSheet": "联合点表",
          "strict": false,
          "addressBaseUsed": "one",
          "rowsScanned": 1
        }
      }
      ```

- **验收自检**：
  - [x] 文档：新增 `Docs/通讯数据采集验证/联合xlsx输入规范.v1.md` 并冻结 sheet/列名/地址基准/去重策略
  - [x] command：`comm_import_union_xlsx(path, options?)` 向后兼容（options 可缺省）
  - [x] strict=true：缺 sheet/缺必填列/非法枚举值 → 硬失败（不 panic），错误信息包含 sheet/缺列/行号等关键字段
  - [x] strict=false：保持宽松导入（尽量导入 + warnings）
  - [x] diagnostics：返回 detectedSheets/detectedColumns/usedSheet/strict/addressBaseUsed/rowsScanned
  - [x] 不修改 `通讯地址表.xlsx` 冻结 headers（本任务未触碰 export headers）
  - [x] DTO 契约：仅新增可选字段（response.diagnostics）与可选参数（options），未改旧字段语义
  - [x] command 内部解析在 `spawn_blocking` 执行，避免阻塞 UI

- **风险与未决项**：
  - 若现场最终冻结的 Sheet 名/列名与 v1 文档不一致，需要 bump `联合xlsx输入规范.v2.md` 并在 strict=true 下同步调整必填列集合。
  - strict 错误当前以“JSON 字符串”形式返回（前端如需结构化展示，需对 error string 做 JSON parse）。
  - `pnpm build` 存在 bundle 体积告警（>500kB），不影响本任务验收，但后续可考虑 code-split。
