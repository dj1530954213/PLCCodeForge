# 分层架构与模块边界

## 分层概览（L0-L5）
- L0 系统与驱动层：UIA 会话管理、窗口/元素定位、输入事件基础设施。
- L1 原子动作层：Click/KeyNav/SetText/Wait/Assert 等动作原语。
- L2 UI Profile 层：锚点、坐标、选择器、导航序列资产化。
- L3 Flow DSL 层：流程结构、控制流、参数绑定与模板机制。
- L4 编排与状态层：任务图/DAG、状态机与策略执行。
- L5 治理与工具层：配置校验、版本发布、证据链、回滚。

## 各层输入/输出
- L0 输入：窗口标识、Selector、坐标；输出：元素引用、动作执行结果。
- L1 输入：Action + args；输出：StepLog + ActionResult。
- L2 输入：Profile 版本信息；输出：解析后的 anchors/positions/selectors。
- L3 输入：Flow DSL + inputs；输出：展开后的 Steps 与执行上下文。
- L4 输入：TaskGraph + 用户起点选择；输出：执行计划与状态流转。
- L5 输入：版本与证据；输出：发布包、回滚指令、审计报告。

## 架构示意（简化）
```
[L5 工具与治理]
   ^
[L4 编排/状态机]  <--- 用户选择顺序/起点
   ^
[L3 Flow DSL]     <--- 模板 + 参数
   ^
[L2 UI Profile]   <--- 坐标/锚点/选择器
   ^
[L1 原子动作]     <--- Click/KeyNav/Wait
   ^
[L0 UIA Session]  <--- STA + UIA
```

## 核心组件
- UIA Agent：执行 UIA 动作，管理 Session（L0-L1）。
- Profile Manager：解析 Profile，输出锚点/坐标/选择器（L2）。
- Flow Runtime：解析 DSL、绑定参数、执行 Step（L3）。
- Orchestrator：任务图调度与用户顺序选择（L4）。
- Tooling：校准、验证、模板管理（L5）。

## 关键数据结构（建议）
- FlowDefinition：flow/inputs/tasks/templates。
- TaskInstance：id/inputs/steps/policy/state。
- StepContext：当前 Step 参数、变量绑定、Profile 解析结果。
- ProfileSnapshot：解析后的 anchors/positions/selectors 缓存。
- ActionResult：ok/warn/error + errorKind。
- StepLog：stepId/action/params/outcome/duration/error。

## 运行时数据流
1) Runner 启动并建立 UIA Session。
2) Profile Manager 选择匹配的 Profile（版本/语言/分辨率）。
3) Flow Runtime 解析 DSL 与模板，构建执行上下文。
4) Orchestrator 生成任务图并根据用户选择生成计划。
5) Flow Runtime 执行 Step/Action，输出 StepLog。
6) Evidence Pack 汇总日志与关键快照，供验收与回放。

## 关键接口（建议）
- RunFlowDsl(flowId, inputs, profileRef, options)
- ValidateFlow(flowId, inputs, profileRef)
- ListProfiles(app, lang, resolution)
- DryRun(flowId, inputs, profileRef)

## 资产类型
- Profile：UI 版本档案（锚点/坐标/selector）。
- Template：流程模板（按模块类型组织）。
- Flow：流程实例（任务图 + 参数）。
- TaskGraph：任务关系图与执行计划。
- Evidence：StepLog 与输出摘要。

## 模块边界与责任
- UIA Agent：只做“动作执行”，不理解业务规则。
- Flow Runtime：只做“动作编排”，不处理坐标细节。
- Profile：只做“坐标/selector 映射”，不包含流程逻辑。
- Orchestrator：只做“依赖与顺序”，不包含 UI 操作细节。

## 跨层约束
- DSL 不允许直接携带坐标常量（必须引用 Profile）。
- Profile 不允许包含流程控制逻辑（只描述 UI 位置与元素）。
- Orchestrator 不允许直接调用 UIA（必须通过 DSL Runtime）。

## 关键扩展点
- Action 扩展：新增动作必须符合参数校验与错误规范。
- Profile 扩展：新增锚点/坐标不影响流程。
- Template 扩展：新增模块类型只需模板与参数映射。
- Orchestrator 扩展：新增任务依赖规则不影响动作层。

## 性能与稳定性假设
- UIA 单线程执行，所有动作严格串行。
- 允许“计划级并行”仅体现在前置检查或校验。
- 允许对同一 Profile 做内存级缓存以提升性能。

## 约束补充
- MFC 自绘区域不可直接 UIA 识别：坐标来自 Profile。
- 地址计算规则外置：流程只接收计算结果参数。
- 并发执行不考虑（UIA 单线程）。

## 与现有系统的对齐
- UiaRpcService 保持原有原子动作入口不变。
- 新增 RunFlowDsl 入口用于 DSL 驱动执行。
- 现有 Flow 实现逐步迁移为模板。
