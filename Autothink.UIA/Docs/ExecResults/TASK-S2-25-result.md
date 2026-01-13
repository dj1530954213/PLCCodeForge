# TASK-S2-25-result.md

## 完成摘要
- Stage2Runner 增加 FlowInputs v1（inputsSource=inline/fromCommIr），可解析 CommIR v1 并落盘 `resolved_inputs.json`。
- summary.json 新增 inputsSource 字段，记录解析模式与 resolved_inputs 路径。
- 提供 CommIR 样例与 demo 配置，并在 Runbook 中写明 fromCommIr 使用方式。

## 改动清单
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/InputBinding/FlowInputs.cs`：inputsSource 配置与解析结果模型。
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/InputBinding/CommIrReader.cs`：CommIR v1 解析逻辑。
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`：inputsSource 解析、resolved_inputs.json 落盘、summary 记录。
- `Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`：inputsSource=fromCommIr 示例。
- `Autothink.UIA/Docs/Samples/comm_ir.sample.json`：CommIR 样例。
- `Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 inputsSource 使用说明。

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

## resolved_inputs.json 片段
来自 `Autothink.UIA/logs/20260103-220800/resolved_inputs.json`：
```json
{
  "ok": true,
  "mode": "fromCommIr",
  "commIrPath": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\Docs\\Samples\\comm_ir.sample.json",
  "variablesFilePath": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\Docs\\Samples\\variables_demo.xlsx",
  "programTextPath": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\Docs\\Samples\\program_demo.st",
  "outputDir": "C:\\temp\\plc-codeforge-output",
  "variablesSource": "inputs.variablesFilePath",
  "programSource": "inputs.programTextPath"
}
```

## summary.json 片段（inputsSource）
来自 `Autothink.UIA/logs/20260103-220800/summary.json`：
```json
{
  "inputsSource": {
    "mode": "fromCommIr",
    "commIrPath": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\Docs\\Samples\\comm_ir.sample.json",
    "resolvedInputsPath": "C:\\Program Files\\Git\\code\\PLCCodeForge\\Autothink.UIA\\Autothink.UIA\logs\\20260103-220800\\resolved_inputs.json",
    "warnings": null
  }
}
```

## Runbook 更新片段
```text
## 3.4 inputsSource（fromCommIr）
- 适用场景：从通讯采集模块输出的 CommIR v1 直接绑定变量表与程序文本路径。
- 配置方式（示例）：
  - inputsSource.mode: "fromCommIr"
  - inputsSource.commIrPath: "..\\..\\Samples\\comm_ir.sample.json"
- 解析产物：
  - Autothink.UIA/logs/<timestamp>/resolved_inputs.json
```

## 自检清单
- [x] 未修改 RPC 契约，仅在 Runner 侧新增 inputsSource 解析。
- [x] CommIR 解析结果落盘到 `resolved_inputs.json`。
- [x] summary.json 包含 inputsSource 字段（mode/commIrPath/resolvedInputsPath）。
- [x] DemoTarget 使用 fromCommIr 解析路径可复现。
