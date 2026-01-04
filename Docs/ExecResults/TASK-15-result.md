# TASK-15-result.md

- **Task 编号与标题**：
  - TASK-15：为导出与采集补充 warnings/diagnostics（不破坏冻结列）

- **完成摘要**：
  - XLSX 导出仍严格保持 3 张 sheet + 冻结列名/列顺序不变（只增强返回结构）：
    - `comm_export_xlsx` 返回新增 `warnings` + `diagnostics`（可选字段），不新增任何 xlsx 列。
  - 采集运行侧：
    - `comm_run_latest` 返回新增 `runWarnings`（可选字段），来自后台采集每轮的统计归并（不触发采集、只读缓存）。
  - 前端：
    - Export 页在导出后展示 `warnings / diagnostics`（可见即可，不影响既有 headers 验收展示）。

- **改动清单（文件路径 + 关键点）**：
  - `src-tauri/src/comm/model.rs`
    - 新增：`CommWarning`、`CommExportDiagnostics/ExportedRows`
    - 增强：`DataType/ByteOrder32` 增加 `Unknown`（`serde(other)`），用于容忍未知值并在导出/导入时给出 warning（不 panic）
  - `src-tauri/src/comm/export_xlsx.rs`
    - 不改冻结 headers：`HEADERS_TCP/HEADERS_RTU485/HEADERS_PARAMS` 保持不变
    - 新增：导出过程中收集 warnings（至少 6 条规则）+ diagnostics（行数/耗时），并在返回值携带
    - 新增单测：`export_xlsx_emits_warnings_without_changing_frozen_headers`
  - `src-tauri/src/comm/engine.rs`
    - 新增：每轮采集写入缓存时生成 `run_warnings`（基于 stats 汇总）
    - `latest()` 返回结构扩展（仍为只读缓存）
  - `src-tauri/src/comm/tauri_api.rs`
    - `CommExportXlsxResponse`：新增可选字段 `warnings`/`diagnostics`
    - `CommRunLatestResponse`：新增可选字段 `runWarnings`
  - `src/comm/pages/Export.vue`
    - 新增：导出后展示 `warnings / diagnostics`（JSON 文本）
  - `src/comm/api.ts`
    - 新增：`CommWarning`/`CommExportDiagnostics` 类型
    - 更新：`CommExportXlsxResponse`/`CommRunLatestResponse` 对齐新增字段

- **完成证据**：
  - `cargo build --manifest-path src-tauri/Cargo.toml`：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.75s
    ```
  - warnings 触发与返回证据（单测 `-- --nocapture` 输出）：
    - `cargo test --manifest-path src-tauri/Cargo.toml export_xlsx_emits_warnings_without_changing_frozen_headers -- --nocapture`
    ```text
    warningCodes=["POINT_MISSING_CHANNEL_NAME", "POINT_NO_SHEET_MATCH", "POINT_MISSING_PROFILE", "POINT_DATATYPE_UNKNOWN", "POINT_BYTEORDER_UNKNOWN", "POINT_SCALE_INVALID"]
    diagnostics=CommExportDiagnostics { exported_rows: ExportedRows { tcp: 3, rtu: 0, params: 1 }, duration_ms: 5 }
    ```
  - 前端构建（Export 页有改动）：
    - `pnpm build`
    ```text
    vite v6.4.1 building for production...
    ✓ built in 3.67s
    ```

- **示例 JSON（导出 invoke 的 request/response 片段）**：
  - request（示例：构造 2 条 warnings：缺 channelName + 缺 profile）：
    ```json
    {
      "request": {
        "outPath": "C:\\\\temp\\\\通讯地址表.xlsx",
        "profiles": {
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
              "retryCount": 1,
              "pollIntervalMs": 500
            }
          ]
        },
        "points": {
          "schemaVersion": 1,
          "points": [
            {
              "pointKey": "00000000-0000-0000-0000-00000000000a",
              "hmiName": "MISSING_CHANNEL",
              "dataType": "UInt16",
              "byteOrder": "ABCD",
              "channelName": "",
              "scale": 1.0
            },
            {
              "pointKey": "00000000-0000-0000-0000-00000000000b",
              "hmiName": "MISSING_PROFILE",
              "dataType": "UInt16",
              "byteOrder": "ABCD",
              "channelName": "tcp-missing",
              "scale": 1.0
            }
          ]
        }
      }
    }
    ```
  - response（新增字段：`warnings` + `diagnostics`，不改变 `outPath/headers` 语义）：
    ```json
    {
      "outPath": "C:\\\\temp\\\\通讯地址表.xlsx",
      "headers": {
        "tcp": ["变量名称（HMI）", "数据类型", "字节序", "起始TCP通道名称", "缩放倍数"],
        "rtu": ["变量名称（HMI）", "数据类型", "字节序", "起始485通道名称", "缩放倍数"],
        "params": ["协议类型", "通道名称", "设备标识", "读取区域", "起始地址", "长度", "TCP:IP / 485:串口", "TCP:端口 / 485:波特率", "485:校验", "485:数据位", "485:停止位", "超时ms", "重试次数", "轮询周期ms"],
        "tcpSheet": ["变量名称（HMI）", "数据类型", "字节序", "起始TCP通道名称", "缩放倍数"],
        "rtu485Sheet": ["变量名称（HMI）", "数据类型", "字节序", "起始485通道名称", "缩放倍数"],
        "paramsSheet": ["协议类型", "通道名称", "设备标识", "读取区域", "起始地址", "长度", "TCP:IP / 485:串口", "TCP:端口 / 485:波特率", "485:校验", "485:数据位", "485:停止位", "超时ms", "重试次数", "轮询周期ms"]
      },
      "warnings": [
        { "code": "POINT_MISSING_CHANNEL_NAME", "message": "point channelName is empty", "pointKey": "00000000-0000-0000-0000-00000000000a", "hmiName": "MISSING_CHANNEL" },
        { "code": "POINT_MISSING_PROFILE", "message": "point references channelName='tcp-missing' but no profile exists; defaulted to TCP sheet", "pointKey": "00000000-0000-0000-0000-00000000000b", "hmiName": "MISSING_PROFILE" }
      ],
      "diagnostics": { "exportedRows": { "tcp": 3, "rtu": 0, "params": 1 }, "durationMs": 5 }
    }
    ```

- **验收自检**：
  - [x] XLSX 冻结列名/列顺序未变更（只通过返回结构输出 warnings/diagnostics）
  - [x] DTO 契约：仅新增可选字段（`warnings/diagnostics/runWarnings`），旧字段语义不变
  - [x] command 不阻塞 UI：本任务未引入 command 内长循环；`comm_run_latest` 仍只读缓存
  - [x] warnings 可观测：后端返回值携带；前端 Export 页可展示
  - [x] mock demo 不受影响（driver/mock 仍为默认）

- **风险/未决项**：
  - `scale` 的 NaN/Inf 在标准 JSON 中不可表示：主要用于“从非标准输入/内部构造”时的兜底；实际 UI 输入仍需前端校验。
  - warnings 的 code 目前为 MVP 字符串常量，后续若要对接其他模块建议冻结一份 code 列表（避免跨模块不一致）。

