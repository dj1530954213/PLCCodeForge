# TASK-S2-19 结果（最小真机闭环：probe -> local 校准 -> flow + popupHandling）

## 1. 完成摘要
- 通过 `Stage2Runner --check` 验证 FullHost 连接与方法探测 OK，connectivity.json 记录完整。
- 使用 DemoTarget 执行 `probe` 校准 build 相关 selector（buildButton/buildStatus）。
- 通过 `.local.json` 校准弹窗 selector，触发并关闭 build 弹窗，StepLog 出现 PopupDetected/PopupDismissed。
- 按配置跳过 importVariables/importProgram，仅运行 attach + build，保证 build flow 可稳定闭环。

## 2. 改动清单（本任务相关）
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`
  - 支持按 config 跳过 importVariables/importProgram/build，并在 summary 中标注 NotRun。
- `Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`
  - 增加 `skipImportVariables=true`、`skipImportProgram=true`，只跑 build。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.popups.local.json`
  - 使用 AutomationId 绑定 DemoTarget 弹窗（buildConfirmDialog / popupOkButton / popupCancelButton）。

## 3. 关键证据

### 3.1 build Release
```text
Autothink.UiaAgent.DemoTarget -> C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\Autothink.UiaAgent.DemoTarget\bin\Release\net8.0-windows\Autothink.UiaAgent.DemoTarget.dll
Autothink.UiaAgent -> C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\Autothink.UiaAgent\bin\Release\net8.0-windows\Autothink.UiaAgent.dll
Autothink.UiaAgent.Stage2Runner -> C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\Autothink.UiaAgent.Stage2Runner\bin\Release\net8.0-windows\Autothink.UiaAgent.Stage2Runner.dll
已成功生成。
```

### 3.2 --check OK（connectivity.json）
- 日志路径：`Autothink.UIA/logs/20260103-191722/connectivity.json`
```json
{
  "ok": true,
  "handshakeReady": true,
  "pingOk": true,
  "methods": {
    "OpenSession": true,
    "CloseSession": true,
    "FindElement": true,
    "Click": true,
    "SetText": true,
    "SendKeys": true,
    "WaitUntil": true
  },
  "durationMs": 195
}
```

### 3.3 probe 输出（buildButton/buildStatus）
- 日志路径：`Autothink.UIA/logs/20260103-192141/probe.autothink.build.json`
```json
{
  "flowName": "autothink.build",
  "entries": [
    {
      "selectorKey": "buildButton",
      "ok": true,
      "matchedCount": 1,
      "element": {
        "name": "buildButton",
        "automationId": "buildButton",
        "controlType": "Button"
      }
    },
    {
      "selectorKey": "buildStatus",
      "ok": true,
      "matchedCount": 1,
      "element": {
        "name": "statusLabel",
        "automationId": "statusLabel",
        "controlType": "Text"
      }
    }
  ]
}
```

### 3.4 .local.json 校准片段（弹窗）
- 文件：`Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.popups.local.json`
```json
{
  "selectors": {
    "popupDialog": {
      "Path": [
        { "Search": "Descendants", "ControlType": "Window", "AutomationId": "buildConfirmDialog", "Index": 0 }
      ]
    },
    "popupOkButton": {
      "Path": [
        { "Search": "Descendants", "ControlType": "Button", "AutomationId": "popupOkButton", "Index": 0 }
      ]
    },
    "popupCancelButton": {
      "Path": [
        { "Search": "Descendants", "ControlType": "Button", "AutomationId": "popupCancelButton", "Index": 0 }
      ]
    }
  }
}
```

### 3.5 flow summary（build 成功 + popupHandledCount=1）
- 日志路径：`Autothink.UIA/logs/20260103-192153/summary.json`
```json
{
  "flows": [
    { "name": "autothink.attach", "ok": true },
    { "name": "autothink.importVariables", "ok": false, "errorKind": "NotRun", "errorMessage": "Skipped by config" },
    { "name": "autothink.importProgram.textPaste", "ok": false, "errorKind": "NotRun", "errorMessage": "Skipped by config" },
    { "name": "autothink.build", "ok": true, "popupHandledCount": 1, "lastPopupTitle": "buildConfirmDialog" }
  ],
  "connectivityOk": true
}
```

### 3.6 popupHandling StepLog（PopupDetected + PopupDismissed）
- 日志路径：`Autothink.UIA/logs/20260103-192153/autothink.build.json`
```json
{
  "stepId": "PopupDetected.AfterClickBuild",
  "parameters": {
    "root": "desktop",
    "found": "true",
    "title": "buildConfirmDialog"
  }
}
```
```json
{
  "stepId": "PopupDismissed.AfterClickBuild",
  "parameters": {
    "root": "desktop",
    "button": "cancel",
    "title": "buildConfirmDialog"
  }
}
```

## 4. 复现步骤（最小闭环）
1) 编译：`dotnet build Autothink.UIA/PLCCodeForge.sln -c Release`
2) 启动 DemoTarget：
   - `Autothink.UIA\Autothink.UiaAgent.DemoTarget\bin\Release\net8.0-windows\Autothink.UiaAgent.DemoTarget.exe`
3) 连接检查：
   - `dotnet run --project Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --check --config Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`
4) probe 校准：
   - `dotnet run --project Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --config Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json --probe --probeFlow autothink.build --probeKeys buildButton,buildStatus --probeSearchRoot mainWindow --probeTimeoutMs 5000`
5) 运行 flow（仅 attach + build，含弹窗处理）：
   - `dotnet run --project Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --config Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`

## 5. 风险与未决项
- 真实 AUTOTHINK 环境仍需现场录制 selector（`.local.json`），本次 DemoTarget 仅用于闭环验证。
- importProgram.textPaste 仍受剪贴板环境影响，已通过 skip 在本次闭环中规避。
