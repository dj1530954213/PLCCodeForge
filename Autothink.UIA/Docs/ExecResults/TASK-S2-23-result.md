# TASK-S2-23-result.md

## 完成摘要
- Stage2Runner 默认 fail-fast：flow 失败即停止后续执行，并在 `summary.json` 记录 `stoppedBecause`。
- DemoTarget 全链路（attach/importVariables/importProgram/build）已默认启用并跑通。
- Runbook 补充 fail-fast 与 skip/allowPartial 说明。

## 改动清单
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`：新增 `stoppedBecause`、fail-fast 逻辑、`allowPartial` 配置支持；缺失 selectorKey 时输出结构化 summary 而非崩溃。
- `Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`：默认不再 skip build，形成完整流水线。
- `Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 fail-fast/skip/allowPartial 说明。

## Build/Test 证据
```text
dotnet build Autothink.UIA/PLCCodeForge.sln -c Release
Autothink.UiaAgent.Stage2Runner -> ...\Autothink.UiaAgent.Stage2Runner.dll
已成功生成。

dotnet test Autothink.UIA/PLCCodeForge.sln -c Release
已通过! - 失败: 0，通过: 34，已跳过: 1，总计: 35
```

## DemoTarget 全链路 OK（summary.json 片段）
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
日志目录：`C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\logs\20260103-211408`

## Fail-fast 证据（故意缺 selectorKey）
```json
{
  "flows": [
    { "name": "autothink.importVariables", "ok": false, "errorKind": "InvalidArgument", "errorMessage": "Selector key not found: missingSelectorKey" },
    { "name": "autothink.importProgram.textPaste", "ok": false, "errorKind": "NotRun" },
    { "name": "autothink.build", "ok": false, "errorKind": "NotRun" }
  ],
  "stoppedBecause": "flowFailed:autothink.importVariables"
}
```
日志目录：`C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\logs\20260103-211925`

## StepLog 关键片段（每 flow 至少 1~2 步）
```text
autothink.attach: GetMainWindow, BringToForeground
autothink.importVariables: OpenImportDialog, SetFilePath, ConfirmImport, WaitImportDone
autothink.importProgram.textPaste: FindEditor, SetClipboardText, SendKeysPaste, VerifyPaste
autothink.build: ClickBuild, PopupDismissed.AfterClickBuild, WaitBuildDone
```

## Runbook 更新片段
```text
3.2 流水线策略（fail-fast / skip / allowPartial）
- 默认 fail-fast；如需跳过 flow 可用 skipImportVariables/skipImportProgram/skipBuild
- 如需失败后继续，设置 allowPartial: true
```

## 自检清单
- [x] 默认流水线包含 4 个 flow，DemoTarget 可一键跑通。
- [x] fail-fast 生效，summary 中记录 stoppedBecause。
- [x] 缺失 selectorKey 时不崩溃，summary 给出 InvalidArgument。
- [x] runbook 已说明 skip/allowPartial 与 popupHandling 开关。
