# Runbook - AUTOTHINK 普通型（UIA Stage2.1）

> 本文用于指导现场联调与证据采集。执行者需在真实 AUTOTHINK 普通型环境下完成。
## 1. 环境前置

- Windows 版本：记录系统版本与补丁号（截图或 `winver`）。
- AUTOTHINK 版本：在“关于/帮助”页面记录版本号与构建号。
- 权限建议：建议以管理员运行 AUTOTHINK 与 Agent（避免 UIA 权限/焦点问题）。

## 1.1 FullHost 启动方式（UiaRpcService）

- FullHost 可执行体：`Autothink.UiaAgent.exe`（Release 输出）。
- 推荐方式：由 Stage2Runner 按 `agentPath` 自动启动（见 `Docs/组态软件自动操作/RunnerConfig/demo.json`）。
- 也可手动启动（用于快速验证）：
  - `dotnet run --project Autothink.UiaAgent/Autothink.UiaAgent.csproj -c Release`

## 2. Selector 录制与校验流程

1. 运行 Agent（Release）：
   - `dotnet build PLCCodeForge.sln -c Release`
   - 启动 `Autothink.UiaAgent.exe`（由 Harness 自动启动亦可）。
2. 运行 WinFormsHarness：
   - `dotnet run --project Autothink.UiaAgent.WinFormsHarness/Autothink.UiaAgent.WinFormsHarness.csproj -c Release`
3. 在 Harness 中执行：
   - 填写进程名/标题 → `OpenSession`。
   - 使用 `FindElement` 验证 selector JSON 是否稳定。
   - 使用 `Click/SetText/SendKeys` 做单点验证。
4. 将 selector 写入 profile 文件：
   - 路径：`Docs/组态软件自动操作/Selectors/*.json`
   - 命名：`<profile>.<flow-suffix>.json`（例：`autothink.importVariables.json`）
   - 结构：`{ "schemaVersion": 1, "selectors": { "key": { ... } } }`
   - 本地覆盖：`<profile>.<flow-suffix>.local.json`（优先级更高，现场私有，不提交仓库）
   - v1 pack 基线：`autothink.v1.base.json`（冻结的必填 key 集合）
   - v1 local 覆盖：`autothink.v1.local.json`（复制 `autothink.v1.local.sample.json` 后改）
   - RunnerConfig 需设置：`selectorPackVersion: "v1"`

## 2.1 SelectorProbe 探针校准（推荐）

1. 使用 Stage2Runner 探针模式（按 flow + key 校验）：
   - `dotnet build PLCCodeForge.sln -c Release`
   - `dotnet run --project Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --config Docs/组态软件自动操作/RunnerConfig/demo.json --probe --probeFlow autothink.build --probeKeys buildButton,buildStatus --probeSearchRoot desktop --probeTimeoutMs 5000`
2. 产物位置：
   - `logs/<timestamp>/probe.<flow>.json`
3. 依据 probe 输出修正：
   - **Ambiguous**：增加 `Index` 或收紧 `NameContains/AutomationIdContains/ClassNameContains`。
   - **0 match**：尝试 `searchRoot=desktop`，或增加 `IgnoreCase/NormalizeWhitespace`。
   - **空白差异**：启用 `NormalizeWhitespace`。
4. 将修正写入 `.local.json`（现场私有，随证据包提交）。

## 2.2 importVariables 关键 selector 校准（v1 pack + local）

- 关键 selector keys（统一放在 `autothink.v1.base.json`）：  
  - `importVariablesMenuOrButton`  
  - `importVariablesDialogRoot`  
  - `importVariablesFilePathEdit`  
  - `importVariablesOkButton`  
  - `importVariablesDoneIndicator`
- DemoTarget 基线：`Docs/组态软件自动操作/Selectors/autothink.v1.base.json`
- 真实 AUTOTHINK：复制 `autothink.v1.local.sample.json` → `autothink.v1.local.json`，再覆盖上述 key（不要提交仓库，随证据包提交）
- 推荐校准命令（示例）：
  - `dotnet run --project Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --config Docs/组态软件自动操作/RunnerConfig/demo.json --probe --probeFlow autothink.importVariables --probeKeys importVariablesMenuOrButton,importVariablesDialogRoot,importVariablesFilePathEdit,importVariablesOkButton --probeSearchRoot desktop --probeTimeoutMs 5000`

## 2.3 importProgram.textPaste 关键 selector 校准（v1 pack + local）

