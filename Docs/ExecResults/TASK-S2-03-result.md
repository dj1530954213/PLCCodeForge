# TASK-S2-03-result.md

- **Task 编号与标题**：
  - TASK-S2-03：Selector 增量扩展（NameContains / IgnoreCase）

- **完成摘要**：
  - 扩展 SelectorStep（仅 Name 维度）支持更稳的匹配方式：
    - `NameContains`：子串匹配
    - `IgnoreCase`：忽略大小写（仅作用于 Name/NameContains）
  - 保持默认行为不变：未设置新字段时仍为原先的精确匹配（Ordinal、区分大小写）。
  - 为匹配逻辑补充纯逻辑单元测试，确保 exact/contains/ignorecase 的语义稳定。

- **改动清单**：
  - `Autothink.UiaAgent/Rpc/Contracts/ElementSelector.cs`：
    - `SelectorStep` 新增 `NameContains`、`IgnoreCase` 字段。
  - `Autothink.UiaAgent/Uia/ElementFinder.cs`：
    - `hasAnyFilter` 逻辑纳入 `NameContains`。
    - Name 匹配改为统一走 `MatchesText(...)`，支持 exact/contains/ignorecase 且默认保持原行为。
  - `Autothink.UiaAgent.Tests/ElementFinderTextMatchTests.cs`：
    - 新增单元测试覆盖默认 exact（区分大小写）与 contains/ignorecase 行为。

- **关键实现说明**：
  - 新字段均为可选：
    - 旧 selector 不需要修改即可保持原先精确匹配。
    - 只有在 `IgnoreCase=true` 或 `NameContains` 有值时才启用增量能力。
  - `MatchesText(actual, expectedExact, expectedContains, ignoreCase)` 规则：
    - 如果设置了 `expectedExact`：先做 equals（默认 Ordinal）。
    - 如果设置了 `expectedContains`：再做 contains（默认 Ordinal）。
    - 两者同时存在时按 AND 语义叠加约束。

- **完成证据**：
  - `dotnet build`（Release）输出片段：
    ```text
    已成功生成。
        0 个警告
        0 个错误
    ```
  - `dotnet test`（Release）输出片段：
    ```text
    已通过! - 失败:     0，通过:    23，已跳过:     1，总计:    24
    ```

- **验收自检**：
  - [x] SelectorStep 新增 `NameContains` / `IgnoreCase` 字段。
  - [x] ElementFinder 支持 contains/ignorecase 且默认 exact 不变。
  - [x] 单测覆盖默认 exact 与 contains/ignorecase。

- **风险/未决项**：
  - 当前增量扩展仅作用于 Name（按计划要求）；如后续需要对 ClassName/AutomationId 也提供 ignorecase/contains，建议在不破坏默认行为的前提下继续按“新增可选字段”扩展。

