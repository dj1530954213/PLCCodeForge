# TASK-S2-01-result.md

- **Task 编号与标题**：
  - TASK-S2-01：补齐错误码 NotImplemented + Flow Registry 骨架

- **完成摘要**：
  - 新增 FlowNames 常量集中管理 4 个固定 FlowName，并提供已知集合判断。
  - 引入 FlowRegistry 骨架，预注册 4 个 StubFlow（IsImplemented=false），统一供 FlowDispatcher 分发。
  - RunFlow 在 ResolveSession 之前先按 flowName 分发：未知 → InvalidArgument；已知但未实现 → NotImplemented（不依赖 session 是否存在）。
  - FlowDispatcher 使用 FlowRegistry/FlowNames 统一判断：未知 FlowName → InvalidArgument；已知但未实现 → NotImplemented（作为实现 flow 的统一入口）。
  - 已确认 `RpcErrorKinds.NotImplemented` 已存在于 `Autothink.UiaAgent/Rpc/Contracts/RpcError.cs`，无需修改。

- **改动清单**：
  - `Autothink.UiaAgent/Flows/FlowNames.cs`：新增 FlowName 常量与已知集合判断。
  - `Autothink.UiaAgent/Flows/FlowRegistry.cs`：新增注册表骨架与 StubFlow 占位实现。
  - `Autothink.UiaAgent/Flows/FlowDispatcher.cs`：改为从 FlowRegistry 分发，未知/未实现分支按规范返回错误。
  - `Autothink.UiaAgent/Rpc/UiaRpcService.cs`：RunFlow 先按 flowName 返回 InvalidArgument/NotImplemented，再进入 session 解析与 flow 执行。
  - `Autothink.UiaAgent.Tests/RunFlowDispatchTests.cs`：新增单测覆盖“未知/未实现 flow 不依赖 session”语义。
  - `Autothink.UiaAgent/Rpc/Contracts/RpcError.cs`：已包含 `NotImplemented`（仅核对，无代码变更）。

- **关键实现说明**：
  - 通过 `FlowNames` 固定 4 个 FlowName（大小写敏感），并提供 `IsKnown()` 作为判定入口，保证“已知但未实现”与“未知”语义分离。
  - `FlowRegistry` 默认预注册 4 个 StubFlow（`IsImplemented=false`），后续真实 flow 仅需 `FlowRegistry.Register(flow)` 替换即可。
  - `UiaRpcService.RunFlow` 在 ResolveSession 之前做 flowName 分发，使“未知/未实现”错误不依赖目标进程是否存在（便于早期联调与快速验收）。
  - `FlowDispatcher` 的行为：
    - 注册且 `IsImplemented=false` → `NotImplemented`
    - 未注册但 `IsKnown=true` → `NotImplemented`
    - `IsKnown=false` → `InvalidArgument`

- **完成证据**：
  - `dotnet build` 输出片段：
    ```text
    已成功生成。
        0 个警告
        0 个错误
    ```
  - `dotnet test` 输出片段：
    ```text
    已通过! - 失败:     0，通过:    23，已跳过:     1，总计:    24
    ```
  - JSON-RPC 示例（不存在 flowName → InvalidArgument；不依赖 session 存在）：
    ```json
    {
      "jsonrpc": "2.0",
      "id": 1,
      "method": "RunFlow",
      "params": {
        "SessionId": "session-1",
        "FlowName": "autothink.unknown",
        "Args": null,
        "TimeoutMs": 30000
      }
    }
    ```
    ```json
    {
      "jsonrpc": "2.0",
      "id": 1,
      "result": {
        "Ok": false,
        "Error": {
          "Kind": "InvalidArgument",
          "Message": "Unknown flow",
          "Details": {
            "flowName": "autothink.unknown",
            "availableFlows": "autothink.attach, autothink.importVariables, autothink.importProgram.textPaste, autothink.build"
          }
        },
        "StepLog": {
          "Steps": [
            {
              "StepId": "ValidateRequest",
              "Action": "Validate RunFlow request",
              "StartedAtUtc": "2025-01-01T00:00:00Z",
              "FinishedAtUtc": "2025-01-01T00:00:00Z",
              "DurationMs": 0,
              "Outcome": "Success"
            },
            {
              "StepId": "DispatchFlow",
              "Action": "Dispatch flow",
              "Parameters": {
                "flowName": "autothink.unknown",
                "availableFlows": "autothink.attach, autothink.importVariables, autothink.importProgram.textPaste, autothink.build"
              },
              "StartedAtUtc": "2025-01-01T00:00:00Z",
              "FinishedAtUtc": "2025-01-01T00:00:00Z",
              "DurationMs": 0,
              "Outcome": "Fail",
              "Error": {
                "Kind": "InvalidArgument",
                "Message": "Unknown flow"
              }
            }
          ]
        }
      }
    }
    ```
  - JSON-RPC 示例（已注册但未实现 → NotImplemented；不依赖 session 存在）：
    ```json
    {
      "jsonrpc": "2.0",
      "id": 2,
      "method": "RunFlow",
      "params": {
        "SessionId": "session-1",
        "FlowName": "autothink.attach",
        "Args": null,
        "TimeoutMs": 30000
      }
    }
    ```
    ```json
    {
      "jsonrpc": "2.0",
      "id": 2,
      "result": {
        "Ok": false,
        "Error": {
          "Kind": "NotImplemented",
          "Message": "Flow is registered but not implemented yet",
          "Details": {
            "flowName": "autothink.attach"
          }
        },
        "StepLog": {
          "Steps": [
            {
              "StepId": "ValidateRequest",
              "Action": "Validate RunFlow request",
              "StartedAtUtc": "2025-01-01T00:00:01Z",
              "FinishedAtUtc": "2025-01-01T00:00:01Z",
              "DurationMs": 0,
              "Outcome": "Success"
            },
            {
              "StepId": "DispatchFlow",
              "Action": "Dispatch flow",
              "Parameters": {
                "flowName": "autothink.attach"
              },
              "StartedAtUtc": "2025-01-01T00:00:01Z",
              "FinishedAtUtc": "2025-01-01T00:00:01Z",
              "DurationMs": 0,
              "Outcome": "Success"
            },
            {
              "StepId": "NotImplemented",
              "Action": "Flow not implemented",
              "Parameters": {
                "flowName": "autothink.attach"
              },
              "StartedAtUtc": "2025-01-01T00:00:01Z",
              "FinishedAtUtc": "2025-01-01T00:00:01Z",
              "DurationMs": 0,
              "Outcome": "Fail",
              "Error": {
                "Kind": "NotImplemented",
                "Message": "Flow is registered but not implemented yet"
              }
            }
          ]
        }
      }
    }
    ```

- **验收自检**：
  - [x] `RpcErrorKinds.NotImplemented` 已存在并可用于流程层错误返回。
  - [x] Flow registry 骨架已建立，可分发到已注册 flow。
  - [x] 未知 flowName → InvalidArgument（示例已给出）。
  - [x] 已注册未实现 flow → NotImplemented（示例已给出）。
  - [x] `dotnet build` / `dotnet test` 在 Release 模式下通过。

- **风险/未决项**：
  - 当前 4 个 flow 均为 StubFlow，真实流程尚未注册；后续 TASK-S2-04~S2-07 需替换为真实实现。

- **下一步建议**（可选）：
  - 进入 TASK-S2-02：实现剪贴板写入能力并在 flow 内提供 StepLog 证据。
