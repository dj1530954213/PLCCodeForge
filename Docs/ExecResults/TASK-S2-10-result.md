# TASK-S2-10-result.md

- **Task 编号与标题**：
  - TASK-S2-10：SelectorStep 增强字段（AutomationIdContains / ClassNameContains / NormalizeWhitespace）

- **完成摘要**：
  - SelectorStep 新增 `AutomationIdContains` / `ClassNameContains` / `NormalizeWhitespace`。
  - ElementFinder 支持 contains + 空白归一化，exact 优先级保持不变。
  - StepLog 在 exact+contains 同时存在时记录 matchRule。

- **改动清单**：
  - `Autothink.UiaAgent/Rpc/Contracts/ElementSelector.cs`：新增字段定义。
  - `Autothink.UiaAgent/Uia/ElementFinder.cs`：匹配逻辑与归一化。
  - `Autothink.UiaAgent/Flows/FlowContext.cs`：matchRule 参数注入。
  - `Autothink.UiaAgent/Rpc/UiaRpcService.cs`：matchRule 参数注入。
  - `Autothink.UiaAgent.Tests/ElementFinderTextMatchTests.cs`：新增测试覆盖。

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

- **新增测试（摘要）**：
  - exact 优先于 contains 的规则保持一致。
  - NormalizeWhitespace 对匹配文本生效。
  - DescribeMatchRules 可标注 AutomationId/ClassName exact 优先级。

- **JSON-RPC selector 示例（含新增字段）**：
  ```json
  {
    "Path": [
      {
        "Search": "Descendants",
        "ControlType": "Button",
        "AutomationIdContains": "BTN_",
        "ClassNameContains": "Button",
        "NameContains": "编译",
        "IgnoreCase": true,
        "NormalizeWhitespace": true,
        "Index": 0
      }
    ]
  }
  ```

- **验收自检**：
  - [x] 新字段已可选使用，旧 exact 行为不变。
  - [x] ElementFinder 支持 contains + NormalizeWhitespace。
  - [x] build/test Release 通过。

- **风险/未决项**：
  - 现场 selector 仍需按 UI 实际值校准（尤其 NameContains/AutomationIdContains）。
