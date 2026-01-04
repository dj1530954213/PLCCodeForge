# TASK-S2-05-result.md

- **Task 编号与标题**：
  - TASK-S2-05：实现 flow：autothink.importProgram.textPaste（剪贴板 + CTRL+V，必要时 fallback Type）

- **完成摘要**：
  - `autothink.importProgram.textPaste` 已实现并注册，默认走“SetClipboardText + CTRL+V”，失败或验证超时可 fallback `Keyboard.Type`。
  - 支持 `verifyMode`（none/editorNotEmpty/elementExists）与 `afterPasteWaitMs`；StepLog 记录完整证据链并避免记录明文。
  - 参数校验已覆盖：`programText` 为空、`editorSelector` 缺失 → InvalidArgument。

- **Args JSON Schema（关键字段 + 默认值）**：
  ```json
  {
    "programText": "string (required, only length logged)",
    "editorSelector": "ElementSelector (required)",
    "afterPasteWaitMs": 1000,
    "verifyMode": "editorNotEmpty | elementExists | none",
    "verifySelector": "ElementSelector (required when verifyMode=elementExists)",
    "findTimeoutMs": 10000,
    "clipboardTimeoutMs": 2000,
    "verifyTimeoutMs": 5000,
    "fallbackToType": true,
    "typeChunkSize": 128,
    "typeChunkDelayMs": 0
  }
  ```

- **改动清单**：
  - `Autothink.UiaAgent/Flows/Autothink/AutothinkImportProgramTextPasteFlow.cs`：新增 verifyMode/afterPasteWaitMs 逻辑，StepId 对齐（SendKeysPaste/VerifyPaste/FallbackTypeText 等）。
  - `Autothink.UiaAgent.Tests/AutothinkImportProgramTextPasteArgsTests.cs`：新增参数校验单测。
  - `Autothink.UiaAgent.Stage2Runner/Program.cs`：示例 args 更新 + JsonElement 安全序列化（便于回归脚本输出）。

- **关键实现说明**：
- StepId（至少包含）：FindEditor / FocusEditor / SetClipboardText / SendKeysPaste / WaitAfterPaste / VerifyPaste（fallback 时追加 FallbackTypeText）。
  - verifyMode=editorNotEmpty：优先读取 ValuePattern/TextPattern，读不到则仅验证控件可用（满足 MVP 要求）。
  - 失败映射：FindError / TimeoutError / ActionError 依照 RunFlow 语义返回。

- **完成证据**：
  - `dotnet build`（Release）输出片段：
    ```text
    已成功生成。
        0 个警告
        0 个错误
    ```
  - `dotnet test`（Release）输出片段：
    ```text
    已通过! - 失败:     0，通过:    29，已跳过:     1，总计:    30
    ```

- **JSON-RPC 示例（成功，含 StepLog）**：
  ```json
  {
    "method": "RunFlow",
    "params": {
      "sessionId": "S-123",
      "flowName": "autothink.importProgram.textPaste",
      "timeoutMs": 30000,
      "args": {
        "programText": "// demo\r\nVAR\r\n  a : INT;\r\nEND_VAR\r\n",
        "editorSelector": {
          "Path": [
            { "Search": "Descendants", "ControlType": "Edit", "AutomationId": "programEditor", "Index": 0 }
          ]
        },
        "afterPasteWaitMs": 1000,
        "verifyMode": "editorNotEmpty",
        "findTimeoutMs": 10000,
        "clipboardTimeoutMs": 2000,
        "verifyTimeoutMs": 5000,
        "fallbackToType": true
      }
    }
  }
  ```
  ```json
  {
    "ok": true,
    "value": {
      "data": { "textLength": 46, "fallbackUsed": false, "verifyMode": "editorNotEmpty" }
    },
    "stepLog": {
      "steps": [
        { "stepId": "ValidateArgs", "outcome": "Success" },
        { "stepId": "GetMainWindow", "outcome": "Success" },
        { "stepId": "BringToForeground", "outcome": "Success" },
        { "stepId": "FindEditor", "outcome": "Success" },
        { "stepId": "FocusEditor", "outcome": "Success" },
        { "stepId": "SetClipboardText", "outcome": "Success" },
        { "stepId": "SendKeysPaste", "outcome": "Success" },
        { "stepId": "WaitAfterPaste", "outcome": "Success" },
        { "stepId": "VerifyPaste", "outcome": "Success" }
      ]
    }
  }
  ```

- **WinFormsHarness 手工验证步骤**：
  1) `dotnet run --project Autothink.UiaAgent.WinFormsHarness/Autothink.UiaAgent.WinFormsHarness.csproj -c Release`
  2) Start Agent → OpenSession（ProcessName=Autothink / TitleContains=AUTOTHINK 视现场调整）
  3) RunFlow 选择 `autothink.importProgram.textPaste`，Args 中填入 `programText` + `editorSelector`
  4) 观察编辑器内容被粘贴，日志中包含 `SetClipboardText` 与 `SendKeysPaste`

- **验收自检**：
  - [x] Flow 已实现并注册；StepLog 包含指定 StepId。
  - [x] 参数校验覆盖 programText/editorSelector 为空场景。
  - [x] 错误映射符合约定（FindError/TimeoutError/ActionError）。
  - [x] Release build/test 通过。
  - [ ] 真实 AUTOTHINK 普通型环境粘贴验证（由你方执行）。

- **风险/未决项**：
  - 某些编辑器不支持 ValuePattern/TextPattern 时仅能做“存在/可用”验证，可能降低“非空”判定精度。
