# C# UIA 资产盘点（P0）

## 范围
- 仅覆盖仓库内 C# UIA 相关模块与资产。
- 不评估 Rust/前端部分。

## 代码资产清单
### 核心工程
- `Autothink.UIA/Autothink.UiaAgent/`：UIA Agent（RPC 服务端，FlaUI UIA3）。
- `Autothink.UIA/Autothink.UiaAgent.Stage2Runner/`：Runner（执行器，读取配置与 selector 资产，调用 RPC）。
- `Autothink.UIA/Autothink.UiaAgent.Tests/`：测试（ElementFinder/Waiter/SendKeys 等）。
- `Autothink.UIA/Autothink.UiaAgent.DemoTarget/`：Demo UI（测试目标）。
- `Autothink.UIA/Autothink.UiaAgent.WinFormsHarness/`：WinForms 客户端示例。

### RPC 契约
- Session：
  - `OpenSessionRequest/Response`、`CloseSessionRequest`。
  - 入口：`UiaRpcService.OpenSession` / `CloseSession`。
- 元素定位与动作：
  - `FindElementRequest/Response`（selector -> ElementRef）。
  - `Click/DoubleClick/RightClick`（ElementRef）。
  - `ClickAt/RightClickAt/ClickRel`（锚点+坐标/比例）。
  - `SetText`（Replace/Append/CtrlAReplace）。
  - `SendKeys`（CTRL+V/ENTER 等）。
  - `WaitUntil`（ElementExists/ElementNotExists/ElementEnabled）。
  - 契约：`Autothink.UIA/Autothink.UiaAgent/Rpc/Contracts/Actions.cs`。
- StepLog 与错误分类：
  - `StepLog`/`StepLogEntry`：证据链基础结构。
  - `RpcErrorKinds`：ConfigError/FindError/TimeoutError/ActionError/UnexpectedUIState/StaleElement/InvalidArgument/NotImplemented。

## 现有 Flow 清单（Stage2）
- `autothink.attach`：附加主窗口并置前。
  - 文件：`Autothink.UIA/Autothink.UiaAgent/Flows/Autothink/AutothinkAttachFlow.cs`
- `autothink.importVariables`：变量导入（菜单/按钮 → 对话框 → 填路径 → 确认 → 等待完成）。
  - 支持 `openImportDialogSteps`（Click/DoubleClick/RightClick/ClickAt/RightClickAt/ClickRel/Hover/SetText/SendKeys/KeyNav/WaitUntil）。
  - 文件：`Autothink.UIA/Autothink.UiaAgent/Flows/Autothink/AutothinkImportVariablesFlow.cs`
- `autothink.importProgram.textPaste`：程序文本粘贴（Ctrl+V + fallback 键入）。
  - 支持 `openProgramSteps`（Click/DoubleClick/RightClick/ClickAt/RightClickAt/ClickRel/SetText/SendKeys/KeyNav/WaitUntil）。
  - 具备剪贴板健康检查与降级策略。
  - 文件：`Autothink.UIA/Autothink.UiaAgent/Flows/Autothink/AutothinkImportProgramTextPasteFlow.cs`
- `autothink.build`：编译构建并等待结果（selector/读取文本两种判定）。
  - 支持 `BuildOutcome` 多模式。
  - 文件：`Autothink.UIA/Autothink.UiaAgent/Flows/Autothink/AutothinkBuildFlow.cs`

## 选择器资产（Selector Profile）
### 文件结构与命名
- Base profile：`Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.v1.base.json`
- Flow profile：
  - `autothink.attach.json`
  - `autothink.importVariables.json`
  - `autothink.importProgram.textPaste.json`
  - `autothink.build.json`
  - `autothink.popups.json`
- Local override：同名 + `.local.json`
- Demo profile：`Autothink.UIA/Docs/组态软件自动操作/Selectors/autothink.demo.json`

### 加载与合并规则（Stage2Runner）
- Base baseline → Base local → Flow baseline → Flow local。
- 入口：`Stage2Runner.Program.LoadProfileWithOverrides`。
- 选择器结构：`ElementSelector`，以 `Path` 逐层匹配。

### Profile 扩展（anchors/positions）
- Selector profile 文件可同时包含 `anchors/positions/navSequences`。
- Runner 在构建 dialog steps 时可用 `positionKey` 解析为 anchorSelector + offset。
- 入口：`Stage2Runner.Program.ResolvePosition`。

## Runner 配置资产
- Demo 配置：`Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json`
- 输入来源支持：
  - inline（直接配置路径）
  - fromCommIr（读取 `comm_ir.v1`）
- 证据包输出：
  - `summary.json`
  - `selector_check_report.json`
  - `step_logs.json`
  - `evidence_summary.v1.json`
  - 可选：`build_outcome.json`、`unexpected_ui_state.json`、`resolved_inputs.json`

## 证据链与诊断资产
- StepLog 由 RPC 与 Flow 层统一写入。
- Stage2Runner 输出 `evidence_pack_v1` 并生成 SHA256 摘要。
- 支持 selector check 报告（缺失 key 统计）。
- 支持 UI 状态恢复报告（unexpected_ui_state.json）。

## 现有能力与缺口对照（与新计划对齐）
### 已具备能力
- selector 驱动的 UIA 操作（ElementFinder）。
- Flow 内步骤数据化（openSteps/conditions）。
- StepLog 证据链。
- popup 处理与 UI 状态恢复（Runner 级）。
- 证据包与校验流程。

### 明确缺口
- 缺少 Profile 资产与校准工具：锚点/坐标仍需外部提供与维护。
- 缺少 Profile 运行时解析器（anchors/positions/navSequences）。
- 缺少 Flow DSL + TaskGraph 编排（当前 Runner 固定顺序）。
- 缺少“模块类型 → 模板”机制（目前只有固定 Flow）。

## 结论（P0）
- 现有 C# UIA 具备稳定的 selector 动作与证据链基础。
- 下一步需要补齐坐标/锚点能力与 DSL 编排层，才能覆盖“硬件组态/模块参数设定”类流程。
