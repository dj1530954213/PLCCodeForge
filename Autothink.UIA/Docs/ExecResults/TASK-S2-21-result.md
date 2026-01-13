# TASK-S2-21 结果（importVariables DemoTarget 贯通）

## 1. 完成摘要
- 引入 `autothink.base.json` 作为 DemoTarget 基线 selector 资产，真实 AUTOTHINK 通过 `.local.json` 覆盖。
- importVariables 流程按 selectorKey 运行，DemoTarget 实测 OK。
- summary 错误信息追加 selectorKey（失败可定位）。

## 2. 改动清单
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`
  - 支持加载 `autothink.base.json`（含 base local override），并与 flow profile 合并。
  - summary errorMessage 自动追加 `selectorKey=...`。
- `Autothink.UIA/Autothink.UiaAgent.DemoTarget/MainForm.cs`
  - 变量导入改为独立对话框（Window），便于 WaitUntil(ElementNotExists) 验证。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.base.json`
  - 新增 importVariables 关键 key（DemoTarget AutomationId）。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.importVariables.local.json`
  - 更新为新 key 命名并匹配 DemoTarget。
- `Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`
  - 更新 importVariables selectorKey 与样例变量表路径。
- `Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`
  - 增加 importVariables 关键 selector 校准说明。
- `Autothink.UIA/Docs/Samples/variables_demo.xlsx`
  - Demo 变量表占位文件。

## 3. 构建/测试证据

### 3.1 build (Release)
```text
Autothink.UiaAgent -> C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\Autothink.UiaAgent\bin\Release\net8.0-windows\Autothink.UiaAgent.dll
Autothink.UiaAgent.DemoTarget -> C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\Autothink.UiaAgent.DemoTarget\bin\Release\net8.0-windows\Autothink.UiaAgent.DemoTarget.dll
Autothink.UiaAgent.Stage2Runner -> C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\Autothink.UiaAgent.Stage2Runner\bin\Release\net8.0-windows\Autothink.UiaAgent.Stage2Runner.dll
已成功生成。
```

### 3.2 test (Release)
```text
已通过! - 失败:     0，通过:    34，已跳过:     1，总计:    35
```

## 4. DemoTarget 运行证据

### 4.1 summary.json（importVariables OK）
- 日志：`Autothink.UIA/logs/20260103-200145/summary.json`
```json
{
  "flows": [
    { "name": "autothink.attach", "ok": true },
    { "name": "autothink.importVariables", "ok": true, "root": "desktop" },
    { "name": "autothink.importProgram.textPaste", "ok": false, "errorKind": "NotRun", "errorMessage": "Skipped by config" },
    { "name": "autothink.build", "ok": false, "errorKind": "NotRun", "errorMessage": "Skipped by config" }
  ],
  "connectivityOk": true
}
```

### 4.2 StepLog 关键片段（Click/SetText/WaitUntil）
- 日志：`Autothink.UIA/logs/20260103-200145/autothink.importVariables.json`
```json
{ "stepId": "OpenImportDialog", "action": "Open import dialog step", "outcome": "Success" }
```
```json
{ "stepId": "SetFilePath", "action": "Set file path", "outcome": "Success" }
```
```json
{ "stepId": "WaitImportDone", "action": "Wait import done", "outcome": "Success" }
```

## 5. Selector 资产片段（base + local）

### 5.1 base（DemoTarget 基线）
- 文件：`Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.base.json`
```json
{
  "selectors": {
    "importVariablesMenuOrButton": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "AutomationId": "openImportButton", "Index": 0 } ] },
    "importDialog": { "Path": [ { "Search": "Descendants", "ControlType": "Window", "AutomationId": "importDialog", "Index": 0 } ] },
    "filePathEdit": { "Path": [ { "Search": "Descendants", "ControlType": "Edit", "AutomationId": "filePathEdit", "Index": 0 } ] },
    "importOkButton": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "AutomationId": "importOkButton", "Index": 0 } ] }
  }
}
```

### 5.2 local（现场覆盖示例）
- 文件：`Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.importVariables.local.json`
```json
{
  "selectors": {
    "importVariablesMenuOrButton": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "AutomationId": "openImportButton", "Index": 0 } ] },
    "importDialog": { "Path": [ { "Search": "Descendants", "ControlType": "Window", "AutomationId": "importDialog", "Index": 0 } ] },
    "filePathEdit": { "Path": [ { "Search": "Descendants", "ControlType": "Edit", "AutomationId": "filePathEdit", "Index": 0 } ] },
    "importOkButton": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "AutomationId": "importOkButton", "Index": 0 } ] }
  }
}
```

## 6. Runbook 更新片段
- 文件：`Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`
```text
## 2.2 importVariables 关键 selector 校准（base + local）
- 关键 selector keys（统一放在 autothink.base.json）：
  - importVariablesMenuOrButton
  - importDialog
  - filePathEdit
  - importOkButton
  - importDoneIndicator
```

## 7. 自检清单
- [x] 未新增 RPC 方法，仅使用 Find/Click/SetText/WaitUntil 组合。
- [x] selector 通过 selectorKey 注入，未在代码里写死 Name。
- [x] summary 可输出 errorKind；若失败可通过 selectorKey 定位。
- [x] DemoTarget importVariables 运行 OK；非必跑 flow 仅在 config 显式 skip 时显示 NotRun。

## 8. 风险与未决项
- 真实 AUTOTHINK 仍需录制 `autothink.base.local.json`（替换 DemoTarget AutomationId）。
- 若 importDialog 在 AUTOTHINK 中不作为 Window 存在，需要调整 searchRoot/selector 组合。
