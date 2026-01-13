# TASK-S2-26-result.md

## 完成摘要
- 冻结 AUTOTHINK v1 selector pack：新增 `autothink.v1.base.json` 与 `autothink.v1.local.sample.json`，并在 RunnerConfig 指定 `selectorPackVersion: "v1"`。
- Stage2Runner 增加 selector-check 门禁并输出 `selector_check_report.json`，缺 key 直接 ConfigError + fail-fast。
- Runbook 明确 v1 pack 维护与 selector-check 校验流程。

## 改动清单
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.v1.base.json`：v1 base 冻结 key 集合（DemoTarget 用 AutomationId）。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.v1.local.sample.json`：真实 AUTOTHINK 模板（local 覆盖示例）。
- `Autothink.UIA/Docs/组态软件自动操作/Selectors/README.md`：v1 pack 命名与覆盖约定。
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`：selector-check 门禁与 `selector_check_report.json` 落盘。
- `Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`：启用 `selectorPackVersion: "v1"`。
- `Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：v1 pack 维护/校准说明。

## Build/Test 证据
```text
dotnet build Autothink.UIA/PLCCodeForge.sln -c Release
Autothink.UiaAgent.DemoTarget -> ...\Autothink.UiaAgent.DemoTarget.dll
Autothink.UiaAgent -> ...\Autothink.UiaAgent.dll
Autothink.UiaAgent.Stage2Runner -> ...\Autothink.UiaAgent.Stage2Runner.dll
Autothink.UiaAgent.WinFormsHarness -> ...\Autothink.UiaAgent.WinFormsHarness.dll
Autothink.UiaAgent.Tests -> ...\Autothink.UiaAgent.Tests.dll
已成功生成。

dotnet test Autothink.UIA/PLCCodeForge.sln -c Release
已通过! - 失败: 0，通过: 34，已跳过: 1，总计: 35
```

## DemoTarget 全链路 summary（全部 ok）
来自 `Autothink.UIA/logs/20260103-233612/summary.json`：
```json
{
  "flows": [
    { "name": "autothink.attach", "ok": true },
    { "name": "autothink.importVariables", "ok": true },
    { "name": "autothink.importProgram.textPaste", "ok": true },
    { "name": "autothink.build", "ok": true }
  ],
  "stoppedBecause": null
}
```

## selector_check_report.json 片段（missingKeys 为空）
来自 `Autothink.UIA/logs/20260103-233612/selector_check_report.json`：
```json
{
  "packVersion": "v1",
  "missingKeys": [],
  "requiredKeys": [
    "autothink.attach:mainWindow",
    "autothink.importVariables:importVariablesMenuOrButton",
    "autothink.importVariables:importVariablesDialogRoot",
    "autothink.importVariables:importVariablesFilePathEdit",
    "autothink.importVariables:importVariablesOkButton",
    "autothink.importVariables:importVariablesDoneIndicator",
    "autothink.importProgram.textPaste:importProgramMenuOrButton",
    "autothink.importProgram.textPaste:programEditorRoot",
    "autothink.importProgram.textPaste:programEditorTextArea",
    "autothink.importProgram.textPaste:programPastedIndicator",
    "autothink.build:buildButton",
    "autothink.build:(anyOf:buildOutputPane|buildStatus)",
    "autothink.build:(anyOf:buildSucceededIndicator|buildFinishedIndicator)"
  ]
}
```

## 缺 key 失败证据（门禁生效）
来自 `Autothink.UIA/logs/20260103-233808/selector_check_report.json`：
```json
{
  "missingKeys": [
    "autothink.build:(anyOf:buildSucceededIndicator|buildFinishedIndicator)"
  ]
}
```
来自 `Autothink.UIA/logs/20260103-233808/summary.json`：
```json
{
  "flows": [
    {
      "name": "autothink.attach",
      "ok": false,
      "errorKind": "ConfigError",
      "errorMessage": "selector-check missingKeys: [autothink.build:(anyOf:buildSucceededIndicator|buildFinishedIndicator)]"
    }
  ],
  "stoppedBecause": "selectorCheckFailed"
}
```

## Runbook 更新片段
```text
- v1 pack 基线：`autothink.v1.base.json`（冻结的必填 key 集合）
- v1 local 覆盖：`autothink.v1.local.json`（复制 `autothink.v1.local.sample.json` 后改）
- RunnerConfig 需设置：`selectorPackVersion: "v1"`
...
- `Autothink.UIA/logs/<timestamp>/selector_check_report.json`（必填 key 校验）
```

## 自检清单
- [x] v1 selector pack 作为冻结真源落地（base + local sample）。
- [x] selector-check 门禁生效，缺 key 直接 ConfigError + fail-fast。
- [x] `selector_check_report.json` 落盘并包含 requiredKeys/missingKeys。
- [x] DemoTarget 全链路可复现（summary ok=true）。
