# TASK-14-result.md

- **Task 编号与标题**：
  - TASK-14：集成测试（ENV gating + 真 Modbus TCP/RTU 服务）

- **完成摘要**：
  - 新增集成测试 `Tauri.CommMapping/src-tauri/tests/comm_it.rs`：
    - **ENV gating（冻结口径）**：仅当 `COMM_IT_ENABLE=1` 且配置必要环境变量时才会尝试真实连接；否则打印 `SKIP ...` 并直接返回（测试通过）。
    - **TCP 用例**：读取 2 个点（`UInt16` + `Float32`），走 `plan/engine/codec` 全链路，断言 `quality==Ok` 且 `valueDisplay` 可解析；同时打印 `RawReadData`（原始寄存器）便于人工核对。
    - **RTU 用例框架**：存在 1 个点的通路验证（同样打印 raw + 断言 `quality==Ok`）。
  - 为了让 integration test 可引用 comm 模块：`Tauri.CommMapping/src-tauri/src/lib.rs` 将 `comm` 提升为 `pub mod comm;`（不影响 Tauri command 契约）。

- **改动清单**：
  - `Tauri.CommMapping/src-tauri/src/lib.rs`
    - 调整：`pub mod comm;`（供 `Tauri.CommMapping/src-tauri/tests/*.rs` 引用）
  - `Tauri.CommMapping/src-tauri/tests/comm_it.rs`
    - 调整：ENV 列表对齐冻结口径（仅 `COMM_IT_ENABLE` + TCP/RTU 固定字段）
    - 新增：无 ENV 时的 `SKIP` 输出；打印 `RawReadData` 作为人工核对证据
  - `Tauri.CommMapping/Docs/ExecResults/TASK-14-result.md`
    - 更新：按冻结 ENV 口径修订说明与证据

- **完成证据**：
  - **无 ENV 时自动 skip（可见输出，使用 `-- --nocapture`）**：
    ```text
    running 2 tests
    SKIP tcp_quality_ok_for_two_points_when_enabled: COMM_IT_ENABLE!=1
    SKIP rtu_quality_ok_for_one_point_when_enabled: COMM_IT_ENABLE!=1
    test tcp_quality_ok_for_two_points_when_enabled ... ok
    test rtu_quality_ok_for_one_point_when_enabled ... ok
    ```
  - **tests 文件路径与核心代码片段**：
    - `Tauri.CommMapping/src-tauri/tests/comm_it.rs`
      ```rust
      if env::var("COMM_IT_ENABLE").ok().as_deref() != Some("1") {
          println!("SKIP tcp_quality_ok_for_two_points_when_enabled: COMM_IT_ENABLE!=1");
          return;
      }
      ```

- **如何启用（运行命令/环境变量）**：
  - **总开关**：
    - `COMM_IT_ENABLE=1`
  - **TCP（Holding，冻结 ENV）**：
    - `COMM_IT_TCP_HOST`（例：`127.0.0.1`）
    - `COMM_IT_TCP_PORT`（例：`502`）
    - `COMM_IT_TCP_UNITID`（例：`1`）
  - **RTU（Holding，冻结 ENV）**：
    - `COMM_IT_RTU_PORT`（例：Windows `COM3` / Linux `/dev/ttyUSB0`）
    - `COMM_IT_RTU_BAUD`（例：`9600`）
    - `COMM_IT_RTU_PARITY`（`None|Even|Odd`）
    - `COMM_IT_RTU_DATABITS`（例：`8`）
    - `COMM_IT_RTU_STOPBITS`（例：`1`）
    - `COMM_IT_RTU_SLAVEID`（例：`1`）
  - **运行命令**：
    - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml --test comm_it -- --nocapture`

- **COMM_IT_ENABLE=1 跑通输出片段**：
  - 待你方提供真实 Modbus TCP/RTU 服务后补贴（本仓库默认不提供真实 PLC/端口依赖，符合 mock 优先）。

- **验收自检**：
  - [x] `COMM_IT_ENABLE!=1` 默认 skip（不依赖真实 PLC/端口）
  - [x] ENV 列表对齐冻结口径（仅 `COMM_IT_ENABLE` + TCP/RTU 固定字段）
  - [x] TCP 用例至少读取 2 点（16-bit + float32），断言 `quality=Ok` + `valueDisplay` 可解析且非空
  - [x] RTU 用例框架存在（未配置时自动 skip）
  - [x] 无 ENV 时可输出 `SKIP ...` 片段用于验收粘贴
  - [x] `Tauri.CommMapping/Docs/ExecResults/TASK-14-result.md` 已归档

- **风险/未决项**：
  - 当前联调用例的 profile `startAddress` 固定为内部 0-based 的 `0`，点位按 `plan` 的顺序映射连续地址；若你方真机地址不从 0 开始，需要补充冻结口径（是否允许新增 ENV / 或改为从 profiles/points 配置读取）。
  - 若需断言“具体期望值”，需要你方提供寄存器写入策略或固定测试工装；当前用例为 smoke（解析成功 + raw 寄存器输出供人工核对）。
