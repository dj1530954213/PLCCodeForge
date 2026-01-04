# TASK-12-result.md

- **Task 编号与标题**：
  - TASK-12：实现四页面（Connection/Points/Run/Export）+ mock 全链路

- **完成摘要**：
  - Connection：TCP/485 Profile 配置（MVP UI 仅暴露 Holding/Coil）；起始地址 UI 采用 1-based 显示/输入，保存到后端为内部 0-based。
  - Points：点位 CRUD + 支持批量设置 `dataType/byteOrder/channelName/scale` + JSON 导入/导出 + demo（mock）一键生成 5 点位示例。
  - Run：默认 Mock driver，start/stop + 每 1s 轮询 latest，展示 `valueDisplay/quality/errorMessage/timestamp/durationMs` 及 stats。
  - Export：调用 `comm_export_xlsx` 导出 xlsx 并展示返回 headers（用于验收逐字对比冻结规范）。

- **关键实现位置**：
  - Connection：`src/comm/pages/Connection.vue`
    - **地址语义（强制）**：UI 1-based ↔ 后端内部 0-based（代码：`setUiStartAddress(profile, uiValue) => startAddress = uiValue - 1`）。
    - **MVP 读取区域限制**：仅 `Holding/Coil`（`AREA_OPTIONS: ["Holding","Coil"]`）。
  - Points：`src/comm/pages/Points.vue`
    - CRUD + 批量设置 + JSON 导入/导出 + demo（mock）。
  - Run：`src/comm/pages/Run.vue`
    - driver 默认 `Mock`；1s 轮询 `comm_run_latest`；结果表字段齐全。
  - Export：`src/comm/pages/Export.vue`
    - 调用 `comm_export_xlsx` 并展示 `outPath` + headers 文本。
  - invoke 封装：`src/comm/api.ts`

- **验收证据**：
  - Points 示例（3~5 个点位，含不同 dataType/byteOrder；可直接在 Points 页“导入 JSON”使用）：
    ```json
    {
      "schemaVersion": 1,
      "points": [
        { "pointKey": "00000000-0000-0000-0000-000000000001", "hmiName": "OK_U16", "dataType": "UInt16", "byteOrder": "ABCD", "channelName": "tcp-ok", "scale": 1.0 },
        { "pointKey": "00000000-0000-0000-0000-000000000002", "hmiName": "OK_F32_CDAB", "dataType": "Float32", "byteOrder": "CDAB", "channelName": "tcp-ok", "scale": 0.1 },
        { "pointKey": "00000000-0000-0000-0000-000000000003", "hmiName": "OK_I32_DCBA", "dataType": "Int32", "byteOrder": "DCBA", "channelName": "tcp-ok", "scale": 1.0 },
        { "pointKey": "00000000-0000-0000-0000-000000000004", "hmiName": "TIMEOUT_U16", "dataType": "UInt16", "byteOrder": "ABCD", "channelName": "tcp-timeout", "scale": 1.0 },
        { "pointKey": "00000000-0000-0000-0000-000000000005", "hmiName": "DECODE_U32_BADC", "dataType": "UInt32", "byteOrder": "BADC", "channelName": "tcp-decode", "scale": 1.0 }
      ]
    }
    ```
  - Run（start -> latest 出现 OK/Timeout/DecodeError 的日志片段；mock，不依赖真实 PLC）：
    ```text
    runId=9b213770-310c-4a50-af9f-85dc8398603e updatedAtUtc=2026-01-02 15:10:35.480644 UTC stats=RunStats { total: 5, ok: 3, timeout: 1, comm_error: 0, decode_error: 1, config_error: 0 }
    row[0] pointKey=00000000-0000-0000-0000-000000000001 quality=Ok ...
    row[3] pointKey=00000000-0000-0000-0000-000000000004 quality=Timeout ... errorMessage='timeout' ...
    row[4] pointKey=00000000-0000-0000-0000-000000000005 quality=DecodeError ... errorMessage='insufficient registers: expected 2 got 1' ...
    ```
    - 生成方式：`cargo test --manifest-path src-tauri/Cargo.toml run_engine_latest_contains_ok_timeout_and_decode_error_when_using_mock -- --nocapture`
  - Export（导出成功 outPath + headers 文本；页面展示与后端返回一致）：
    - outPath（实际生成文件）：
      - `C:\\Users\\DELL\\AppData\\Local\\Temp\\PLCCodeForge-TASK-10-通讯地址表-bf5b2299-ce29-42ed-8fb2-322bb5cc4704.xlsx`
    - headers（冻结逐字逐序；来自返回值）：
      ```json
      {
        "headers": {
          "tcp": ["变量名称（HMI）","数据类型","字节序","起始TCP通道名称","缩放倍数"],
          "rtu": ["变量名称（HMI）","数据类型","字节序","起始485通道名称","缩放倍数"],
          "params": ["协议类型","通道名称","设备标识","读取区域","起始地址","长度","TCP:IP / 485:串口","TCP:端口 / 485:波特率","485:校验","485:数据位","485:停止位","超时ms","重试次数","轮询周期ms"]
        }
      }
      ```

- **验收自检**：
  - [x] 默认使用 Mock driver（demo 不依赖真实 PLC/端口）。
  - [x] Connection：仅暴露 Holding/Coil；startAddress UI 1-based，保存为内部 0-based。
  - [x] Points：CRUD + 批量设置 dataType/byteOrder/channelName/scale + JSON 导入/导出。
  - [x] Run：start/stop + 1s 轮询 latest；结果表显示 valueDisplay/quality/errorMessage/timestamp/durationMs + stats。
  - [x] Export：导出成功，展示 outPath + headers 文本（用于冻结规范验收）。
  - [x] `Docs/ExecResults/TASK-12-result.md` 已归档（本文件）。

- **风险/未决项**：
  - `valueDisplay` 对 Float32 目前为直接字符串格式（可能较长）；如需冻结显示格式（小数位/科学计数法），需补充规则并加测试锁定。
