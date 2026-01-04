# Stage2.1 当前进度（AUTOTHINK 普通型）

本文汇总 Stage2.1（TASK-S2-09 ~ TASK-S2-14）的最新完成情况与可测试入口。

## 1. 总体结论

- Stage2.1 代码与文档已落地，具备现场回归与证据采集能力。
- 主要新增：selector profile、匹配增强、searchRoot、配置化回归脚本、Runbook。

## 2. 已完成清单

- **TASK-S2-09**：Stage2Runner 支持 `--selectorsRoot` / `--profile`，profile JSON 已落地。
- **TASK-S2-10**：Selector 增强（AutomationIdContains / ClassNameContains / NormalizeWhitespace）。
- **TASK-S2-11**：Flow 级 `searchRoot`（mainWindow/desktop），StepLog 记录 root。
- **TASK-S2-12**：Stage2Runner 配置化回归 + StepLog 输出到 `logs/<timestamp>/`。
- **TASK-S2-13**：Runbook 文档已输出。
- **TASK-S2-14**：selector 基线资产包 + local override + summary.json 输出已完成。

## 3. 关键落点

- Selector profile：`Docs/组态软件自动操作/Selectors/`
- 回归配置：`Docs/组态软件自动操作/RunnerConfig/demo.json`
- Runbook：`Docs/组态软件自动操作/Runbook-Autothink-普通型.md`
- Selector baseline/local：`Docs/组态软件自动操作/Selectors/`

## 4. 可测试入口

- WinFormsHarness：
  - `dotnet run --project Autothink.UiaAgent.WinFormsHarness/Autothink.UiaAgent.WinFormsHarness.csproj -c Release`
- Stage2Runner：
  - `dotnet build PLCCodeForge.sln -c Release`
  - `dotnet run --project Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --config Docs/组态软件自动操作/RunnerConfig/demo.json`

## 5. 待办

- 现场录制 selector 并更新 profile JSON。
- 在真实 AUTOTHINK 普通型环境跑通 4 条 flow，提交 `logs/<timestamp>/` 证据。
