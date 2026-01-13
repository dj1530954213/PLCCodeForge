# TASK-S2-09-result.md

- **Task 编号与标题**：
  - TASK-S2-09：Selector Profile 加载机制（Stage2Runner）

- **完成摘要**：
  - Stage2Runner 支持 `--selectorsRoot` / `--profile` 并按 `<profile>.<flow-suffix>.json` 加载 selector profile。
  - Selector 资产落地到 `Autothink.UIA/Docs/组态软件自动操作/Selectors/`，每个 flow 一份 JSON。
  - Runner 展开 selector，不改 Agent RPC 契约；兼容直接传 selector。

- **改动清单**：
  - `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/Program.cs`：新增 profile 加载、selector key 展开、CLI 参数。
  - `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.attach.json`：profile 模板。
  - `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.importProgram.textPaste.json`：profile 模板（含 editor/buildButton 示例）。
  - `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.importVariables.json`：profile 模板。
  - `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.build.json`：profile 模板。
  - `Autothink.UIA/Docs/组态软件自动操作/Selectors/README.md`：命名与 schema 说明。

- **build/test 证据**：
  - `dotnet build Autothink.UIA/PLCCodeForge.sln -c Release`：
    ```text
    已成功生成。
        0 个警告
        0 个错误
    ```
  - `dotnet test Autothink.UIA/Autothink.UiaAgent.Tests/Autothink.UiaAgent.Tests.csproj -c Release`：
    ```text
    已通过! - 失败:     0，通过:    34，已跳过:     1，总计:    35
    ```

- **Selector Profile 示例（完整文件）**：
  - `Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.importProgram.textPaste.json`
    ```json
    {
      "schemaVersion": 1,
      "selectors": {
        "editor": {
          "Path": [
            { "Search": "Descendants", "ControlType": "Edit", "AutomationId": "TODO_EDITOR_AUTOMATION_ID", "Index": 0 }
          ]
        },
        "verify": {
          "Path": [
            { "Search": "Descendants", "ControlType": "Pane", "NameContains": "TODO_VERIFY_TARGET", "IgnoreCase": true, "Index": 0 }
          ]
        },
        "buildButton": {
          "Path": [
            { "Search": "Descendants", "ControlType": "Button", "NameContains": "TODO_BUILD", "IgnoreCase": true, "Index": 0 }
          ]
        }
      }
    }
    ```

- **Stage2Runner 运行日志片段（示例）**：
  ```text
  SelectorsRoot: C:\...\Autothink.UIA\Docs\组态软件自动操作\Selectors
  Profile: autothink
  Loading profile: ...\autothink.importVariables.json
  === RunFlow.autothink.importVariables ===
  Request:
  { ... }
  Response:
  { "ok": true, "stepLog": { "steps": [ ... ] } }
  ```

- **验收自检**：
  - [x] Selector profile 文件已落地到固定目录。
  - [x] Stage2Runner 支持 `--selectorsRoot` / `--profile`。
  - [x] build/test Release 通过。

- **风险/未决项**：
  - Profile 内 selector 仍为占位模板，需现场录制替换。
