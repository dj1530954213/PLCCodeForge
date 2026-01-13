# TASK-26-result.md

- **Task 编号与标题**：
  - TASK-26：pointKey 诊断工具：复用命中来源可解释 + 冲突报告可导出

- **完成摘要**：
  - mapper（`unionToCommPoints`）新增返回 `decisions`（每个点位的 pointKey 复用决策来源：keyV2/keyV2NoDevice/keyV1/new），便于现场解释“为什么复用/为什么新建”。
  - 在 `联合导入` 页面新增“复用诊断”表格（hmiName/channelName/deviceId/pointKey/reuseDecision）并支持过滤（只看 created、只看 warnings 相关）。
  - 新增冲突报告导出：可下载 `conflict_report.json`（包含冲突的旧 pointKey 列表与旧点位摘要），用于现场排障；不写入 points.v1.json（不改变交付数据结构）。

- **改动清单（文件路径 + 关键点）**：
  - Frontend
    - `src/comm/mappers/unionToCommPoints.ts`
      - 返回新增：`decisions`（仅前端诊断字段）
      - 返回新增：`conflictReport`（keyV1/keyV2NoDevice 冲突聚合）
      - 不修改 `CommPoint` DTO（避免契约漂移）
    - `src/comm/pages/ImportUnion.vue`
      - 新增：复用诊断表格 + 过滤开关
      - 新增：导出 `conflict_report.json` 按钮（浏览器下载）
    - `src/comm/api.ts`
      - `CommWarning` 扩展可选字段：`channelName?`、`deviceId?`（向后兼容）

- **完成证据（build/test）**：
  - `pnpm build`：
    ```text
    > vue-tsc --noEmit && vite build
    ✓ built in 3.55s
    ```
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`（本任务无 Rust 逻辑改动，仅贴一次全量通过证据）：
    ```text
    test result: ok. 26 passed; 0 failed
    ```

- **decisions 表格展示说明（字段齐全）**：
  - 页面：`通讯采集 → 联合导入`
  - 操作：点击 `导入并生成通讯点位`
  - 预期：出现“复用诊断（pointKey 决策可解释）”表格，列包含：
    - `hmiName` / `channelName` / `deviceId` / `pointKey` / `reuseDecision`
  - `reuseDecision` 值域：
    - `reused:keyV2` / `reused:keyV2NoDevice` / `reused:keyV1` / `created:new`

- **conflict_report.json 示例片段（至少一条冲突）**：
  ```json
  {
    "generatedAtUtc": "2026-01-03T00:00:00.000Z",
    "conflicts": [
      {
        "keyType": "keyV1",
        "hmiName": "流量",
        "pointKeys": ["PK_A","PK_B"],
        "points": [
          { "pointKey": "PK_A", "hmiName": "流量", "channelName": "tcp-1@1", "deviceId": 1 },
          { "pointKey": "PK_B", "hmiName": "流量", "channelName": "tcp-2@1", "deviceId": 1 }
        ]
      }
    ]
  }
  ```

- **验收自检**：
  - [x] 不破坏 `points.v1.json`：诊断信息仅存在于 UI/内存，未修改落盘 DTO 结构
  - [x] 复用来源可解释：每点位有 `reuseDecision`
  - [x] 冲突可观测可导出：可下载 `conflict_report.json` 用于排障

- **风险与未决项**：
  - `deviceId` 在缺少 `@<id>` 后缀且 profiles 无法唯一反推时可能为空；诊断表会显示为空并通过 warnings 暴露不确定性。

