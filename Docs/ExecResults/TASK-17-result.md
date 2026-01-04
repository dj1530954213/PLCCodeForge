# TASK-17-result.md

- **Task 编号与标题**：
  - TASK-17：实现“联合 xlsx（IO+设备表）→ CommPoint/Profiles”映射层（可融合接口）

- **完成摘要**：
  - 新增后端映射器：读取联合 xlsx（取第一张 sheet），生成：
    - `PointsV1`（含 `pointKey` 稳定生成规则）
    - `ProfilesV1`（按通道聚合生成 TCP/485 profile；缺参生成 skeleton + warnings）
    - `warnings`（缺字段/冲突/未知枚举/重复点位等）
  - 新增 Tauri command：
    - `comm_import_union_xlsx(path) -> { points, profiles, warnings }`
    - 解析在 `spawn_blocking` 中执行，避免阻塞 UI。

- **改动清单（文件路径 + 关键点）**：
  - `src-tauri/Cargo.toml`
    - 新增依赖：`calamine`（xlsx 读取）
    - `uuid` 开启 `v5`（用于确定性 pointKey）
  - `src-tauri/src/comm/import_union_xlsx.rs`
    - 新增：联合 xlsx 解析与映射逻辑（points/profiles/warnings）
    - pointKey：UUID v5（SHA1）确定性生成；重复点位 first-wins + warning
    - profile：按通道聚合；若同一通道出现多个 deviceId，自动 disambiguate `channelName@deviceId` + warning
  - `src-tauri/src/comm/mod.rs`
    - 新增：`pub mod import_union_xlsx;`
  - `src-tauri/src/comm/tauri_api.rs`
    - 新增 command：`comm_import_union_xlsx`
    - 新增 response DTO：`CommImportUnionXlsxResponse`
  - `src-tauri/src/lib.rs`
    - 注册：`comm_import_union_xlsx` 到 `invoke_handler`
  - `src/comm/api.ts`
    - 新增：`commImportUnionXlsx()` 封装与返回类型

- **完成证据**：
  - `cargo build --manifest-path src-tauri/Cargo.toml`：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.75s
    ```
  - 新增 command 的签名与注册位置：
    - `src-tauri/src/comm/tauri_api.rs`
      ```rust
      #[tauri::command]
      pub async fn comm_import_union_xlsx(path: String) -> Result<CommImportUnionXlsxResponse, String>
      ```
    - `src-tauri/src/lib.rs`：`invoke_handler` 中包含 `comm_import_union_xlsx`

- **pointKey 生成算法（冻结规则说明）**：
  - 目标：同一份导入输入中，同一 `变量名称（HMI） + 通道名称 + 设备标识` 生成稳定 pointKey，禁止随机 uuid。
  - 实现：UUID v5（SHA1）
    - namespace：`POINTKEY_NAMESPACE`（常量）
    - name：`"{hmiName}|{baseChannelName}|{deviceId}"`
    - `Uuid::new_v5(&POINTKEY_NAMESPACE, name.as_bytes())`
  - 去重策略（拍板）：
    - 若生成的 pointKey 重复：返回 warning `DUPLICATE_POINT_KEY_SKIP`，并 **保留第一条，跳过后续重复行**。

- **示例输入/输出（用列名模拟最小联合表）**：
  - 最小输入（示意：第一张 sheet 的表头 + 1 行数据）：
    ```json
    {
      "sheet[0]": [
        {
          "变量名称（HMI）": "TEMP_1",
          "数据类型": "UInt16",
          "字节序": "ABCD",
          "通道名称": "tcp-1",
          "缩放倍数": 1.0,
          "协议类型": "TCP",
          "设备标识": 1,
          "读取区域": "Holding",
          "起始地址": 0,
          "长度": 10,
          "TCP:IP / 485:串口": "192.168.0.10",
          "TCP:端口 / 485:波特率": 502
        }
      ]
    }
    ```
  - 输出 points/profiles（片段）：
    ```json
    {
      "points": {
        "schemaVersion": 1,
        "points": [
          {
            "pointKey": "<uuid-v5>",
            "hmiName": "TEMP_1",
            "dataType": "UInt16",
            "byteOrder": "ABCD",
            "channelName": "tcp-1",
            "scale": 1.0
          }
        ]
      },
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
            "ip": "192.168.0.10",
            "port": 502,
            "timeoutMs": 1000,
            "retryCount": 0,
            "pollIntervalMs": 1000
          }
        ]
      },
      "warnings": []
    }
    ```

- **warnings（至少 5 条，MVP 已覆盖）**：
  - `ROW_MISSING_HMI_NAME`：缺 HMI 名称 → 跳过该行
  - `ROW_MISSING_CHANNEL_NAME`：缺通道名称 → 仍可生成（会 default channelName 并提示）
  - `ROW_MISSING_DEVICE_ID_DEFAULT_1`：缺设备标识 → 默认 1
  - `ROW_DATATYPE_UNKNOWN_SKIP`：未知数据类型 → warning + 跳过该行
  - `ROW_BYTEORDER_UNKNOWN_DEFAULT_ABCD`：未知字节序 → warning + 默认 ABCD（仅对 32-bit 类型提示）
  - `PROFILE_TCP_PARAM_MISSING` / `PROFILE_RTU_PARAM_MISSING`：缺 ip/port 或 serial 参数 → 生成 skeleton profile + warning
  - `PROFILE_CHANNEL_DISAMBIGUATED`：同一通道出现多个 deviceId → 自动改为 `channelName@deviceId` + warning
  - `PROFILE_LENGTH_DEFAULTED`：profile.length 缺失 → 按点位需求推导最小长度 + warning
  - `DUPLICATE_POINT_KEY_SKIP`：重复点位（同 hmiName+通道+设备）→ 保留第一条

- **验收自检**：
  - [x] 新增 command：`comm_import_union_xlsx(path) -> { points, profiles, warnings }`
  - [x] pointKey 非随机：使用 UUID v5（SHA1）确定性生成
  - [x] dataType/byteOrder 仅支持冻结枚举；未知值返回 warning（不 panic）
  - [x] profiles 能按通道聚合生成；缺参数可生成 skeleton 并给出 warning
  - [x] command 避免阻塞 UI：解析在 `spawn_blocking` 执行

- **风险/未决项**：
  - 联合 xlsx 的真实列名/Sheet 名可能存在差异：当前按“第一张 sheet + 常见列名候选”解析；若你方冻结了具体列名/Sheet 名，需要补充一份映射表（即可做到 0 歧义）。
  - 当前对“起始地址/长度”的解释为内部 0-based（与通讯参数表一致）；若你的联合表采用 1-based，需要明确后再加转换规则（避免与 UI 1-based 混淆）。

