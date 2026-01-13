# TASK-S2-22-result.md

## 完成摘要
- 实现 `autothink.importProgram.textPaste`：剪贴板优先 + CTRL+V；剪贴板不可用时记录 Warning 并自动 fallback 到键盘输入。
- DemoTarget 成功跑通 `importProgram.textPaste`（含 StepLog 证据链）。
- Selector 资产与 Runbook 已补充 importProgram 校准说明与模板。

## 改动清单
- `Autothink.UIA/Autothink.UiaAgent/Flows/Autothink/AutothinkImportProgramTextPasteFlow.cs`：新增 clipboard 失败时的 `SendKeysPaste` Warning 记录，保证证据链完整；保留 fallback 行为。
- `Autothink.UIA/Autothink.UiaAgent.DemoTarget/MainForm.cs`：修复变量导入对话框非模态展示的生命周期，确保 DemoTarget 可稳定联调。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.base.json`：补齐 DemoTarget 对应的 importProgram 关键 selector key。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.importProgram.local.sample.json`：真实 AUTOTHINK 模板。
- `Autothink.UIA/Docs/Samples/program_demo.st`：DemoTarget 用 ST 文本样例。
- `Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 importProgram 校准说明。

## Build/Test 证据
```text
dotnet build Autothink.UIA/PLCCodeForge.sln -c Release
Autothink.UiaAgent.DemoTarget -> ...\Autothink.UiaAgent.DemoTarget.dll
Autothink.UiaAgent -> ...\Autothink.UiaAgent.dll
Autothink.UiaAgent.Stage2Runner -> ...\Autothink.UiaAgent.Stage2Runner.dll
已成功生成。

dotnet test Autothink.UIA/PLCCodeForge.sln -c Release
已通过! - 失败: 0，通过: 34，已跳过: 1，总计: 35
```

## DemoTarget 运行证据（summary.json 片段）
```json
{
  "name": "autothink.importProgram.textPaste",
  "ok": true,
  "root": "mainWindow",
  "logFile": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\logs\\20260103-211408\\autothink.importProgram.textPaste.json"
}
```

## StepLog 关键片段（剪贴板 + CTRL+V + fallback）
```json
[
  { "stepId": "SetClipboardText", "outcome": "Warning", "error": { "message": "SetClipboardText failed" } },
  { "stepId": "SendKeysPaste", "outcome": "Warning", "error": { "message": "Clipboard unavailable; skipped CTRL+V" } },
  { "stepId": "FallbackTypeText", "outcome": "Success" },
  { "stepId": "VerifyPaste", "outcome": "Success", "parameters": { "mode": "editorNotEmpty" } }
]
```
说明：本机剪贴板写入失败时会触发 fallback，且 StepLog 仍保留 `SendKeysPaste` 证据；若现场剪贴板可用，则为 Success 并走 CTRL+V 主路径。

## Selector 资产片段
`Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.base.json`：
```json
{
  "importProgramMenuOrButton": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "AutomationId": "importProgramButton", "Index": 0 } ] },
  "programEditorTextArea": { "Path": [ { "Search": "Descendants", "AutomationId": "programEditor", "Index": 0 } ] }
}
```

`Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.importProgram.local.sample.json`（真实 AUTOTHINK 模板）：
```json
{
  "programEditorTextArea": { "Path": [ { "Search": "Descendants", "ControlType": "Document", "NameContains": "ST", "IgnoreCase": true, "Index": 0 } ] }
}
```

## Runbook 更新片段
```text
2.3 importProgram.textPaste 关键 selector 校准（base + local）
- importProgramMenuOrButton / programEditorRoot / programEditorTextArea ...
- 真实 AUTOTHINK 模板：Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.importProgram.local.sample.json
```

## 自检清单
- [x] clipboard 主路径 + CTRL+V 已实现，且 fallback 可记录 StepLog。
- [x] DemoTarget 跑通 importProgram.textPaste（见 summary + StepLog）。
- [x] selector 资产与 runbook 校准说明已更新。
- [x] 失败时 errorKind/StepLog 可定位到问题点。
