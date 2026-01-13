# TASK-28-result.md

- **Task 编号与标题**：
  - TASK-28：诊断联动（decisions/冲突）→ 落盘与导出提示：可解释的质量门禁

- **完成摘要**：
  - ImportUnion 页面新增“诊断摘要卡片”，汇总复用决策与冲突数量，形成现场可解释的质量提示。
  - 当检测到冲突（尤其 keyV1=hmiName-only 冲突）时：
    - UI 明确提示“建议先修复 channelName/deviceId”
    - 交付导出（Wizard 的 export 步骤）会弹出二次确认对话框（必须勾选“我已知晓冲突风险”才可继续）。
  - 与 TASK-27 联动：导出证据包时若存在冲突，自动包含 `conflict_report.json`。

- **改动清单（文件路径 + 关键点）**：
  - `src/comm/pages/ImportUnion.vue`
    - 新增：诊断摘要卡片（reused:keyV2 / reused:keyV2NoDevice / reused:keyV1 / created:new / conflicts / keyV1Conflicts）
    - 新增：冲突门禁二次确认 Dialog（checkbox 确认）
  - `src/comm/services/demoPipeline.ts`
    - export 前 gate：检测 `conflictReport.conflicts>0` 时调用 `confirmExportWithConflicts` 回调，否则取消导出
  - `src/comm/services/evidencePack.ts`
    - 冲突存在时自动附带 `conflict_report.json`

- **完成证据（build/test）**：
  - `pnpm build`：
    ```text
    vite v6.4.1 building for production...
    ✓ built in 3.51s
    ```

- **诊断摘要卡片示例（文字描述）**：
  - 页面：`通讯采集 → 联合导入`
  - 完成 `导入并生成通讯点位` 或 `一键演示（Wizard）` 后，摘要卡片会显示：
    - `reused:keyV2 = N1`
    - `reused:keyV2NoDevice = N2`
    - `reused:keyV1 = N3`
    - `created:new = N4`
    - `conflicts = C`
    - `keyV1Conflicts = C1`

- **冲突存在时导出二次确认（步骤说明）**：
  1) 准备一个存在冲突的旧数据（例如旧 points 中同一 hmiName 对应多个 pointKey）
  2) 点击：`一键演示（Wizard）`
  3) Wizard 在 export 前会弹出“冲突风险确认”对话框：
     - 展示 conflictReport 详情（JSON）
     - 必须勾选 `我已知晓冲突风险，仍要继续导出交付表`
     - 未勾选或点击取消 → 导出被取消（不会静默）

- **自检**：
  - [x] 不改 `CommPoint` DTO、不改 `points.v1.json` 落盘结构：诊断信息仅 UI/内存使用
  - [x] 冲突提示可观测：摘要卡片 + warning alert
  - [x] 导出门禁可执行：冲突时强制二次确认后才能继续 export
  - [x] 与证据包联动：conflicts>0 自动包含 `conflict_report.json`

- **风险与未决项**：
  - 当前门禁策略为 MVP 写死（conflicts>0 即触发），若未来需要按冲突类型细分（仅 keyV1 强制、keyV2NoDevice 仅提示），需在 v2 调整规则并更新验收口径。

