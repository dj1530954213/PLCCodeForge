# TASK-S2-18-result.md

- **Task 编号与标题**：
  - TASK-S2-18：完整 UiaRpcService Host + --check 成功样例

- **完成摘要**：
  - Autothink.UiaAgent 入口改为 UiaRpcService 作为 JSON-RPC target（READY + HeaderDelimitedMessageHandler + STA 单线程保持不变）。
  - Stage2Runner 支持从 RunnerConfig 读取 agentPath，demo.json 指向 FullHost。
  - 本机 --check 成功，methods 全部通过，connectivity.json 已落盘。

- **改动清单**：
  - `Autothink.UIA/Autothink.UiaAgent/AgentHost.cs`：RPC target 切换为 `UiaRpcService`。
  - `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`：RunnerConfig 增加 `agentPath` 支持。
  - `Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`：补充 `agentPath` 指向 FullHost。
  - `Autothink.UIA/Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 FullHost 启动说明与 --check 步骤。

- **build 证据**：
  - `dotnet build Autothink.UIA/PLCCodeForge.sln -c Release`：
    ```text
    已成功生成。
        0 个警告
        0 个错误
    ```

- **--check 成功示例**：
  - stdout 片段：
    ```text
    RunDir: C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\logs\20260103-181038
    Connectivity: C:\Program Files\Git\code\PLCCodeForge\Autothink.UIA\logs\20260103-181038\connectivity.json
    Connectivity check: OK
    ```
  - `Autothink.UIA/logs/20260103-181038/connectivity.json` 片段：
    ```json
    {
      "ok": true,
      "handshakeReady": true,
      "pingOk": true,
      "methods": {
        "OpenSession": true,
        "CloseSession": true,
        "FindElement": true,
        "Click": true,
        "SetText": true,
        "SendKeys": true,
        "WaitUntil": true
      },
      "stdoutHead": ["READY"],
      "durationMs": 209
    }
    ```

- **Runbook 更新片段**：
  ```text
  ## 1.1 FullHost 启动方式（UiaRpcService）
  - FullHost 可执行体：Autothink.UiaAgent.exe（Release 输出）。
  - 推荐方式：由 Stage2Runner 按 agentPath 自动启动。
  - 也可手动启动：dotnet run --project Autothink.UIA/Autothink.UiaAgent/Autothink.UiaAgent.csproj -c Release
  ```

- **验收自检**：
  - [x] RPC target 为 UiaRpcService，未修改 RPC 契约。
  - [x] READY 握手与 HeaderDelimitedMessageHandler 保持不变。
  - [x] STA + SingleThreadSynchronizationContext 仍生效（沿用 AgentHost 结构）。
  - [x] Stage2Runner --check 成功，methods 探测全通过。

- **风险/未决项**：
  - 仅验证了联通性与方法存在性，真实 AUTOTHINK UI 行为仍需后续 S2-19 真机闭环验证。
