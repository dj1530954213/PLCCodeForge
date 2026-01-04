# TASK-S2-08-result.md

- **Task 编号与标题**：
  - TASK-S2-08：Stage 2 汇总与回归脚本

- **完成摘要**：
  - Stage 2 4 条 flow 均已实现并注册（attach / importVariables / importProgram.textPaste / build）。
  - 新增 WinFormsHarness + DemoTarget + Stage2Runner 作为小范围回归入口。
  - Selector 统一存放位置已明确，避免散落。

- **回归入口（小范围测试建议）**：
  - WinFormsHarness（交互式单点验证）：
    - `dotnet run --project Autothink.UiaAgent.WinFormsHarness/Autothink.UiaAgent.WinFormsHarness.csproj -c Release`
  - Stage2Runner（整套回归）：
    - `dotnet build PLCCodeForge.sln -c Release`
    - `dotnet run --project Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release`

- **Selector 配置存放约定**：
  - 统一存放在 `Docs/组态软件自动操作/Selectors/` 下（每个 flow 一份 JSON）。
  - 更新方式：现场录制后仅更新对应 JSON（不改代码），并在结果文档中贴出关键 selector 片段。

- **固定 FlowName**：
  - autothink.attach
  - autothink.importProgram.textPaste
  - autothink.importVariables
  - autothink.build

- **Args JSON schema（摘要）**：
  - `autothink.attach`：`args = null`
  - `autothink.importProgram.textPaste`：
    - programText / editorSelector / afterPasteWaitMs / verifyMode / verifySelector
  - `autothink.importVariables`：
    - filePath / openImportDialogSteps[] / filePathEditorSelector / confirmButtonSelector / successCondition?
  - `autothink.build`：
    - buildButtonSelector / waitCondition / timeoutMs? / optionalCloseDialogSelector?

- **WinFormsHarness 最小联调脚本**：
  - 打开 AUTOTHINK → OpenSession → RunFlow attach → importProgram.textPaste → build。

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

- **4 个 flow 的 JSON-RPC 示例请求/响应（可复制）**：

  - `autothink.attach`
    ```json
    {
      "method": "RunFlow",
      "params": {
        "sessionId": "S-123",
        "flowName": "autothink.attach",
        "timeoutMs": 30000,
        "args": null
      }
    }
    ```
    ```json
    {
      "ok": true,
      "value": { "data": { "processId": 1234, "mainWindowTitle": "AUTOTHINK" } },
      "stepLog": { "steps": [
        { "stepId": "GetMainWindow", "outcome": "Success" },
        { "stepId": "BringToForeground", "outcome": "Success" }
      ]}
    }
    ```

  - `autothink.importProgram.textPaste`
    ```json
    {
      "method": "RunFlow",
      "params": {
        "sessionId": "S-123",
        "flowName": "autothink.importProgram.textPaste",
        "timeoutMs": 30000,
        "args": {
          "programText": "// demo\r\nVAR\r\n  a : INT;\r\nEND_VAR\r\n",
          "editorSelector": { "Path": [ { "Search": "Descendants", "ControlType": "Edit", "AutomationId": "programEditor", "Index": 0 } ] },
          "afterPasteWaitMs": 1000,
          "verifyMode": "editorNotEmpty"
        }
      }
    }
    ```
    ```json
    {
      "ok": true,
      "value": { "data": { "textLength": 46, "fallbackUsed": false, "verifyMode": "editorNotEmpty" } },
      "stepLog": { "steps": [
        { "stepId": "FindEditor", "outcome": "Success" },
        { "stepId": "FocusEditor", "outcome": "Success" },
        { "stepId": "SetClipboardText", "outcome": "Success" },
        { "stepId": "SendKeysPaste", "outcome": "Success" },
        { "stepId": "VerifyPaste", "outcome": "Success" }
      ]}
    }
    ```

  - `autothink.importVariables`
    ```json
    {
      "method": "RunFlow",
      "params": {
        "sessionId": "S-123",
        "flowName": "autothink.importVariables",
        "timeoutMs": 30000,
        "args": {
          "filePath": "C:\\temp\\vars.xlsx",
          "openImportDialogSteps": [
            { "action": "Click", "selector": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "Name": "导入", "Index": 0 } ] } }
          ],
          "dialogSelector": { "Path": [ { "Search": "Descendants", "ControlType": "Window", "NameContains": "导入", "IgnoreCase": true, "Index": 0 } ] },
          "filePathEditorSelector": { "Path": [ { "Search": "Descendants", "ControlType": "Edit", "AutomationId": "FilePath", "Index": 0 } ] },
          "confirmButtonSelector": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "Name": "确定", "Index": 0 } ] }
        }
      }
    }
    ```
    ```json
    {
      "ok": true,
      "value": { "data": { "path": "C:\\temp\\vars.xlsx" } },
      "stepLog": { "steps": [
        { "stepId": "OpenImportDialog", "outcome": "Success" },
        { "stepId": "SetFilePath", "outcome": "Success" },
        { "stepId": "ConfirmImport", "outcome": "Success" },
        { "stepId": "WaitImportDone", "outcome": "Success" }
      ]}
    }
    ```

  - `autothink.build`
    ```json
    {
      "method": "RunFlow",
      "params": {
        "sessionId": "S-123",
        "flowName": "autothink.build",
        "timeoutMs": 30000,
        "args": {
          "buildButtonSelector": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "Name": "编译", "Index": 0 } ] },
          "waitCondition": { "kind": "ElementEnabled", "selector": { "Path": [ { "Search": "Descendants", "ControlType": "Button", "Name": "编译", "Index": 0 } ] } }
        }
      }
    }
    ```
    ```json
    {
      "ok": true,
      "value": { "data": { "waitedKind": "ElementEnabled" } },
      "stepLog": { "steps": [
        { "stepId": "FindBuildButton", "outcome": "Success" },
        { "stepId": "ClickBuild", "outcome": "Success" },
        { "stepId": "WaitBuildDone", "outcome": "Success" }
      ]}
    }
    ```

- **验收自检**：
  - [x] 4 条 flow 已实现并注册。
  - [x] Stage2Runner + WinFormsHarness 可用于小范围回归。
  - [x] Selector 存放位置与更新方式明确。
  - [x] Release build/test 通过。

- **风险/未决项**：
  - 真实 AUTOTHINK 环境 selector 需按现场录制更新，否则可能 FindError。
