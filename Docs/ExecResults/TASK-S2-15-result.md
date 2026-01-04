# TASK-S2-15-result.md

- **Task 编号与标题**：
  - TASK-S2-15：SelectorProbe 探针模式 + 现场校准闭环

- **完成摘要**：
  - Stage2Runner 新增 `--probe` 模式，可按 flow + selectorKey 逐个校验并落盘 `probe.<flow>.json`。
  - 探针输出包含 selectorKey、root、matchedCount/usedIndex、元素快照与可执行建议。
  - Runbook 增补了 SelectorProbe 使用流程与 `.local.json` 校准闭环。

- **改动清单**：
  - `Autothink.UiaAgent.Stage2Runner/Program.cs`：新增 probe 模式、探针结果落盘、Selector 解析与建议规则。
  - `Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 SelectorProbe 章节与证据要求。

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

- **Stage2Runner probe 运行日志片段**：
  ```text
  RunDir: C:\Program Files\Git\code\PLCCodeForge\logs\20260103-140341
  ProbeFlow: autothink.build
  ProbeKeys: buildButton,buildStatus
  ProbeRoot: desktop
  ProbeTimeoutMs: 2000
  Probe written: C:\Program Files\Git\code\PLCCodeForge\logs\20260103-140341\probe.autothink.build.json
  ```

- **probe.<flow>.json 示例（2 个 key）**：
  - `logs/20260103-140341/probe.autothink.build.json`
    ```json
    {
      "flowName": "autothink.build",
      "root": "desktop",
      "entries": [
        {
          "selectorKey": "buildButton",
          "ok": false,
          "errorKind": "RpcError",
          "usedIndex": "step0:0"
        },
        {
          "selectorKey": "buildStatus",
          "ok": false,
          "errorKind": "RpcError",
          "usedIndex": "step0:0"
        }
      ]
    }
    ```

- **Runbook 新增章节片段**：
  ```text
  ## 2.1 SelectorProbe 探针校准（推荐）
  - --probe --probeFlow autothink.build --probeKeys buildButton,buildStatus
  - 产物：logs/<timestamp>/probe.<flow>.json
  - Ambiguous/0 match 依据 probe 输出修正 .local.json
  ```

- **验收自检**：
  - [x] 不修改 RPC 契约，仅 Stage2Runner 侧新增 probe。
  - [x] probe 输出落盘到 `logs/<timestamp>/probe.<flow>.json`。
  - [x] 每条记录包含 selectorKey/root/匹配信息/建议字段。
  - [x] Runbook 已补充 SelectorProbe 流程与证据要求。

- **风险/未决项**：
  - 本机环境 OpenSession 未注册导致 probe 返回 `RpcError`；现场需使用完整 Agent 进行真实元素校验。
