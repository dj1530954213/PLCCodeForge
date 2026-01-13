# TASK-21-result.md

- **Task 编号与标题**：
  - TASK-21：联合导入 → CommPoint 映射 → AppData 落盘 → Run 直接可跑（mock 优先）

- **完成摘要**：
  - 联合导入页新增“一键落盘”流程：导入成功后，将导入结果映射为通讯点位（生成/复用 `pointKey`）并调用 `comm_points_save` / `comm_profiles_save` 写入 AppData。
  - 完成后可直接跳转到 Run 页，用默认 Mock driver 启动采集并看到 `valueDisplay/quality/errorMessage/timestamp/durationMs/stats`。

- **改动清单（文件路径 + 关键点）**：
  - `src/comm/mappers/unionToCommPoints.ts`
    - 新增：ImportUnion points → CommPoint 的映射器
    - 规则：
      - `pointKey`：优先按 `hmiName` 复用已保存点位的 `pointKey`；否则生成 `crypto.randomUUID()`（uuidv4）
      - 处理重复 `hmiName`：后续重复项重新生成 `pointKey` 并产出 warning
      - 对缺失 `hmiName/channelName`、`dataType/byteOrder==Unknown` 的行：warning + 跳过
      - 大数据量：每 500 行 `setTimeout(0)` 让出 UI
  - `src/comm/pages/ImportUnion.vue`
    - 新增按钮：`导入并生成通讯点位`
    - 导入成功后：
      - 调用映射器生成 `PointsV1`
      - 调用 `comm_points_save(points)` 落盘到 `AppData/<app>/comm/points.v1.json`
      - profiles：与已有 profiles 做合并（key=`protocolType|channelName|deviceId`，existing-first），再 `comm_profiles_save`
    - 新增快捷入口：跳转到 `Points` / `Run`
    - UI 展示 warnings：import warnings + mapper warnings 合并展示

- **完成证据（build/test）**：
  - `pnpm build`：
    ```text
    vite v6.4.1 building for production...
    ✓ built in 3.50s
    ```
  - `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`（本任务前端为主，但仓库整体构建通过）：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.44s
    ```
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`：
    ```text
    test result: ok. 26 passed; 0 failed
    ```

- **演示步骤（导入成功 → 生成点位 → Run 可跑）**：
  1. 进入菜单 `联合导入`（路由：`/comm/import-union`）
  2. 填写 `文件路径`，按需选择 `strict/sheetName/addressBase`
  3. 点击 `导入并生成通讯点位`
     - 页面显示“落盘结果（AppData/comm/*.v1.json）”：points/profiles 数量、pointKey 复用/新建数量
  4. 点击 `打开 Run（Mock 可直接跑）`
  5. Run 页点击 `Start`：
     - 每 1s 轮询 `latest`
     - 结果表可见：`valueDisplay/quality/errorMessage/timestamp/durationMs` + stats

- **points.v1.json 片段（示例，schemaVersion + pointKey）**：
  - 文件落点：`AppData/<app-name>/comm/points.v1.json`
  - 内容片段（示例）：
    ```json
    {
      "schemaVersion": 1,
      "points": [
        {
          "pointKey": "b5e5b1c7-2c12-4b0c-8b0f-1f2a8d7a6c10",
          "hmiName": "TEMP_1",
          "dataType": "UInt16",
          "byteOrder": "ABCD",
          "channelName": "tcp-1",
          "scale": 1.0
        }
      ]
    }
    ```

- **warnings 示例与 UI 位置**：
  - 示例（mapper 产生）：
    ```json
    {
      "code": "IMPORTED_POINT_MISSING_CHANNEL_SKIPPED",
      "message": "imported point hmiName='TEMP_1' missing channelName; skipped",
      "hmiName": "TEMP_1"
    }
    ```
  - UI 位置：`联合导入` 页面底部卡片 `warnings（import + mapper）`

- **验收自检**：
  - [x] 导入成功后可一键生成并保存 points 到 `points.v1.json`（schemaVersion=1）
  - [x] `pointKey` 不可变策略：按 `hmiName` 复用旧 `pointKey`；新点生成 uuidv4
  - [x] 默认 demo 仍使用 mock driver；Run 页默认 driver=Mock
  - [x] UI 结果可观测：Run 页显示 `valueDisplay/quality/errorMessage/timestamp/durationMs/stats`
  - [x] 不阻塞 UI：导入解析在后端 `spawn_blocking`；前端映射循环分片让出 UI

- **风险与未决项**：
  - 当前 `pointKey` 复用以 `hmiName` 为匹配键：若现场存在同名变量（不同通道/设备），需后续升级为更稳定的业务匹配键（例如 `hmiName+channelName+deviceId`）并明确迁移策略。
  - profiles 合并策略为 existing-first：若希望导入覆盖已存在 profile 参数，需要额外提供“覆盖模式”开关与冲突提示。

