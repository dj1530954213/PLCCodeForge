# TASK-04-result.md

- **Task 编号与标题**：
  - TASK-04：plan（分组/聚合/分批/确定性）

- **完成摘要**：
  - 在 `Tauri.CommMapping/src-tauri/src/comm/plan.rs` 实现 `ReadPlan` 构建：按 `channelName` 分组、按 points 顺序做地址自动映射、对连续地址聚合并按最大长度分批。
  - 输出顺序确定性：channel 按“首次出现的 point 下标”排序；channel 内 points 按（points 下标，pointKey）排序，避免 HashMap 迭代导致的非确定性。
  - 补齐 2 个单测：聚合/分批正确、输出顺序稳定。

- **改动清单**：
  - `Tauri.CommMapping/src-tauri/src/comm/plan.rs`
    - 新增类型：`PlanOptions`、`ReadPlan`、`ReadJob`、`PlannedPointRead`
    - 新增错误：`PlanError`（缺少 profile、重复通道名、区域与数据类型不匹配、超范围、point 超过单 job 上限）
    - 新增入口：`build_read_plan(profiles, points, options) -> ReadPlan`
    - 新增单测：
      - `plan_aggregates_contiguous_and_splits_by_max_registers`
      - `plan_output_order_is_stable_by_first_point_index`
  - `Tauri.CommMapping/Docs/ExecResults/TASK-04-result.md`
    - 新建任务结果归档文件（本文件）

- **关键实现说明**：
  - 地址自动映射（每个 channel 内）：从 `ConnectionProfile.startAddress`（内部 0-based）开始，按 points 顺序累加占用长度（16-bit=1 reg，32-bit=2 regs，Bool=1 coil）。
  - 聚合/分批：对连续地址聚合为同一个 job；若累计长度超过 `maxRegistersPerJob/maxCoilsPerJob` 则切分为新 job。
  - 稳定性：不依赖 HashMap 的迭代顺序，显式排序得到可复现的 jobs 列表与 job 内 points 顺序。

- **plan 输出示例（片段）**：
  - 示例：`tcp-1` 下 3 个点位（Int16/UInt16/Int32），`maxRegistersPerJob=2`：
    ```json
    {
      "jobs": [
        {
          "channelName": "tcp-1",
          "readArea": "Holding",
          "startAddress": 0,
          "length": 2,
          "points": [
            { "pointKey": "00000000-0000-0000-0000-000000000000", "dataType": "Int16", "byteOrder": "ABCD", "scale": 1.0, "offset": 0, "length": 1 },
            { "pointKey": "00000000-0000-0000-0000-000000000001", "dataType": "UInt16", "byteOrder": "ABCD", "scale": 1.0, "offset": 1, "length": 1 }
          ]
        },
        {
          "channelName": "tcp-1",
          "readArea": "Holding",
          "startAddress": 2,
          "length": 2,
          "points": [
            { "pointKey": "00000000-0000-0000-0000-000000000002", "dataType": "Int32", "byteOrder": "ABCD", "scale": 1.0, "offset": 0, "length": 2 }
          ]
        }
      ]
    }
    ```

- **完成证据**：
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml` 输出片段：
    ```text
    running 6 tests
    test comm::plan::tests::plan_aggregates_contiguous_and_splits_by_max_registers ... ok
    test comm::plan::tests::plan_output_order_is_stable_by_first_point_index ... ok
    ```

- **验收自检**：
  - [x] 按 `channelName` 分组并构建 ReadJobs。
  - [x] 连续地址聚合 + 按 max 长度分批。
  - [x] 输出顺序确定性（按 points 顺序 + pointKey tie-break）。
  - [x] ≥2 个单测覆盖聚合正确与排序稳定。
  - [x] `cargo test` 通过并输出测试名。
  - [x] `Tauri.CommMapping/Docs/ExecResults/TASK-04-result.md` 已归档。

- **风险/未决项**：
  - 当前 `ConnectionProfile.length` 被视为 channel 可用地址总长度（寄存器/线圈数）；后续若引入“动态扩容/多段地址”，需要扩展模型（只能加可选字段）。

- **下一步建议**：
  - 进入 TASK-05：实现 `driver/mock.rs`，让 engine 在无真实 PLC 环境下能稳定产出 OK/Timeout/DecodeError 与统计。
