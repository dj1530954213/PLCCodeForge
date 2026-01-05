# UIA 自动操作架构说明

本文档仅描述 C# 侧（UiaAgent / Stage2Runner / WinFormsHarness / DemoTarget）的业务与架构设计，便于现场联调与后续维护。

## 1. 模块分层与职责

- UiaAgent（sidecar）
  - 对外提供 JSON-RPC 方法：OpenSession/Find/Click/SetText/SendKeys/WaitUntil/RunFlow。
  - 在 STA 单线程中执行 UIA/FlaUI，避免跨线程问题。
  - 负责 StepLog 证据链的生成与错误语义归一。

- Flow 层（UiaAgent/Flows）
  - 把原子操作组合成可交付流程（attach/importVariables/importProgram.textPaste/build）。
  - 只依赖 selector 与输入参数，不在代码里硬编码控件文本。
  - 对失败进行结构化映射：FindError/ActionError/TimeoutError/InvalidArgument/NotImplemented。

- Uia 层（UiaAgent/Uia）
  - 提供选择器解析、元素查找、等待、键盘输入与剪贴板能力。
  - ElementFinder 负责 selector 路径匹配与错误细分。

- Stage2Runner（执行器）
  - 读取配置与 selector profile，启动/连接 Agent 并按顺序执行 flows。
  - 提供 check/probe/verify 以及 summary/evidence pack 输出。
  - 不修改 RPC 契约，仅通过 JSON-RPC 组装参数与调度。

- WinFormsHarness（手工测试台）
  - 用于现场快速验证 selector/原子动作/RunFlow。
  - 支持加载 selector JSON 与日志查看。

- DemoTarget（演示 UI）
  - 为开发/CI 提供可控的 UIA 目标窗口，避免依赖真实 AUTOTHINK。
  - 控件 Name/AccessibleName 用于稳定的 selector 资产。

## 2. 进程模型

- Autothink.UiaAgent.exe
  - 通过 stdout 输出 READY 作为握手信号。
  - JSON-RPC 通过 stdin/stdout 传输，业务日志写 stderr。
  - STA 线程承载所有 UIA 调用。

- Stage2Runner / WinFormsHarness
  - 作为客户端连接 UiaAgent，负责参数/selector 的组织与验证。

## 3. 核心数据约定

- ElementSelector
  - 采用路径 Path 的形式逐层匹配。
  - 支持 Name/AutomationId/ClassName 的 exact/contains + IgnoreCase/NormalizeWhitespace。
  - 每一步至少一个过滤条件。

- StepLog
  - 每个关键动作/等待必须写入 StepLogEntry。
  - 失败必须包含 RpcError.Kind，并在 Runner 侧汇总为 summary。

- RunFlow
  - 统一入口：FlowName + Args(JsonElement/ArgsJson)。
  - FlowName 固定 4 个（autothink.attach / importVariables / importProgram.textPaste / build）。

## 4. Selector 资产与现场联调

- selector JSON 统一放在 Docs/组态软件自动操作/Selectors/。
- 支持 local override（*.local.json），现场录制值不入库但需随证据包提交。
- Stage2Runner 支持 profileName + key 的展开方式，避免上层拼 selector。

## 5. 证据链与回归

- 所有 RPC 与 flow 都必须返回 StepLog。
- Runner 会输出 summary.json、selector_check_report.json、build_outcome.json 等证据文件。
- evidence_pack_v1 可用于跨环境复现与验收。

## 6. 与真实 AUTOTHINK 的差异

- DemoTarget 仅用于验证“流程骨架 + 选择器机制”。
- 真实 AUTOTHINK 需要重新录制 selector，并通过 probe 校准。
- 对话框/弹窗需通过 popup handling 机制收敛。

## 7. 维护原则

- 不改 RPC 契约字段语义；仅允许新增可选字段。
- selector 不在代码里硬编码；只通过 selectorKey/配置驱动。
- 失败必须结构化，便于现场定位与回归对比。
