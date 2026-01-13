# TASK-22-result.md

- **Task 编号与标题**：
  - TASK-22：导出最终交付：通讯地址表.xlsx（三张表冻结）+ 可选附加 Results（对齐 pointKey/HMI）

- **完成摘要**：
  - 新增交付版导出命令：`comm_export_delivery_xlsx`，使用 `spawn_blocking` 写入 xlsx，保证 command 不阻塞 UI。
  - 交付版默认输出三张冻结表（TCP/485/通讯参数），列名与列顺序逐字匹配冻结规范；可选附加第 4 张 `采集结果` sheet（不影响冻结三表）。
  - 前端 Export 页新增“交付导出（通讯地址表.xlsx）”按钮，并展示返回 headers 作为验收证据。

- **改动清单（文件路径 + 关键点）**：
  - Rust
    - `Tauri.CommMapping/src-tauri/src/comm/export_delivery_xlsx.rs`
      - 新增：交付版 workbook 生成（三张冻结表 + 可选 Results）
      - headers const：`TCP_HEADERS_V1` / `RTU485_HEADERS_V1` / `PARAM_HEADERS_V1`（引用冻结常量值）
      - 点位排序：按 points 原始顺序，tie-break `pointKey`
      - 参数表排序：按 `channelName + deviceId` 稳定排序
    - `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
      - 新增 command：`comm_export_delivery_xlsx(request)`（async + spawn_blocking）
      - 新增 DTO：`CommExportDeliveryXlsxRequest/Response`
    - `Tauri.CommMapping/src-tauri/src/lib.rs`
      - 注册 command：`comm_export_delivery_xlsx`
    - `Tauri.CommMapping/src-tauri/src/comm/mod.rs`
      - 导出模块：`pub mod export_delivery_xlsx;`
  - Frontend
    - `src/comm/api.ts`
      - 新增：`commExportDeliveryXlsx()` 封装与返回类型
    - `src/comm/pages/Export.vue`
      - 新增交付导出按钮 + `includeResults` 勾选框
      - 导出后展示 headers（tcp/rtu/params）

- **完成证据（build/test）**：
  - `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.44s
    ```
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`：
    ```text
    test result: ok. 26 passed; 0 failed
    ```
  - `pnpm build`：
    ```text
    vite v6.4.1 building for production...
    ✓ built in 3.50s
    ```

- **冻结 headers（必须逐字逐序）**：
  - TCP通讯地址表（5列）：
    ```text
    ["变量名称（HMI）","数据类型","字节序","起始TCP通道名称","缩放倍数"]
    ```
  - 485通讯地址表（5列）：
    ```text
    ["变量名称（HMI）","数据类型","字节序","起始485通道名称","缩放倍数"]
    ```
  - 通讯参数（14列）：
    ```text
    ["协议类型","通道名称","设备标识","读取区域","起始地址","长度","TCP:IP / 485:串口","TCP:端口 / 485:波特率","485:校验","485:数据位","485:停止位","超时ms","重试次数","轮询周期ms"]
    ```

- **导出路径示例**：
  - Export 页填写：`C:\temp\通讯地址表.xlsx`
  - 点击：`交付导出（通讯地址表.xlsx）`
  - UI 将显示：`outPath` + headers（用于逐字对比验收）

- **验收自检**：
  - [x] 三张固定表列名/顺序逐字冻结（未改名/未增删/未调序）
  - [x] `pointKey` 不出现在三张交付表中；内部按 `pointKey` 对齐（Results sheet 可选，仅展示 HMI）
  - [x] 排序稳定：points 按原始顺序 + pointKey tie-break
  - [x] command 不阻塞 UI：写文件在 `spawn_blocking` 执行
  - [x] 返回值携带 headers（tcp/rtu/params）作为验收证据

- **风险与未决项**：
  - 可选 Results sheet 当前仅在存在 `last_results.v1.json` 或有结果缓存时可写入；若现场需要“必须附带结果”，需要明确结果来源（Run latest 传入 vs AppData 读取）与缺失时策略。