- 关键 selector keys（统一放在 `autothink.v1.base.json`）：  
  - `importProgramMenuOrButton`  
  - `programEditorRoot`  
  - `programEditorTextArea`  
  - `programPastedIndicator`
- 真实 AUTOTHINK 模板：`Docs/组态软件自动操作/Selectors/autothink.v1.local.sample.json`
- 现场使用：复制为 `autothink.v1.local.json` 后再按实际 UI 修正。
- 推荐校准命令（示例）：
  - `dotnet run --project Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --config Docs/组态软件自动操作/RunnerConfig/demo.json --probe --probeFlow autothink.importProgram.textPaste --probeKeys importProgramMenuOrButton,programEditorRoot,programEditorTextArea,programPastedIndicator --probeSearchRoot mainWindow --probeTimeoutMs 5000`

## 3. Stage2Runner 一键回归步骤

1. 先做联通性自检（推荐）：
   - 确认 config 中 `agentPath` 指向 FullHost。
   - `dotnet run --project Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --check --config Docs/组态软件自动操作/RunnerConfig/demo.json --timeoutMs 2000`
   - 产物：`logs/<timestamp>/connectivity.json`
1. 准备配置文件：
   - 示例：`Docs/组态软件自动操作/RunnerConfig/demo.json`（DemoTarget 用例，现场需替换 processName/title）
   - 启用 v1 pack：`selectorPackVersion: "v1"`
   - 指定 `programTextPath`（ST 文本）与 `variablesFilePath`（变量表文件）。
   - 可选：使用 `inputsSource.mode=fromCommIr`（详见 3.5）。
2. 运行命令：
   - `dotnet build PLCCodeForge.sln -c Release`
   - `dotnet run --project Autothink.UiaAgent.Stage2Runner/Autothink.UiaAgent.Stage2Runner.csproj -c Release --config Docs/组态软件自动操作/RunnerConfig/demo.json`
3. 执行顺序：
   - `autothink.attach` → `autothink.importVariables` → `autothink.importProgram.textPaste` → `autothink.build`
4. 运行输出：
   - `logs/<timestamp>/selector_check_report.json`（必填 key 校验）
   - `logs/<timestamp>/autothink.attach.json`
   - `logs/<timestamp>/autothink.importVariables.json`
   - `logs/<timestamp>/autothink.importProgram.textPaste.json`
   - `logs/<timestamp>/autothink.build.json`
   - `logs/<timestamp>/build_outcome.json`（编译签收证据）
   - `logs/<timestamp>/summary.json`（快速定位用）

## 3.1 summary.json 快速定位

- 查看 `summary.json` 中每个 flow 的 `ok/errorKind/failedStepId/selectorKey/root`。
- 先看 `selector_check_report.json` 的 `missingKeys`，非空则先修 selector pack。
- 若 `errorKind=FindError`，优先回到 selector 录制环节校验 `selectorKey` 对应元素。
- 若 `stoppedBecause=flowFailed:<flow>`，说明已触发 fail-fast，后续 flow 会标记 `NotRun`。

## 3.2 流水线策略（fail-fast / skip / allowPartial）

- 默认策略：fail-fast（某个 flow 失败后停止后续执行）。
- 如需跳过某个 flow，可在 config 中设置：
  - `skipImportVariables: true`
  - `skipImportProgram: true`
  - `skipBuild: true`
- 如需“失败后继续跑后续 flow”，可设置：
  - `allowPartial: true`（仅在明确需要时开启）

## 3.3 popupHandling（弹窗收敛）

- 默认关闭；仅在弹窗干扰流程时启用。
- 在 RunnerConfig 中设置：
  - `enablePopupHandling: true`
  - `popupSearchRoot: desktop`
  - `popupTimeoutMs: 1500`
  - `allowPopupOk: false`（仅当确认可点 OK 时才设 true）
- selector 来源：`Docs/组态软件自动操作/Selectors/autothink.popups.json`（可用 `.local.json` 覆盖）。

## 3.4 buildOutcome（编译签收模板）

> 通过 buildOutcome 判定编译成功/失败，产物落盘 `logs/<timestamp>/build_outcome.json`。

- 模板 A：waitSelector（只看成功指示控件）
  ```json
  "buildOutcome": {
    "mode": "waitSelector",
    "successSelectorKey": "buildSucceededIndicator",
    "timeoutMs": 60000
  }
  ```
- 模板 B：readTextContains（读取输出/状态文本）
  ```json
  "buildOutcome": {
    "mode": "readTextContains",
    "textProbeSelectorKey": "buildOutputPane",
    "successTextContains": ["编译成功", "Build OK", "0 errors"],
    "timeoutMs": 60000
  }
  ```
