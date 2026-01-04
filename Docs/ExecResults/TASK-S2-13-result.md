# TASK-S2-13-result.md

- **Task 编号与标题**：
  - TASK-S2-13：真实 AUTOTHINK 普通型联调 Runbook

- **完成摘要**：
  - 已输出现场联调 Runbook，覆盖环境前置、selector 录制、Stage2Runner 运行、失败定位与证据提交。

- **改动清单**：
  - `Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 Runbook 文档。

- **Runbook 关键片段（摘录）**：
  ```text
  ## 2. Selector 录制与校验流程
  1. 运行 Agent（Release）...
  2. 运行 WinFormsHarness...
  3. FindElement 验证 selector...

  ## 4. 失败定位指南
  - FindError：单独验证 selector
  - TimeoutError：调大 waitTimeoutMs/verifyTimeoutMs
  - ActionError：检查焦点/剪贴板/输入法
  ```

- **证据提交要求**：
  - `logs/<timestamp>/` 目录整包提交。

- **build/test 证据**：
  - `dotnet build PLCCodeForge.sln -c Release`：
    ```text
    已成功生成。
        0 个警告
        0 个错误
    ```
  - `dotnet test Autothink.UiaAgent.Tests/Autothink.UiaAgent.Tests.csproj -c Release`：
    ```text
    已通过! - 失败:     0，通过:    34，已跳过:     1，总计:    35
    ```

- **验收自检**：
  - [x] Runbook 已落地到文档目录。
  - [x] 现场联调步骤与证据提交要求清晰。

- **风险/未决项**：
  - 需由现场执行人员按 Runbook 跑通真实 AUTOTHINK，并提交 logs 证据。
