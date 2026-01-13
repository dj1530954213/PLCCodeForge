# TASK-02-result.md

- **Task 编号与标题**：
  - TASK-02：模型与 DTO（冻结契约 + schemaVersion=1）

- **完成摘要**：
  - 在 `Tauri.CommMapping/src-tauri/src/comm/model.rs` 落地稳定 DTO：`ConnectionProfile` / `CommPoint` / `SampleResult` / `RunStats`。
  - 引入并固化 `pointKey`（UUID，运行期稳定主键）与 `hmiName`（变量名称/HMI，业务键）。
  - 增加持久化顶层结构 `ProfilesV1` / `PointsV1`，顶层字段包含 `schemaVersion: 1`。
  - 补齐 serde roundtrip 单测，锁定 JSON 字段为 camelCase（避免前后端字段漂移）。

- **改动清单**：
  - `Tauri.CommMapping/src-tauri/src/comm/model.rs`
    - 新增：`SCHEMA_VERSION_V1 = 1`
    - 新增 DTO：`DataType`、`ByteOrder32`、`RegisterArea`、`SerialParity`、`Quality`
    - 新增 DTO：`ConnectionProfile`（`protocolType: "TCP" | "485"`，字段 camelCase）
    - 新增 DTO：`CommPoint`（包含 `pointKey` + `hmiName` + `dataType` + `byteOrder` + `channelName` + `scale`）
    - 新增 DTO：`SampleResult`、`RunStats`
    - 新增持久化顶层：`ProfilesV1`、`PointsV1`（顶层 `schemaVersion`）
    - 新增单测：serde roundtrip + JSON 形态断言（含 `pointKey`）
  - `Tauri.CommMapping/Docs/ExecResults/TASK-02-result.md`
    - 新建任务结果归档文件（本文件）

- **关键实现说明**：
  - `ConnectionProfile` 采用 serde 内部 tag：`protocolType`，取值严格为 `"TCP"` / `"485"`，用于后续导出 `通讯参数` sheet 的“协议类型”列。
  - `CommPoint` 不直接存放 Modbus 地址；它通过 `channelName` 关联 `ConnectionProfile`，后续在 TASK-04（plan）内根据 points 顺序做“地址自动映射/聚合/分批”。
  - JSON 字段统一 camelCase；`ConnectionProfile` 使用 `rename_all_fields = "camelCase"` 锁死字段形态。

- **示例 JSON（片段）**：
  - `profiles.v1.json`（示例）：
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
          "length": 20,
          "ip": "127.0.0.1",
          "port": 502,
          "timeoutMs": 1000,
          "retryCount": 2,
          "pollIntervalMs": 500
        }
      ]
    }
    ```
  - `points.v1.json`（示例）：
    ```json
    {
      "schemaVersion": 1,
      "points": [
        {
          "pointKey": "00000000-0000-0000-0000-000000000001",
          "hmiName": "TANK_TEMP",
          "dataType": "Float32",
          "byteOrder": "ABCD",
          "channelName": "tcp-1",
          "scale": 1.0
        }
      ]
    }
    ```

- **完成证据**：
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml` 输出片段：
    ```text
    running 2 tests
    test comm::model::tests::points_v1_json_roundtrip_includes_schema_version_and_point_key ... ok
    test comm::model::tests::profiles_v1_json_roundtrip ... ok
    ```

- **验收自检**：
  - [x] `ConnectionProfile/CommPoint/SampleResult/RunStats` 已在 `Tauri.CommMapping/src-tauri/src/comm/model.rs` 定义。
  - [x] `CommPoint` 包含 `pointKey`（稳定键）与 `hmiName`（业务键）。
  - [x] 持久化结构顶层包含 `schemaVersion: 1`（`ProfilesV1`/`PointsV1`）。
  - [x] serde roundtrip 单测通过，且断言 JSON 为 camelCase（含 `pointKey`）。
  - [x] `Tauri.CommMapping/Docs/ExecResults/TASK-02-result.md` 已归档。

- **风险/未决项**：
  - DTO 一旦进入 `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs` 对外暴露将视为冻结契约；后续只允许新增可选字段，避免破坏前端兼容性。

- **下一步建议**：
  - 进入 TASK-03：实现 `codec.rs`（>=10 组测试向量，覆盖 `Bool/Int16/UInt16/Int32/UInt32/Float32` + `ABCD/BADC/CDAB/DCBA`，失败产出 `DecodeError`）。
