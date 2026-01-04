# TASK-16-result.md

- **Task 编号与标题**：
  - TASK-16：对齐真实 Modbus 地址语义（profile baseStart/len/area）+ plan/driver 一致性测试

- **完成摘要**：
  - 点位/通道地址语义对齐（内部统一 0-based）：
    - `ConnectionProfile.startAddress/length/readArea` 作为通道的 baseStart/范围/区域。
    - `CommPoint.addressOffset`（新增可选字段）表示“相对 profile.baseStart 的偏移”；缺省则保持旧行为（按 points 顺序自动顺排）。
  - `plan.rs`：
    - 构建 ReadJob 时，显式 offset 与自动顺排可共存，且输出按地址确定性排序，仍支持聚合/分批。
    - 新增 2 个单测锁定 baseStart!=0 与 offset 生效。
  - `modbus_tcp.rs` / `modbus_rtu.rs`：
    - 读操作严格使用 `ReadJob.startAddress/length`，无 `start=0` 硬编码（满足要求）。
  - IT（ENV gating）：
    - `src-tauri/tests/comm_it.rs` 在 `COMM_IT_ENABLE=1` 时打印 `job.start/job.len`（便于对照真机寄存器布局）。

- **改动清单（文件路径 + 关键点）**：
  - `src-tauri/src/comm/model.rs`
    - 新增：`CommPoint.addressOffset?: u16`（可选字段；缺省保持兼容旧行为）
  - `src-tauri/src/comm/plan.rs`
    - 计划构建：支持 `profile.startAddress + point.addressOffset` 生成点位地址
    - 兼容：`addressOffset=None` 仍按 points 顺序从 profile.startAddress 自动顺排
    - 确定性：显式 offset 可能导致地址乱序，计划在聚合前按地址排序以保证合并/分批稳定
    - 新增单测：
      - `plan_respects_profile_base_start_for_auto_mapping_when_base_start_is_not_zero`
      - `plan_uses_point_address_offset_relative_to_profile_base_start`
  - `src-tauri/tests/comm_it.rs`
    - 新增：`println!` 输出 `job.startAddress/job.length/readArea`（仅 COMM_IT_ENABLE=1 时会执行）

- **完成证据**：
  - `cargo test --manifest-path src-tauri/Cargo.toml`（新增 plan 单测出现在输出中）：
    ```text
    test comm::plan::tests::plan_respects_profile_base_start_for_auto_mapping_when_base_start_is_not_zero ... ok
    test comm::plan::tests::plan_uses_point_address_offset_relative_to_profile_base_start ... ok
    ```
  - plan 单测输入/输出断言片段（baseStart=100 -> job.start=100+offset）：
    ```rust
    let profiles = vec![tcp_profile_with_base_start("tcp-1", 100, 20)];
    let points = vec![
      point_with_offset("tcp-1", DataType::UInt16, Uuid::from_u128(1), 5),
      point_with_offset("tcp-1", DataType::UInt16, Uuid::from_u128(2), 6),
    ];
    let plan = build_read_plan(&profiles, &points, PlanOptions::default()).unwrap();
    assert_eq!(plan.jobs[0].start_address, 105);
    ```
  - driver 关键调用片段（证明未硬编码 0，贯穿 job.start/job.len）：
    - `src-tauri/src/comm/driver/modbus_tcp.rs`
      ```rust
      ctx.read_holding_registers(job.start_address, job.length).await
      ```

- **示例 JSON（points/profiles 片段）**：
  - profiles（内部 0-based baseStart=100）：
    ```json
    {
      "schemaVersion": 1,
      "profiles": [
        {
          "protocolType": "TCP",
          "channelName": "tcp-1",
          "deviceId": 1,
          "readArea": "Holding",
          "startAddress": 100,
          "length": 20,
          "ip": "127.0.0.1",
          "port": 502,
          "timeoutMs": 1000,
          "retryCount": 0,
          "pollIntervalMs": 500
        }
      ]
    }
    ```
  - points（addressOffset 相对 profile.startAddress）：
    ```json
    {
      "schemaVersion": 1,
      "points": [
        {
          "pointKey": "00000000-0000-0000-0000-000000000001",
          "hmiName": "TEMP_1",
          "dataType": "UInt16",
          "byteOrder": "ABCD",
          "channelName": "tcp-1",
          "addressOffset": 5,
          "scale": 1.0
        }
      ]
    }
    ```

- **验收自检**：
  - [x] UI 仍 1-based 输入（本任务未引入 40001/30001 风格解析）；内部仍统一 0-based
  - [x] plan 使用 profile 的 area/start/len，且支持 `profile.baseStart + point.addressOffset`
  - [x] driver 严禁硬编码 `start=0`：读操作使用 `ReadJob.startAddress/length`
  - [x] 新增 plan 单测锁定 baseStart!=0 的行为，防回归
  - [x] IT（COMM_IT_ENABLE=1）具备 `job.start/job.len` 打印位置（便于真机对照）

- **风险/未决项**：
  - 当前 offset 与自动顺排可共存：若同一 channel 中混用显式 offset 与缺省 offset，自动分配会避开已占用段，但仍建议后续冻结“混用策略/是否允许”的业务规则。
  - 若显式 offset 产生重叠地址，目前会以 `ExceedsChannelRange` 报错（MVP 兜底）；后续可细化为更明确的 overlap 错误码。