- 模板 C：either（selector + text 双保险）
  ```json
  "buildOutcome": {
    "mode": "either",
    "successSelectorKey": "buildSucceededIndicator",
    "textProbeSelectorKey": "buildOutputPane",
    "successTextContains": ["编译成功", "Build OK"],
    "timeoutMs": 60000
  }
  ```

## 3.5 inputsSource（fromCommIr）

- 适用场景：从通讯采集模块输出的 CommIR v1 直接绑定变量表与程序文本路径。
- 配置方式（示例）：
  - `inputsSource.mode: "fromCommIr"`
  - `inputsSource.commIrPath: "..\\..\\Samples\\comm_ir.sample.json"`
- 解析产物：
  - `logs/<timestamp>/resolved_inputs.json`
- 解析优先级：
  1) CommIR 中 `inputs.variablesFilePath` / `inputs.programTextPath`
  2) `outputs.variablesFilePath` / `outputs.programTextPath`（若有）
  3) `sources.unionXlsxPath`（仅变量表 fallback）
  4) 若以上缺失则回退到 config 的 `variablesFilePath/programTextPath`

## 3.6 剪贴板排查清单（CTRL+V 主路径）

- 确认进程权限：建议 AUTOTHINK 与 Agent 同权限（最好管理员）。
- 剪贴板占用：关闭可能占用剪贴板的程序（远程剪贴板工具、同步工具、截图工具）。
- RDP 场景：若远程桌面剪贴板被禁用/受限，可能导致 `ClipboardBusy`/`AccessDenied`。
- 若健康检查失败：可设置 `forceFallbackOnClipboardFailure=true` 强制走 Keyboard.Type。
- 若仍失败：调大 `clipboardRetry.times` 或 `clipboardRetry.intervalMs` 并观察 StepLog 的 failureKind。

## 3.7 EvidencePack v1（证据包打包/验证）

- 默认启用：`evidencePack.enable=true`（RunnerConfig 可显式关闭）。
- 生成位置：`logs/<timestamp>/evidence_pack_v1/`
- 包含文件（示例）：`summary.json`、`selector_check_report.json`、`build_outcome.json`（如有）、`step_logs.json`、`resolved_inputs.json`（如有）、`unexpected_ui_state.json`（如有）
- 证据摘要：`evidence_summary.v1.json`（含 digests 与关键指标）
- 验证命令：
  - `Autothink.UiaAgent.Stage2Runner.exe --verify --evidence logs/<timestamp>/evidence_pack_v1`

## 3.8 UIStateRecovery（UnexpectedUIState 收敛）

- 目的：关键动作失败时尝试自动处理异常弹窗（最多 N 次），并落盘证据。
- 配置示例：
  ```json
  "uiStateRecovery": {
    "enable": true,
    "maxAttempts": 2,
    "searchRoot": "desktop"
  }
  ```
- selector 约定（v1 pack）：`global.popupRoot` / `global.popupOkButton` / `global.popupNoButton` / `global.popupWarningText`
- 产物：
  - `logs/<timestamp>/unexpected_ui_state.json`
  - `summary.json` 的 `uiStateRecovery` 字段
  - `step_logs.json` 中 `stepId=UnexpectedUIState` 的 StepLogEntry

## 4. 失败定位指南

- **FindError**：
  - 用 `FindElement` 单独验证 selector。
  - 必要时加入 `NameContains/IgnoreCase/AutomationIdContains/ClassNameContains/NormalizeWhitespace`。
- **TimeoutError**：
  - 调大 `waitTimeoutMs/verifyTimeoutMs`。
  - 确认 `waitCondition` 可观测（按钮 Enabled、对话框消失等）。
- **ActionError**：
  - 检查焦点是否正确、输入法是否干扰、剪贴板是否被占用。
- **UnexpectedUIState**：
  - 检查弹窗是否出现；补充 `unexpectedSelectors` 或关闭步骤。
  - 必要时启用 `popupHandling`（默认 Cancel/关闭）。

## 5. 证据提交要求

- 将 `logs/<timestamp>/` 目录整包提交。
- 同时提交：
  - `Docs/组态软件自动操作/Selectors/` 下的 baseline 与 `.local.json`（如有）。
  - 运行时的配置文件（例如 `Docs/组态软件自动操作/RunnerConfig/demo.json`）。
  - `probe.<flow>.json`（如使用 SelectorProbe 校准）。
- 若需补充：附上 AUTOTHINK 版本截图与关键 UI 状态截图。
