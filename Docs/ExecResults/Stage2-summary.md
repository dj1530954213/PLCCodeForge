# Stage2-summary.md

- 范围：UIA 自动操作 Stage 2（仅 AUTOTHINK 普通型）
- 交付：RunFlow RPC + 4 条 flow（attach / importVariables / importProgram.textPaste / build）
- 回归入口：
  - WinFormsHarness（交互式）
  - Stage2Runner（自动跑 DemoTarget + Agent）
- Selector 统一存放：`Docs/组态软件自动操作/Selectors/`
- 结果文档：`Docs/ExecResults/TASK-S2-01-result.md` ~ `TASK-S2-08-result.md`

## 快速回归

```powershell
# 1) Release build
 dotnet build PLCCodeForge.sln -c Release

# 2) 一键回归（DemoTarget + Agent）
 dotnet run --project Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release
```

## 说明
- DemoTarget 用于模拟编辑器/导入/编译控件，便于脱离真实 AUTOTHINK 做回归。
- 真机验证时，仅替换 selector JSON，无需修改 flow 代码。
