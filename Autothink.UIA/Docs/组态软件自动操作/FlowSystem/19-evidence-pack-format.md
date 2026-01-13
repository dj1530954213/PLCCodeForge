# Evidence Pack 结构

## 文件清单
- step_logs.json：完整 StepLog。
- summary.json：流程摘要与失败原因。
- profile_version.txt：Profile 版本记录。
- flow_version.txt：Flow 版本记录。
- runtime_context.json：运行环境信息（可选）。
- screenshots/：关键步骤截图（可选）。

## summary.json 示例
```json
{
  "flowId": "comm-full",
  "flowVersion": "1.0.0",
  "startedAtUtc": "2026-01-08T08:00:00Z",
  "finishedAtUtc": "2026-01-08T08:10:00Z",
  "ok": false,
  "error": { "kind": "FindError", "message": "Element not found" },
  "tasks": [
    { "id": "import_variables", "ok": true },
    { "id": "hardware_config", "ok": false }
  ]
}
```

## step_logs.json 字段说明
- stepId：步骤标识
- action：动作名
- params：参数摘要
- outcome：Success/Warning/Fail
- durationMs：耗时
- error：错误信息（可选）
- uiState/windowTitle：可选辅助定位字段

## step_logs.json 示例
```json
{
  "steps": [
    {
      "stepId": "FindElement",
      "action": "FindElement",
      "outcome": "Fail",
      "durationMs": 5000,
      "error": { "kind": "FindError", "message": "Element not found" }
    }
  ]
}
```

## 截图命名建议
- {stepId}_{timestamp}.png
- 与 StepLog 通过 ScreenshotId 关联
