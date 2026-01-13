# TASK-S2-29-result.md

## 完成摘要
- Stage2Runner 新增 UIStateRecovery 处理管线：在关键动作前/失败时尝试处理异常弹窗（最多 2 次），记录 StepLog 并落盘 `unexpected_ui_state.json`。
- 新增 v1 pack global.* selector keys（popupRoot/Ok/No/WarningText），支持本地覆盖。
- DemoTarget 增加启动提示弹窗，Runner 可自动关闭并继续执行。

## 改动清单
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`：UIStateRecovery 管线、`unexpected_ui_state.json` 落盘、summary.uiStateRecovery 字段。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.v1.base.json`：新增 `global.*` keys。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.v1.local.sample.json`：新增 `global.*` keys 模板。
- `Autothink.UIA/Autothink.UiaAgent.DemoTarget/MainForm.cs`：启动提示弹窗（OK）。
- `Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 UIStateRecovery 使用说明。

## Build/Test 证据
```text
dotnet build Autothink.UIA/PLCCodeForge.sln -c Release
Autothink.UiaAgent.DemoTarget -> ...\Autothink.UiaAgent.DemoTarget.dll
Autothink.UiaAgent -> ...\Autothink.UiaAgent.dll
Autothink.UiaAgent.WinFormsHarness -> ...\Autothink.UiaAgent.WinFormsHarness.dll
Autothink.UiaAgent.Tests -> ...\Autothink.UiaAgent.Tests.dll
Autothink.UiaAgent.Stage2Runner -> ...\Autothink.UiaAgent.Stage2Runner.dll
已成功生成。

dotnet test Autothink.UIA/PLCCodeForge.sln -c Release
已通过! - 失败: 0，通过: 34，已跳过: 1，总计: 35
```

## DemoTarget 弹窗处理步骤（可复现）
1. 启动 DemoTarget（Release）。
2. 运行 Stage2Runner：`Autothink.UiaAgent.Stage2Runner.exe --config Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`
3. 启动提示弹窗自动被关闭，流程继续执行。

## summary.json 片段（uiStateRecovery）
来自 `Autothink.UIA/logs/20260104-004044/summary.json`：
```json
{
  "uiStateRecovery": {
    "attempts": 3,
    "handled": 1,
    "lastHandler": "GenericOk",
    "evidencePath": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\logs\\20260104-004044\\unexpected_ui_state.json"
  }
}
```

## unexpected_ui_state.json 片段
来自 `Autothink.UIA/logs/20260104-004044/unexpected_ui_state.json`：
```json
{
  "attempts": 3,
  "handled": 1,
  "entries": [
    {
      "flowName": "autothink.importVariables",
      "handlerName": "GenericOk",
      "stage": "Preflight",
      "selectorKeys": ["global.popupRoot", "global.popupOkButton"],
      "success": true
    }
  ]
}
```

## StepLog 片段（UnexpectedUIState）
来自 `Autothink.UIA/logs/20260104-004044/step_logs.json`：
```json
{
  "stepId": "UnexpectedUIState",
  "action": "Handle unexpected UI state",
  "parameters": {
    "flowName": "autothink.importVariables",
    "handlerName": "GenericOk",
    "matchedSelectorKeys": "global.popupRoot,global.popupOkButton"
  }
}
```

## selector pack 新增 global keys 片段
来自 `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.v1.base.json`：
```json
"global.popupRoot": {
  "Path": [
    { "Search": "Descendants", "ControlType": "Window", "AutomationIdContains": "Dialog", "Index": 0 }
  ]
},
"global.popupOkButton": {
  "Path": [
    { "Search": "Descendants", "ControlType": "Button", "AutomationId": "popupOkButton", "Index": 0 }
  ]
}
```

## Runbook 更新片段
```text
## 3.8 UIStateRecovery（UnexpectedUIState 收敛）
- 配置示例："uiStateRecovery": { "enable": true, "maxAttempts": 2, "searchRoot": "desktop" }
- selector 约定：global.popupRoot / global.popupOkButton / global.popupNoButton / global.popupWarningText
- 产物：unexpected_ui_state.json + summary.uiStateRecovery
```

## 自检清单
- [x] 未新增 RPC 方法；逻辑仅在 Runner/flow 调度层实现。
- [x] 命中处理写入 StepLog（stepId=UnexpectedUIState）。
- [x] `unexpected_ui_state.json` 落盘，summary 提供 uiStateRecovery 概览。
- [x] DemoTarget 弹窗可自动关闭并继续跑通流程。
