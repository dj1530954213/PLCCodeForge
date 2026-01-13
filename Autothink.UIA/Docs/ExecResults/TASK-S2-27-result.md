# TASK-S2-27-result.md

## 完成摘要
- buildOutcome 判定工程化：支持 waitSelector/readTextContains/either，输出 `build_outcome.json` 并写入 summary.build。
- DemoTarget buildOutcome 通过可观测指示控件（Build OK）稳定签收。
- 提供 Unknown 场景（readTextContains + 不匹配 token）用于超时诊断。

## 改动清单
- `Autothink.UIA/Autothink.UiaAgent/Flows/Autothink/AutothinkBuildFlow.cs`：BuildOutcome 判定 + StepLog 证据。
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`：summary.build 字段与 `build_outcome.json` 落盘。
- `Autothink.UIA/Autothink.UiaAgent.DemoTarget/MainForm.cs`：statusLabel 同步 AccessibleName，确保 UIA 可读状态文本。
- `Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`：buildOutcome 使用 `buildSucceededIndicator`（waitSelector）。
- `Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：buildOutcome 模板与签收说明。

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

## DemoTarget buildOutcome=Success 证据
来自 `Autothink.UIA/logs/20260103-233612/summary.json`：
```json
{
  "build": {
    "outcome": "Success",
    "evidencePath": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\logs\\20260103-233612\\build_outcome.json"
  }
}
```
来自 `Autothink.UIA/logs/20260103-233612/build_outcome.json`：
```json
{
  "outcome": "Success",
  "usedMode": "waitSelector",
  "selectorEvidence": { "successHit": true }
}
```

## Unknown 场景演示（readTextContains + 不匹配 token）
说明：selector-check 门禁会阻止“缺 key”直接进入 flow，因此 Unknown 通过“不匹配文本 token”演示。
来自 `Autothink.UIA/logs/20260103-234039/summary.json`：
```json
{
  "flows": [
    {
      "name": "autothink.build",
      "ok": false,
      "errorKind": "TimeoutError",
      "failedStepId": "BuildOutcome"
    }
  ],
  "build": {
    "outcome": "Unknown",
    "evidencePath": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\logs\\20260103-234039\\build_outcome.json"
  }
}
```
来自 `Autothink.UIA/logs/20260103-234039/build_outcome.json`：
```json
{
  "outcome": "Unknown",
  "usedMode": "readTextContains",
  "textEvidence": {
    "probed": true,
    "lastTextSample": "Build OK",
    "matchedToken": null
  },
  "errorKind": "TimeoutError"
}
```

## Runbook 更新片段
```text
## 3.4 buildOutcome（编译签收模板）
- 模板 A：waitSelector（只看成功指示控件）
  "buildOutcome": {
    "mode": "waitSelector",
    "successSelectorKey": "buildSucceededIndicator",
    "timeoutMs": 60000
  }
- 模板 B：readTextContains（读取输出/状态文本）
  "buildOutcome": {
    "mode": "readTextContains",
    "textProbeSelectorKey": "buildOutputPane",
    "successTextContains": ["编译成功", "Build OK", "0 errors"],
    "timeoutMs": 60000
  }
```

## 自检清单
- [x] 未新增 RPC 方法，仅在 Build flow 与 Runner summary 追加字段。
- [x] `build_outcome.json` 成功落盘，并写入 summary.build.evidencePath。
- [x] Success 与 Unknown 场景均可复现，TimeoutError 可定位到 BuildOutcome。
