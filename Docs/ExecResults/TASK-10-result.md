# TASK-10-result.md

- **Task 编号与标题**：
  - TASK-10：export_xlsx.rs（三张表冻结规范 + header const + 返回 headers）

- **完成摘要**：
  - 在 `src-tauri/src/comm/export_xlsx.rs` 使用 `rust_xlsxwriter` 导出 `通讯地址表.xlsx`，固定输出三张 sheet：`TCP通讯地址表` / `485通讯地址表` / `通讯参数`。
  - 三张表 headers 用 `const [&str]` 定义为唯一真源（列名/顺序逐字匹配冻结规范），并通过 `comm_export_xlsx` 返回值携带 headers 作为验收证据（同时兼容 `tcp/rtu/params` 与 `tcpSheet/rtu485Sheet/paramsSheet` 字段）。
  - 导出行顺序确定性：按 points 原始顺序输出；tie-break 用 `pointKey`（实现于导出函数内部排序逻辑）。

- **改动清单**：
  - `src-tauri/src/comm/export_xlsx.rs`
    - 新增：冻结 headers 常量 `HEADERS_TCP/HEADERS_RTU485/HEADERS_PARAMS`
    - 新增：导出实现 `export_comm_address_xlsx(out_path, profiles, points) -> ExportHeaders`
    - 新增：确定性排序（按 points index + pointKey）
    - 更新：单测打印 outPath 与 headers（便于验收回填到 result.md）
  - `src-tauri/src/comm/tauri_api.rs`
    - `comm_export_xlsx` 返回值 `headers` 同时提供：
      - `headers.tcp/rtu/params`（冻结验收口径）
      - `headers.tcpSheet/rtu485Sheet/paramsSheet`（前端现有展示口径）
  - `Docs/ExecResults/TASK-10-result.md`
    - 新建/更新任务结果归档文件（本文件）

- **冻结 headers（const 代码片段）**：
  - `src-tauri/src/comm/export_xlsx.rs`：
    ```rust
    pub const HEADERS_TCP: [&str; 5] = [
        "变量名称（HMI）", "数据类型", "字节序", "起始TCP通道名称", "缩放倍数",
    ];
    pub const HEADERS_RTU485: [&str; 5] = [
        "变量名称（HMI）", "数据类型", "字节序", "起始485通道名称", "缩放倍数",
    ];
    pub const HEADERS_PARAMS: [&str; 14] = [
        "协议类型", "通道名称", "设备标识", "读取区域", "起始地址", "长度",
        "TCP:IP / 485:串口", "TCP:端口 / 485:波特率", "485:校验", "485:数据位", "485:停止位",
        "超时ms", "重试次数", "轮询周期ms",
    ];
    ```

- **完成证据**：
  - `cargo build --manifest-path src-tauri/Cargo.toml` 输出片段：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.45s
    ```
  - 导出成功 outPath（单测实际生成文件路径）：
    - `C:\\Users\\DELL\\AppData\\Local\\Temp\\PLCCodeForge-TASK-10-通讯地址表-bf5b2299-ce29-42ed-8fb2-322bb5cc4704.xlsx`
  - `comm_export_xlsx` 返回的 headers（tcp/rtu/params 三组；文本逐字对比冻结规范）：
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
  - [x] 三张 sheet 名称固定：`TCP通讯地址表`/`485通讯地址表`/`通讯参数`
  - [x] headers 用 `const [&str]` 定义且逐字逐序匹配冻结规范
  - [x] `comm_export_xlsx` 返回值携带 headers（含 `tcp/rtu/params`）
  - [x] 导出行顺序确定性（按 points 原始顺序；tie-break pointKey）
  - [x] 缺字段不 panic：不适用字段填空字符串/0（导出过程无 unwrap；错误通过 `Result` 返回）
  - [x] `Docs/ExecResults/TASK-10-result.md` 已归档

- **风险/未决项**：
  - 点位引用不存在的 `channelName` 时会被跳过写入（不 panic）；如需强制提示，可在导出前做校验并返回结构化错误。

