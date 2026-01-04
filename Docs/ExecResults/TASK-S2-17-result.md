# TASK-S2-17-result.md

- **Task 编号与标题**：
  - TASK-S2-17：Runner↔Agent 联通性自检 + 方法探测

- **完成摘要**：
  - Stage2Runner 增加 `--check` 自检模式，跑 READY/Ping/方法探测并落盘 `connectivity.json`。
  - 每次 flow/probe 前默认执行自检，失败则停止并在 summary.json 标注 `connectivityOk/failedReason`。
  - 自检超时可配置（`--timeoutMs`，默认 2000ms）。

- **改动清单**：
  - `Autothink.UiaAgent.Stage2Runner/Program.cs`：新增自检流程、方法探测、connectivity.json 落盘、summary 字段扩展。
  - `Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 `--check` 使用说明。

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

- **--check 失败示例（本机 Agent 仅 Ping）**：
  - stdout 片段：
    ```text
    RunDir: C:\Program Files\Git\code\PLCCodeForge\logs\20260103-174245
    Connectivity: C:\Program Files\Git\code\PLCCodeForge\logs\20260103-174245\connectivity.json
    Connectivity check: FAIL - Missing RPC methods: OpenSession, CloseSession, FindElement, Click, SetText, SendKeys, WaitUntil
    Hint: You may be running the minimal AgentHost (Ping only). Use the full UiaRpcService host.
    ```
  - `logs/20260103-174245/connectivity.json` 片段（含 durationMs）：
    ```json
    {
      "ok": false,
      "handshakeReady": true,
      "pingOk": true,
      "methods": { "OpenSession": false, "CloseSession": false },
      "error": { "kind": "ConfigError", "message": "Missing RPC methods: ..." },
      "durationMs": 231
    }
    ```

- **--check 成功示例（需现场完整 Agent）**：
  - 说明：当前仓库内 `Autothink.UiaAgent.exe` 仅暴露 Ping，因此无法在本机生成成功样例；需在现场使用完整 UiaRpcService Host 执行。
  - 预期 stdout 结构：
    ```text
    Connectivity check: OK
    ```
  - 预期 connectivity.json 结构：
    ```json
    {
      "ok": true,
      "handshakeReady": true,
      "pingOk": true,
      "methods": { "OpenSession": true, "CloseSession": true, "FindElement": true }
    }
    ```

- **复现与定位 OpenSession not found**：
  - 复现：运行 `dotnet run ... --config Docs/组态软件自动操作/RunnerConfig/demo.json`。
  - 定位：summary.json 中 `connectivityOk=false`，`failedReason` 明确提示缺失方法；connectivity.json 可直接看到缺失的方法清单。

- **summary.json 片段（connectivityOk/failedReason）**：
  - `logs/20260103-174318/summary.json`
    ```json
    {
      "connectivityOk": false,
      "connectivityFailedReason": "Missing RPC methods: OpenSession, CloseSession, FindElement, Click, SetText, SendKeys, WaitUntil"
    }
    ```

- **验收自检**：
  - [x] 不修改 RPC 契约，仅 Runner 侧探测方法存在性。
  - [x] connectivity.json 落盘并含 stdoutHead/handshake/ping/methods/error/durationMs。
  - [x] summary.json 包含 connectivityOk/failedReason。
  - [x] 默认自检时长 <= 2s（durationMs 证据）。

- **风险/未决项**：
  - 现场需使用完整 UiaRpcService Host 才能通过自检；当前最小 AgentHost 仅 Ping 会导致失败。
